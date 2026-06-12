---
status: draft
type: spec
depends-on: [tag-model, alarm-engine, command-path]
---

# Device Types

Nobody hand-writes 100,000 tags. Real plants are built from repeating
equipment: the same pump 40 times, the same air handler 12 times. Device types
are IRON's answer — define the shape once, instantiate it per device, override
only what differs. (Ignition users know this as UDTs; Rails users as models.)

## Defining a type

```yaml
# config/types/centrifugal_pump.yaml
type: centrifugal_pump
params:                          # filled in at instantiation
  plc: {}                        # connection alias
  base: {}                       # register base address
  rated_current: { default: 12.0 }

tags:
  running:
    source: "modbus://{plc}/coil/{base}"
    type: bool
  current:
    source: "modbus://{plc}/holding/{base + 0x10}"
    type: float32
    unit: "A"
    range: [0, "{rated_current * 1.5}"]
    deadband: 0.2
    alarms:
      high:
        limit: "{rated_current * 1.1}"
        priority: 2
        message: "Pump overcurrent"

commands:                        # generated into config/commands/, never config/tags/
  start:
    target: "modbus://{plc}/coil/{base + 0x20}"
    type: bool
    requires_role: operator
    requires_confirm: true
    feedback_tag: "{instance}.running"
    feedback_timeout: 5s

dashboard:                       # default widget composition for one instance
  - { type: status, tag: "{instance}.running" }
  - { type: numeric, tag: "{instance}.current" }
```

## Instantiating

```yaml
# config/tags/line_1.yaml
instances:
  pump_01: { type: centrifugal_pump, plc: plc-01, base: 0x0100 }
  pump_02: { type: centrifugal_pump, plc: plc-01, base: 0x0200 }
  pump_03:
    type: centrifugal_pump
    plc: plc-02
    base: 0x0100
    rated_current: 18.0          # override per instance
    tags:
      current:
        alarms:
          high: { limit: 21.0 }  # deep override when convention isn't enough
```

Three lines per pump instead of forty. `iron validate` expands instances and
validates the result exactly as if the tags were written by hand — the
expansion is deterministic and inspectable (`iron expand pump_03` prints the
resulting YAML).

## Normative rules

- A type MUST keep tags and commands in separate sections; expansion writes
  them to separate config trees, preserving READ/WRITE separation.
- Parameter substitution is explicit (`{param}`) with compile-time arithmetic
  only — no runtime evaluation, nothing Turing-complete. If a type needs
  logic, it needs a WASM module or a developer, not a templating language.
- Types MAY extend other types (`extends: pump`), single inheritance only.
- Changing a type re-expands all instances; `iron validate` reports the full
  diff of effective configuration, so a one-line type edit that touches 40
  pumps is reviewed as exactly that.

## Importing the real world: IO lists

Plants do not start from YAML — they start from an instrument/IO list, usually
an Excel sheet from the electrical designer. IRON treats that as a first-class
input:

```bash
iron import io-list ./P-2024-117_IO_List.xlsx \
  --map "Tag No=name, Description=description, Range Min=range.0, \
         Range Max=range.1, Signal=signal_type, Address=source"

# → generates config/tags/*.yaml (and a report of rows it could not map)
# ✅ 412 tags generated
# ⚠️  17 rows skipped: no address column value (listed in import-report.md)
```

Combined with device types (`--type-by "Equipment Type"` maps rows to
instances), an integrator turns the project IO list into validated
configuration in minutes — the single largest time sink in traditional SCADA
engineering.

## Standard library

IRON ships starter types: `centrifugal_pump`, `valve_onoff`, `control_valve`,
`motor`, `tank`, `ahu`, `greenhouse_zone`. They are conventions to copy and
adapt, not certified equipment models.
