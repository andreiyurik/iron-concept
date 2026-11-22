---
status: draft
type: spec
depends-on: [edge-agent, alarm-engine, expressions, protocol-drivers]
---

# Tag Model

A tag is a named value with quality and time. Most tags map a physical
signal; some are computed from other tags. Tags are the core abstraction of
IRON. This document defines the tag specification format and its invariants.

## The specification file

Tags are defined in YAML under `config/tags/`. The YAML file is the single source
of truth: the edge agent configures polling from it, `iron validate` checks it,
`iron generate` derives tests and dashboards from it, `iron field` derives
verification steps from it.

```yaml
# config/tags/reactor_01.yaml
reactor_01:
  temperature:
    source: modbus://plc-01/holding/0x1000
    type: float32
    unit: "°C"
    scan_rate: 1s
    deadband: 0.5
    range: [0, 200]
    alarms:
      high:
        limit: 180
        priority: 2
        message: "High reactor temperature"
      high_high:
        limit: 195
        priority: 1
        message: "Critical high reactor temperature"
```

> **Note what is absent.** Alarms carry no `action:` field. An IRON alarm
> notifies humans; it MUST NOT trigger commands. Automatic protective reactions
> (trips, interlocks, emergency shutdown) belong in the PLC or the safety system,
> where no network or broker sits between cause and effect.
> See [alarm-engine.md](alarm-engine.md) and [command-path.md](command-path.md). `REQ-TAG-001 unit`

## Fields

