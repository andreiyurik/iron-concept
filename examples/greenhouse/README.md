# Example: Greenhouse

A complete, realistic IRON project for a two-zone greenhouse — the kind of
site the `greenhouse` template of `iron new` will scaffold. **Status: these
files demonstrate the target configuration format defined in the specs; no
software exists yet to run them.** They exist so that:

- the spec formats ([tag-model](../../docs/specs/tag-model.md),
  [command-path](../../docs/specs/command-path.md),
  [testing](../../docs/specs/testing.md)) are exercised against a real site,
  not just described;
- an automation engineer can review concrete tags and tell us what the
  greenhouse knows that we don't ([CONTRIBUTING](../../CONTRIBUTING.md));
- the future `iron validate` has its first fixture: **every file here MUST
  pass validation, forever** — this example goes red in CI the moment a spec
  change breaks it.

## The site

Two zones, one Raspberry Pi running the single-binary default
([deployment](../../docs/specs/deployment.md)), sensors on two ESP32 Modbus
nodes, one controllable vent. 14 tags, 1 command, 1 simulation scenario.

```
examples/greenhouse/
  config/
    tags/zone_1.yaml       air temp · humidity · soil moisture ×2 · CO2
    tags/zone_2.yaml       air temp · humidity · soil moisture ×2
    tags/site.yaml         outdoor temp · water tank level · pump running · power ok · vent position
    commands/vent.yaml     the one WRITE surface — deliberately separate
  test/sim/scenarios/
    frost_night.yaml       outdoor temp falls below zero → frost alarm fires
```

Note what the project does **not** contain — and still works, by the
[complexity budget](../../docs/specs/cli.md): no dashboards (the
auto-dashboard renders every tag), no device types (14 tags don't need
templating yet), no mimics, no plugins. Each of those is a layer this site
adds only when it grows.
