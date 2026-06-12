---
status: draft
type: spec
depends-on: [command-path, security]
---

# READ/WRITE Path Separation

The most important architectural property in IRON. Every design decision is
evaluated against it.

```
READ:   Sensors → Edge → NATS (data subjects) → Storage → UI
        (one direction, always, no exceptions)

WRITE:  Operator → Auth → RBAC → Command Service → Audit → NATS (command subjects)
        → Edge → Machine
        (explicit, authorized, fully logged, confirmed)
```

A visualization bug must be incapable of sending a command to a machine.
In classical SCADA monoliths (WinCC, Wonderware, MasterSCADA) these paths are
entangled in a single process. That is the architectural mistake IRON refuses
to make.

## Normative requirements

### Code and configuration

- READ and WRITE paths MUST NOT share code, processes, or configuration files.
- Tags (READ) live in `config/tags/`. Commands (WRITE) live in `config/commands/`.
  A tag definition MUST NOT contain a writable target. A command definition MUST
  NOT be referenced by display widgets except through the Command Service API.
- The alarm engine is a READ-path consumer. An alarm MUST NOT trigger a command.
  See [alarm-engine.md](alarm-engine.md) for why, and for what to do instead.

```yaml
# config/commands/reactor_01.yaml — physically separate from tags/
reactor_01:
  pump_start:
    target: modbus://plc-01/coil/0x0020
    type: bool
    requires_confirm: true
    requires_role: operator
```

### Transport

Logical separation is not enough: both paths traverse the same NATS broker, so
the separation MUST also be enforced at the transport authorization layer.

- Data and commands MUST use disjoint subject namespaces:
  `data.>` and `cmd.>` (no subject belongs to both).
- Each edge agent MUST hold credentials that allow it to **publish only** to its
  own data subjects (`data.<site>.<area>.>`) and **subscribe only** to its own
  command subjects (`cmd.<site>.<area>.>`). An agent's credentials MUST NOT
  permit publishing to `cmd.>` — a compromised edge device cannot inject
  commands to other zones.
- Only the Command Service identity may publish to `cmd.>`. No other component —
  not the dashboard, not the alarm engine, not the historian — holds publish
  rights on command subjects.
- All NATS connections MUST use TLS; agent credentials SHOULD use mTLS or NATS
  NKeys with per-device keys, revocable individually.

This makes the separation a property of broker authorization, not a convention
in application code. A compromise of any READ-path component yields zero
capability to write. Full threat model: [security.md](security.md).

### Process

- iron-web's Command Service is the only WRITE entry point. It runs as a
  distinct OTP application boundary with its own supervision tree; the
  rendering layer calls it through one narrow, auditable API.
- The edge agent's command executor is a distinct module from the polling
  engine. It executes only commands received on its authorized command
  subjects, validates them against `config/commands/`, and refuses anything
  not declared there.

## What this eliminates

- A compromised dashboard sending spurious commands to machines
- A developer accidentally wiring a read widget to a write endpoint
- Privilege escalation from the UI layer to the command layer
- A compromised edge device in one area writing to another area

## What this does not eliminate

A malicious insider with valid operator credentials, a compromised iron-web
host, or a vulnerability in the Command Service itself can still send
authorized-looking commands. Defense in depth — audit logging, confirmation,
RBAC scoping, command rate limits ([command-path.md](command-path.md)) — exists
for exactly this reason. And safety-critical protection never depends on IRON
at all: see "What IRON does not claim" in [security.md](security.md).
