---
status: draft
type: spec
depends-on: [command-path, security, transport]
---

# READ/WRITE Path Separation

The most important architectural property in IRON. Every design decision is
evaluated against it.

```
READ:   Sensors → Edge → transport (data) → Storage → UI
        (one direction, always, no exceptions)

WRITE:  Operator → Auth → RBAC → Command Service → Audit → transport (commands)
        → Edge → Machine
        (explicit, authorized, fully logged, confirmed)
```

In classical SCADA monoliths (WinCC, Wonderware, MasterSCADA) any tag is
potentially writable and any screen script can write it. That is the
architectural mistake IRON refuses to make.

## The guarantee

Stated precisely, because the precise form is the one that can be tested:

> **Nothing in IRON can change the state of a machine except the Command
> Service, and the Command Service can write only to targets declared in
> `config/commands/`, only after authentication, authorization, validation,
> and a journal entry.**

Consequences that follow from it:

- A display widget, a trend, an alarm, a derived tag, an expression, a
  plugin, the historian, the fallback page, and the agent interface have no
  way to cause a write — not by bug, not by misconfiguration.
- The set of everything a site can write is a reviewable list of files.

What the guarantee is **not**: it does not say the browser cannot send
commands. The operator UI contains control widgets on purpose; clicking one
submits a command *request*, which then passes the whole WRITE path. The
guarantee is about what happens to that request and about everything that is
not one.

## How it is enforced in every deployment

These hold in the default single-process deployment and in plant mode alike.

### By construction, in code

- A protocol driver exposes two capabilities: a read handle and a **write
  handle** ([protocol-drivers.md](protocol-drivers.md)). Write handles are
  created once at startup and given only to the edge's command executor.
- The command executor accepts input only from the Command Service's
  channel. There is no other caller.
- The Command Service is a distinct crate. READ-path crates — tag engine,
  alarm engine, historian, real-time fan-out, fallback page, plugin port —
  MUST NOT depend on it or on the driver write API. **CI enforces this on
  the crate dependency graph**; a pull request that adds such an edge fails. `REQ-RWS-001 ci`
- `iron-domain` is shared by both paths. It is pure logic with no I/O and
  can perform neither a read nor a write.

### By the shape of the configuration

- Tags (READ) live in `config/tags/`. Commands (WRITE) live in
  `config/commands/`. A tag definition MUST NOT contain a writable target. `REQ-RWS-002 integration`
- A control widget MUST reference a command definition; binding one to a tag
  is a validation error ([visual-system.md](visual-system.md)). `REQ-RWS-003 integration`
- The alarm engine is a READ-path consumer. An alarm MUST NOT trigger a
  command — see [alarm-engine.md](alarm-engine.md) for why, and for what to
  do instead. `REQ-RWS-004 integration`
- No expression context exists on the WRITE path
  ([expressions.md](expressions.md)), and no plugin or adjustment schema has
  a field that can name a command.

```yaml
# config/commands/reactor_01.yaml — physically separate from tags/
reactor_01:
  pump_start:
    target: modbus://plc-01/coil/0x0020
    type: bool
    requires_confirm: true
    requires_role: operator
```

### By the shape of the API

- The real-time WebSocket MUST be **read-only**: its protocol has message types
  for subscribing and for receiving data, and none for writing. A client
  cannot express a command on it. `REQ-RWS-007 integration`
- Commands are submitted through one authenticated endpoint of the Command
  Service API and nowhere else. The console and the REST API use the same
  endpoint ([command-path.md](command-path.md)).

### At the edge

- The command executor validates every command against its own copy of
  `config/commands/` and MUST refuse anything not declared there, whatever the
  server says. `REQ-RWS-008 integration`
- The local fallback page is served by code with no path to the command
  executor ([edge-agent.md](edge-agent.md)).

## Additional enforcement in plant mode

When edge and server are separate processes on a broker, the separation is
also a property of transport authorization, not only of code:

- Data and commands use disjoint subject roots
  ([transport.md](transport.md)).
- Each edge's credentials allow it to **publish only** its own data, alarm,
  and result subjects and **subscribe only** to its own command subjects. An
  edge's credentials MUST NOT permit publishing on `cmd.>` — a compromised
  edge device cannot inject commands into another area. `REQ-RWS-005 integration`
- Only the Command Service identity may publish on `cmd.>`. No other
  component — not the dashboard, not the alarm engine, not the historian,
  not a plugin — holds publish rights there.
- All broker connections MUST use TLS; edge credentials SHOULD be per-device
  keys (mTLS or NATS NKeys), individually revocable. `REQ-RWS-006 integration`

The per-identity permission table is in [security.md](security.md).

One further level is *deferred*: running the outward-facing role on a host
that holds **no command credentials at all**, so the keys are not there to
steal ([security.md](security.md), contours;
[business/deferred.md](../business/deferred.md)).

## What this eliminates

- A display, trend, alarm, or expression causing a write
- A developer wiring a read widget to a write target
- Writing to an address nobody declared and reviewed
- In plant mode: a compromised edge device writing into another area, and a
  compromised READ-path service forging commands

## What this does not eliminate

- **A compromised operator session.** Malicious script running in an
  operator's browser can request any command that operator is authorized
  for. RBAC scoping, confirmation, limits, cooldowns, and the audit journal
  bound the damage ([command-path.md](command-path.md)); they do not prevent
  it.
- **A compromised server host**, or a vulnerability in the Command Service
  itself, in the default deployment where the service and its credentials
  share a host with the UI.
- **A malicious insider** with valid credentials.

Safety-critical protection never depends on IRON at all: see "What IRON does
not claim" in [security.md](security.md).
