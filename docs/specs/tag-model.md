---
status: draft
type: spec
depends-on: [edge-agent, alarm-engine]
---

# Tag Model

A tag maps a physical signal to a named value in the system. Tags are the core
abstraction of IRON. This document defines the tag specification format and its
invariants.

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
> See [alarm-engine.md](alarm-engine.md) and [command-path.md](command-path.md).

## Fields

| Field | Required | Meaning |
|---|---|---|
| `source` | yes | Signal address as a URI (scheme = protocol driver) |
| `type` | yes | `bool`, `int16`, `int32`, `uint16`, `uint32`, `float32`, `float64` |
| `unit` | for analog | Engineering unit string, display only |
| `scan_rate` | no (default `1s`) | Polling interval |
| `deadband` | no (default `0`) | Minimum change that triggers a publish — see [edge-agent.md](edge-agent.md) |
| `range` | for analog | `[min, max]` engineering range; also drives 4–20mA scaling |
| `scaling` | no | Explicit raw→engineering conversion when convention is not enough |
| `alarms` | no | Alarm definitions — semantics in [alarm-engine.md](alarm-engine.md) |

### Source URI schemes

| Scheme | Example | Driver |
|---|---|---|
| `modbus://` | `modbus://plc-01/holding/0x1000` | Modbus TCP (host alias or IP) |
| `modbus+rtu://` | `modbus+rtu:///dev/ttyUSB0/1/holding/0x1000` | Modbus RTU over RS-485 |
| `opcua://` | `opcua://plc-02/ns=3;s=Reactor.Temp` | OPC-UA client |
| `s7://` | `s7://plc-03/DB10.DBD4` | Siemens S7 |
| `mqtt://` | `mqtt://broker/sensors/gh1/temp` | MQTT subscribe |
| `i2c://`, `gpio://`, `1wire://`, `spi://` | `i2c://1/0x44/sht31` | Direct Linux I/O (Level 1 hardware) |
| `sim://` | `sim://sine:20:180:60s` | Built-in simulator |

Host aliases (`plc-01`) are resolved in `config/connections.yaml`, so an IP change
is a one-line diff.

## Invariants (enforced by `iron validate`)

- Alarm limits MUST lie within `range`. A `high_high` of 220 with `range: [0, 200]`
  is a specification error: the alarm could never trigger.
- Alarm limits MUST be ordered: `low_low < low < high < high_high`.
- Every `source` MUST parse against a known scheme; reachability is checked
  when the target is online (warning otherwise, not an error).
- A tag name MUST be unique across the project; the full dotted path
  (`reactor_01.temperature`) is the canonical identity used in NATS subjects,
  the historian, dashboards, and mimics.
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
(report-by-exception) data are defined in [historian.md](historian.md).

## Scaling convention

For analog inputs, `type: float32` + `range: [min, max]` implies linear 4–20mA
scaling by convention (4mA → min, 20mA → max). When the raw signal is not
4–20mA, declare scaling explicitly:

```yaml
scaling:
  raw: [0, 27648]        # Siemens analog raw range
  engineering: [0, 200]
```

Raw values outside the declared raw range MUST clamp to the boundary and degrade
quality to `UNCERTAIN` (broken wire / sensor fault detection).

## Repeating yourself is a smell

If you are writing the same eight tags for the fifth pump, stop — define a
device type once and instantiate it. See [device-types.md](device-types.md).