| Field | Required | Meaning |
|---|---|---|
| `source` | one of `source` / `derived` | Signal address as a URI (scheme = protocol driver) |
| `derived` | one of `source` / `derived` | An expression over other tags — see [Derived tags](#derived-tags) |
| `type` | yes | `bool`, `int16`, `int32`, `uint16`, `uint32`, `float32`, `float64` |
| `unit` | for analog | Engineering unit string, display only |
| `scan_rate` | no (default `1s`) | Polling interval. Not allowed on derived tags |
| `deadband` | no (default `0`) | Minimum change that triggers a publish — see [edge-agent.md](edge-agent.md) |
| `range` | for analog | `[min, max]` in engineering units: the values the tag can legitimately take |
| `scaling` | no | Raw→engineering conversion — see [Scaling](#scaling) |
| `signal` | no | The physical signal (`4-20mA`, `0-10V`, `pt100`, `dry-contact`, …); documentation that also selects the [field verification](field-verification.md) protocol |
| `alarms` | no | Alarm definitions — semantics in [alarm-engine.md](alarm-engine.md) |

### Source URI schemes

| Scheme | Example | Driver |
|---|---|---|
| `modbus://` | `modbus://plc-01/holding/0x1000` | Modbus TCP |
| `modbus+rtu://` | `modbus+rtu://vfd-01/holding/0x1000` | Modbus RTU over RS-485 |
| `opcua://` | `opcua://plc-02/ns=3;s=Reactor.Temp` | OPC-UA client |
| `s7://` | `s7://plc-03/DB10.DBD4` | Siemens S7 |
| `mqtt://` | `mqtt://broker/sensors/gh1/temp` | MQTT subscribe |
| `i2c://`, `gpio://`, `1wire://`, `spi://` | `i2c://1/0x44/sht31` | Direct Linux I/O (Level 1 hardware) |
| `sim://` | `sim://sine:20:180:60s` | Built-in simulator |

The host part is an alias from `config/connections.yaml` or a literal
hostname or IP address. Connection parameters, the exact Modbus address
syntax (tables, zero-based addresses, bit access, byte order), and how polls
are grouped are specified in [protocol-drivers.md](protocol-drivers.md).
The table above lists the schemes IRON intends to support; only those with
an addressing section in that document are implemented.

## Invariants (enforced by `iron validate`)

- A tag MUST have exactly one of `source` and `derived`. `REQ-TAG-002 unit`
- Alarm limits MUST lie within `range`. A `high_high` of 220 with `range: [0, 200]`
  is a specification error: the alarm could never trigger. `REQ-TAG-003 unit`
- Alarm limits MUST be ordered: `low_low < low < high < high_high`. `REQ-TAG-004 unit`
- Every `source` MUST parse against a known scheme; reachability is checked
  when the target is online (warning otherwise, not an error). `REQ-TAG-005 unit`
- A tag name MUST be unique across the project; the full dotted path
  (`reactor_01.temperature`) is the canonical identity used by the
  [transport](transport.md), the historian, dashboards, mimics, and
  expressions. The first segment is the tag's **area**. `REQ-TAG-006 unit`
- Names beginning with `_` are reserved for IRON's own system tags
  ([protocol-drivers.md](protocol-drivers.md)); a project MUST NOT define
  them. `REQ-TAG-007 unit`
- Tags are READ-path only. Anything writable is a command and lives in
  `config/commands/` — never in a tag file. See
  [read-write-separation.md](read-write-separation.md).

## Tag value semantics

Every published tag value carries:

| Field | Meaning |
|---|---|
| `value` | Engineering-unit value after conversion |
| `quality` | `GOOD` / `UNCERTAIN` / `BAD` — see [edge-agent.md](edge-agent.md) |
| `timestamp` | Assigned by the edge agent at acquisition time (UTC) |

Consumers MUST treat `quality` as part of the value. A dashboard rendering a
`BAD` value as if it were `GOOD` is a bug. Historian semantics for sparse
(report-by-exception) data are defined in [historian.md](historian.md). `REQ-TAG-008 unit`

## Scaling

**There is no implicit scaling.** Without a `scaling` field, the value read
from the device MUST be taken as the engineering value, unchanged — the common case for a
controller that already holds 87.5 in a float register. `REQ-TAG-011 unit`

When the device delivers a raw quantity, declare the conversion:

```yaml
scaling:
  raw: [0, 27648]        # Siemens analog raw range
  engineering: [0, 200]
```

```yaml
signal: 4-20mA
scaling:
  raw: [4, 20]           # an input module that reports milliamps
  engineering: [0, 200]
```

Normative rules:

- Scaling is linear between the two declared pairs.
- With `scaling`: a raw value outside the declared raw range MUST clamp to
  the boundary and degrade quality to `UNCERTAIN` — this is how a broken
  wire (below 4 mA) or a saturated sensor is detected. `REQ-TAG-009 unit`
- Without `scaling`: a value outside `range` MUST be published as read, with
  quality `UNCERTAIN`. IRON does not invent a number the device did not send. `REQ-TAG-012 unit`
- `signal` never changes a value. It records what is physically wired, so
  that `iron field` knows to say "apply 4 mA" rather than "apply 0 V".

## Derived tags

A derived tag is computed from other tags by an expression
([expressions.md](expressions.md)). It is a tag in every respect: quality,
history, alarms, dashboards.

```yaml
# config/tags/zone_1.yaml
zone_1:
  vpd:                       # vapour-pressure deficit — what the crop actually feels
    derived: "0.6108 * exp(17.27 * zone_1.air_temperature / (zone_1.air_temperature + 237.3)) * (1 - zone_1.humidity / 100)"
    type: float32
    unit: "kPa"
    deadband: 0.05
    range: [0, 8]
    alarms:
      high:
        limit: 1.6
        priority: 3
        message: "Zone 1 VPD high — plants closing stomata"
```

Normative rules:

- A derived tag MUST be re-evaluated whenever one of its inputs produces a
  new sample. It has no scan rate of its own. `REQ-TAG-013 unit`
- Quality and timestamp follow the rules in
  [expressions.md](expressions.md): worst quality of the inputs; time of the
  triggering input.
- The result passes the same publish filter as any tag — deadband, quality
  change, alarm-limit crossing, heartbeat ([edge-agent.md](edge-agent.md)).
- A derived tag is declared in the area it belongs to; its inputs may come
  from any area.
- **Where it is evaluated is determined, not configured.** If every input is
  owned by one edge, that edge evaluates the tag — on every scan, from
  pre-deadband values, so its alarms keep working when the server is
  unreachable. If the inputs span edges, the server evaluates it from the
  samples it receives. `iron explain` shows which.
- Alarms on a server-evaluated derived tag depend on the server and on the
  link. `iron validate` MUST warn when such a tag carries a priority-1
  alarm. `REQ-TAG-010 unit`

This is how display logic stays out of the UI: if a number deserves to be
shown, it deserves to be a tag ([visual-system.md](visual-system.md)).
Computations that need state, history, or a library are plugins
([extensions.md](extensions.md)).

## Repeating yourself is a smell

If you are writing the same eight tags for the fifth pump, stop — define a
device type once and instantiate it. See [device-types.md](device-types.md).
