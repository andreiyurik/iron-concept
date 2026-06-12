---
status: draft
type: spec
depends-on: [read-write-separation, security, edge-agent]
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
  → NATS command subject (cmd.<site>.<area>.…)
  → Edge agent: validate against config/commands/ → protocol write → read-back
  → Result event (ACK / NACK) → audit record completed → UI feedback
```

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

## Command lifecycle

Every command is a tracked state machine, not a fire-and-forget write:

```
ISSUED → DELIVERED (edge received) → EXECUTED (protocol write OK)
       → CONFIRMED (feedback_tag reached expected state)
       | FAILED (NACK / protocol exception / feedback timeout)
       | EXPIRED (not delivered within TTL — edge offline)
```

- Commands carry a TTL (default 10s). A command that cannot be delivered while
  the operator is watching MUST expire rather than execute unexpectedly minutes
  later when the link returns. Stale commands are the most dangerous kind.
- Every state transition is appended to the audit record.
- The UI shows the lifecycle explicitly — an operator always knows whether the
  command was confirmed, failed, or expired.

## Audit policy — availability over centralized logging

Every command MUST be journaled before it is sent. But "journaled" means a
**local append-only journal on the iron-web host** (fsync'd file or local
table), not a synchronous write to the central historian.

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
  emergency stop is a hardwired circuit and was never IRON's job — see below.)

## What the command path is not

- **Not a safety system.** Emergency stops, trips, and SIL-rated interlocks are
  hardwired or live in safety PLCs. IRON observes their state; it is never the
  channel that actuates them. A `reactor_01.emergency_shutdown` button in a
  SCADA UI, if a plant chooses to have one, is a *request* to the PLC's own
  shutdown logic — the PLC sequence is the protection, IRON is one way to ask
  for it.
- **Not an automation engine.** Scheduled or condition-triggered writes
  ("every night at 2am close the valve") are out of scope for v1. When they
  arrive, they will be explicit declarations in `config/commands/` that run
  through this same pipeline with a service identity — never side effects of
  alarms or dashboards.
