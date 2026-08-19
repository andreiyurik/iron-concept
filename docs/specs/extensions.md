---
status: draft
type: spec
depends-on: [read-write-separation, edge-agent, tag-model]
---

# Extension Model

How integrators and contributors extend IRON **without writing Rust or
touching server internals** and without forking. Decision rationale:
[ADR 0008](../decisions/0008-extensions-beyond-elixir.md).

> **v1 scope.** The first release ships **plugins** and the **REST /
> WebSocket API** — both are "a program in any language with its own
> credentials", which is zero new host code in IRON. In-process WASM modules
> (edge and server), the `iron module` command, and `iron contract diff` are
> specified below but *deferred*, each with its trigger, in
> [business/deferred.md](../business/deferred.md). Plugins are the proving
> ground: a need no plugin can meet is what brings WASM back.

```
v1 ─ Out-of-process (plugins):       v1 ─ External (REST / WebSocket API):
     custom protocol drivers                 BI tools, mobile apps, ERP sync
     analytics / ML inference
     derived tags, bridges              deferred ─ In-process (sandboxed WASM):
                                             edge: conversions, payload framing
                                             server: alarm enrichment, notification channels
```

## Plugins (v1)

An IRON plugin is a **separate program** — it never runs inside the core.
For anything that is a program — a custom protocol driver, an ML model, a
bridge to a legacy system, a derived-tag calculator — write a normal program
in any language and connect it to NATS with its own credentials:

```
plugin "vibration-ml" (Python):
  credentials: subscribe data.plant.line1.vibration.>
               publish   data.plant.line1.vibration_health.>
  → reads raw spectra, publishes a health-score derived tag
```

- Plugins are declared in `config/plugins/` (name, language-agnostic,
  the subjects it may subscribe to and publish on); `iron deploy` provisions
  their credentials from that declaration, so a plugin cannot be granted
  more than its spec says.
- Plugins authenticate like edge agents: per-identity keys, subject-scoped
  permissions, revocable individually ([security.md](security.md)). A
  plugin never receives `cmd.>` publish rights — the declaration schema
  has no field that could name a command subject.
- A plugin that publishes tags MUST follow the tag value contract
  (value/quality/timestamp — [tag-model.md](tag-model.md)); `iron validate`
  checks declared plugin tags like any others, and they are first-class:
  quality, historian, alarms.
- Custom *protocol drivers* are plugins by default (any language, crash in
  isolation); drivers graduate into iron-core (Rust) when they prove general
  demand.
- In the single-binary default deployment there is no NATS; a plugin
  connects to the `iron` process's built-in plugin port with the same
  credential model. The plugin's code does not change between modes.

### Reference skeletons: Python and Rust

Plugins are language-agnostic; the *skeletons IRON ships* are a choice
about who the first contributor is, not about which language is best:

```bash
iron new plugin dew_point --lang python   # the audience: automation & data engineers already write Python
iron new plugin dew_point --lang rust     # the graduation path: a proven plugin moves into iron-core with little rewriting
```

