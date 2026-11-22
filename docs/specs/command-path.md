---
status: draft
type: spec
depends-on: [read-write-separation, security, edge-agent, transport, config-lifecycle]
---

# Command Path (WRITE)

Every change IRON makes to the physical world goes through exactly one pipeline.
There is no second way to write.

```
Operator (UI / API)
  → Authentication
  → RBAC check (role + object scope)
  → Confirmation (when required)
  → Command Service
  → Audit journal (local, append-only, fsync)        ← gate
  → Transport: command to the owning edge (at-most-once, never stored)
  → Edge agent: check id · TTL · config hash → validate against
    config/commands/ → protocol write → read-back
  → Result event (ACK / NACK) → audit record completed → UI feedback
```

Requests enter through one authenticated endpoint of the Command Service
API. The real-time WebSocket cannot carry a command
([read-write-separation.md](read-write-separation.md)).

## Command definition

```yaml
# config/commands/reactor_01.yaml — physically separate from tags/
reactor_01:
  pump_start:
    target: modbus://plc-01/coil/0x0020
    type: bool
    requires_role: operator
    requires_confirm: true     # UI confirmation dialog
    cooldown: 10s              # cannot be re-sent within 10s
    feedback_tag: pump_01.running   # expected state confirmation
    feedback_timeout: 5s
  setpoint_temperature:
    target: modbus://plc-01/holding/0x2000
    type: float32
    requires_role: operator
    limits: [20, 150]          # server rejects values outside; PLC enforces its own limits too
    max_step: 10               # reject changes larger than 10°C at once
```

| Field | Meaning |
|---|---|
| `target` | Writable address (same URI schemes as tags) |
| `requires_role` | Minimum role; object-scoped RBAC applies on top |
| `requires_confirm` | Explicit confirmation dialog before send |
| `cooldown` | Minimum interval between sends of this command |
| `limits` / `max_step` | Server-side sanity bounds for analog writes |
| `feedback_tag` / `feedback_timeout` | Tag that must reflect the new state; timeout produces a FAILED result |

## The command envelope

What the Command Service sends to the edge, for every command:

| Field | Meaning |
|---|---|
| `id` | Unique identifier assigned by the Command Service; the key of the audit record |
| `command` | Canonical path of the command definition (`reactor_01.pump_start`) |
| `value` | The value to write, already validated against `type`, `limits`, `max_step` |
| `issued_at` | Server time at which the journal entry was written |
| `ttl` | How long the command stays valid (default 10s) |
| `config_hash` | The effective-config hash the server validated against ([config-lifecycle.md](config-lifecycle.md)) |
| `issuer` | Operator identity and role, for the edge's own log |

## Command lifecycle

Every command is a tracked state machine, not a fire-and-forget write:

```
ISSUED → DELIVERED (edge accepted) → EXECUTED (protocol write OK)
       → CONFIRMED (feedback_tag reached expected state)
       | REJECTED (edge refused: unknown command, config mismatch, duplicate id)
       | FAILED (protocol exception / read-back mismatch / feedback timeout)
       | EXPIRED (not accepted within TTL — edge offline or too late)
```

Every state transition is appended to the audit record, and the UI shows the
lifecycle explicitly — an operator always knows whether the command was
confirmed, failed, rejected, or expired.

### Delivery: at most once, never late

Stale and repeated commands are the most dangerous kind. Four rules remove
them:

- **No storage, no retry.** The transport MUST deliver a command at most
  once and MUST NOT store it ([transport.md](transport.md)). If the owning edge has
  no live session, the command goes straight to `EXPIRED`. Nothing waits in
  a queue for the link to return. The Command Service never re-sends; an
  operator who wants to try again issues a new command with a new `id`. `REQ-CMD-004 integration`
