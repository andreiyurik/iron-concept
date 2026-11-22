# Example: Greenhouse

A complete, realistic IRON project for a two-zone greenhouse — the kind of
site the `greenhouse` template of `iron new` will scaffold. **Status:
`iron validate` accepts this project and `iron test --sim` runs its scenario;
nothing can run it against real devices yet.** The files exist so that:

- the spec formats ([tag-model](../../docs/specs/tag-model.md),
  [command-path](../../docs/specs/command-path.md),
  [testing](../../docs/specs/testing.md)) are exercised against a real site,
  not just described;
- an automation engineer can review concrete tags and tell us what the
  greenhouse knows that we don't ([CONTRIBUTING](../../CONTRIBUTING.md));
- `iron validate` has its first fixture: **every file here MUST
  pass validation, forever** — this example goes red in CI the moment a spec
  change breaks it.

## The site

Two zones, one Raspberry Pi running the single-binary default
([deployment](../../docs/specs/deployment.md)), sensors on two ESP32 Modbus
nodes, one controllable vent. 15 tags (14 measured, 1 derived), 1 command,
1 simulation scenario.

```
examples/greenhouse/
  config/
    tags/zone_1.yaml       air temp · humidity · VPD (derived) · soil moisture ×2 · CO2
    tags/zone_2.yaml       air temp · humidity · soil moisture ×2
    tags/site.yaml         outdoor temp · water tank level · pump running · power ok · vent position
    commands/vent.yaml     the one WRITE surface — deliberately separate
    connections.yaml       the two nodes: address, unit id, byte order (optional file)
  test/sim/scenarios/
    frost_night.yaml       outdoor temp falls below zero → frost alarm fires
```

Three things the example makes concrete:

- **Who controls the vent.** Not IRON. The ESP32 node runs the loop that
  moves the vent toward its setpoint and holds it there; IRON's one command
  writes that setpoint, and `site.vent_position` reads back what the node
  did. Switch the Pi off and the greenhouse keeps regulating
  ([hardware guide](../../docs/guides/hardware.md), Level 1).
- **A computed value is a tag.** `zone_1.vpd` is one `derived:` line — no
  plugin, no widget logic — and carries its own alarm
  ([expressions](../../docs/specs/expressions.md)).
- **No implicit scaling.** The nodes publish engineering values in float
  registers, so no tag declares `scaling`; `range` only says what is
  plausible ([tag model](../../docs/specs/tag-model.md)). The byte order of
  those floats is a property of the node and lives in `connections.yaml`.

Note what the project does **not** contain — and still works, by the
[complexity budget](../../docs/specs/cli.md): no dashboards (the
auto-dashboard renders every tag), no device types (15 tags don't need
templating yet), no mimics, no plugins. Each of those is a layer this site
adds only when it grows.