Each skeleton is a complete, runnable plugin: the declaration in
`config/plugins/`, a client that connects with the provisioned
credentials, a subscribe → compute → publish loop that emits well-formed
`{value, quality, timestamp}` samples, a test against a recorded sample
stream, and a README. The Python skeleton uses `nats-py`; protocol work
reaches for the libraries engineers already have (`pymodbus`, `asyncua`,
`python-snap7`, `numpy`). Skeletons for other languages (Ruby on
`nats-pure`, Go, C#) are welcome as community contributions under
`examples/plugins/` — the contract is the same 40 lines in every language.

### Protocol drivers as plugins: the proving ground

A community driver starts as a plugin and earns its way into the core.
The evidence is a test, not an opinion:

```bash
iron test --driver ./modbus_rtu_plugin --against diagslave     # Modbus
iron test --driver ./s7_plugin --against snap7                 # S7
iron test --driver ./opcua_plugin --against open62541          # OPC-UA
```

`iron test --driver` runs the plugin against a protocol simulator with a
recorded register map, checks that every declared tag is published with the
right value, quality, and timing, injects faults (timeout, disconnect,
out-of-range), and reports diagnostics in the standard format
([agent-interface.md](agent-interface.md)). A driver that passes is eligible
for the `verified` tier below; a driver that proves general demand graduates
into `iron-core` in Rust.

### Trust tiers

A plant will not run an unknown program next to a PLC, and it should not
have to guess:

| Tier | Meaning | Who grants it |
|---|---|---|
| `community` | Declared, validated, runs in isolation with scoped credentials — that is already more than most scripting in SCADA offers | anyone, by publishing |
| `verified` | Passes `iron test --driver` (or the plugin test harness) in IRON's CI against the reference simulators; pinned version, signed | maintainers, by running the harness |
| `core` | Part of `iron-core`, Rust, covered by the scaling benchmark and the hardware-in-the-loop bench | maintainers, by merging |

The tier is visible in the declaration and in `iron explain`; a site MAY
refuse to deploy anything below a chosen tier (`deploy.yml:
min_plugin_tier: verified`).

## REST / WebSocket API (v1)

The boring, stable layer for external consumers: read tags and history, query
alarms, subscribe to live updates, submit commands (full WRITE-path semantics:
auth, RBAC, audit — [command-path.md](command-path.md)). Versioned, documented
OpenAPI; the same API the UI itself uses, so it cannot rot.

## WASM modules (deferred)

A module is a WASM binary plus a manifest:

```yaml
# modules/dew_point/module.yaml
name: dew_point
version: 0.2.0
runs_on: server                # edge | server | both
capabilities:
  subscribe: ["data.plant.gh1.temperature", "data.plant.gh1.humidity"]
  publish:   ["data.plant.gh1.dew_point"]   # derived tag it may create
inputs:
  - { tag: "gh1.temperature", as: t }
  - { tag: "gh1.humidity",   as: rh }
```

Normative rules:

- Modules run sandboxed: no filesystem, no network, no clock beyond what the
  host interface grants. The host passes samples in; the module returns
  samples (or notification payloads) out.
- **Capabilities are declared in the manifest and enforced by the host.** A
  module's `publish` list may contain data subjects only — the manifest
  schema has no field that could name a command subject. READ/WRITE
  separation extends to modules by construction
  ([read-write-separation.md](read-write-separation.md)).
- Derived tags produced by modules are first-class tags: they carry quality
  (worst-of-inputs by default), appear in the historian, can have alarms.
- Notification-channel modules receive alarm events and return outbound
  payloads; the host — not the module — performs delivery, so a module
  cannot exfiltrate data beyond its declared channel.
- Module updates deploy without recompiling or restarting the host
  (`iron module add ./dew_point.wasm`, hot-swapped between scan cycles).
- The same module binary MUST run unmodified inside `iron test --sim` —
  modules are testable without hardware, like everything else.

## What is deliberately NOT extensible

- Server internals (tag-engine layout, internal channels, crate APIs) —
  private, may change any release.
- The Command Service pipeline — no extension hooks before or after
  authorization/audit. Extending the WRITE path means proposing a change to
  IRON itself, in the open.
- The alarm engine's state machine — extensions enrich and notify; they cannot
  suppress or acknowledge.

## Contract stability

The plugin tag/subject contract and the REST/WebSocket API are versioned
with semver and a written changelog; a community driver that worked on
version N keeps working on version N+1, or the release notes say why not.
Automated breaking-change detection (`iron contract diff`) is *deferred*
until there is a second stable contract to diff
([deferred.md](../business/deferred.md)).

## Distribution

Plugins are ordinary programs and ship however their language ships.
When WASM modules arrive, `iron module add <path|url>` installs into the
project (vendored, committed to Git like all configuration — a plant's
deployment never depends on a registry being online).