- **The edge enforces the TTL itself.** It executes a command only if
  `issued_at + ttl` has not passed, measured in server time using the
  session's clock-offset estimate. An edge with no current estimate MUST
  reject. The TTL does not depend on NTP being configured correctly on a box
  in a cabinet. `REQ-CMD-001 integration`
- **Duplicate ids MUST be rejected.** The edge remembers the id of every command
  it accepted for at least twice the maximum TTL and rejects a repeat. After
  a restart it rejects any command issued before it started. `REQ-CMD-005 integration`
- **Config must match.** The edge MUST reject a command whose `config_hash`
  differs from its own: the two sides disagree about what the command means. `REQ-CMD-006 integration`

A command the edge refuses produces a result naming the reason; nothing is
dropped silently. The final state follows the reason: too late, or no clock
estimate to judge lateness by, is `EXPIRED`; a duplicate id, a config
mismatch, or an unknown command is `REJECTED`.

### Execution

The edge writes, then reads the target back where the protocol allows and
reports the observed value. A read-back that differs from the written value
MUST be reported as `FAILED`, not `EXECUTED`. When a `feedback_tag` is declared, `CONFIRMED`
additionally requires that tag to reach the expected state within
`feedback_timeout`. `REQ-CMD-007 integration`

## Audit policy — availability over centralized logging

Every command MUST be journaled before it is sent. But "journaled" means a
**local append-only journal on the iron-server host** (fsync'd file or local
table), not a synchronous write to the central historian. `REQ-CMD-002 integration`

- Journal append fails (local disk failure) → command is refused. This is the
  only condition that blocks control, and it means the host itself is dying.
- Historian (TimescaleDB) unavailable → commands still work; the journal
  syncs to the historian when it returns. **An operator must be able to stop a
  pump while the database server is being rebooted.** Loss of central audit
  sync is an alarm-worthy event, not a reason to freeze the plant.

Audit records are immutable and complete:

```
2026-03-24T14:23:11Z  arman@plant.kz  COMMAND  reactor_01.pump_start → true
  role: operator  ip: 192.168.20.15  confirmed: true  result: CONFIRMED
  previous_value: false  feedback: pump_01.running=true after 1.2s

2026-03-24T14:23:45Z  arman@plant.kz  COMMAND  reactor_01.pump_start → true
  result: FAILED  reason: "PLC exception 02 (illegal data address)"
```

## Authentication strength vs. urgency

Confirmation requirements must scale with *consequence*, but never against
*urgency*:

- `requires_confirm` — appropriate for any state-changing command.
- TOTP / second factor — appropriate for rare administrative operations
  (changing command definitions, role grants), and MAY be configured for
  high-consequence routine commands.
- **Two-factor prompts MUST NOT be configurable on emergency or protective
  commands.** Forcing an operator to find a phone and type a code while a
  reactor overheats is a hazard, not a control. (The deeper truth: a true
  emergency stop is a hardwired circuit and was never IRON's job — see below.) `REQ-CMD-003 integration`

## What the command path is not

- **Not a safety system.** Emergency stops, trips, and SIL-rated interlocks are
  hardwired or live in safety PLCs. IRON observes their state; it is never the
  channel that actuates them. A `reactor_01.emergency_shutdown` button in a
  SCADA UI, if a plant chooses to have one, is a *request* to the PLC's own
  shutdown logic — the PLC sequence is the protection, IRON is one way to ask
  for it.
- **Not an automation engine.** Scheduled or condition-triggered writes
  ("every night at 2am close the valve", "open the vent above 30°C") are out
  of scope for v1: closed-loop logic belongs in a controller next to the
  process, and IRON supervises it by writing setpoints. A site with no
  controller at all is therefore not a v1 site
  ([guides/hardware.md](../guides/hardware.md), Level 1). Supervisory
  automation is *deferred* with its trigger in
  [business/deferred.md](../business/deferred.md); if it arrives, it will be
  explicit declarations in `config/commands/` that run through this same
  pipeline with a service identity — never side effects of alarms or
  dashboards.
