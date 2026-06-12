---
status: draft
type: spec
depends-on: [read-write-separation, edge-agent, tag-model]
---

# Extension Model

How integrators and contributors extend IRON **without writing Elixir** and
without forking. Decision rationale:
[ADR 0008](../decisions/0008-extensions-beyond-elixir.md).

```
In-process (sandboxed WASM):           Out-of-process (NATS satellites):
  edge:   derived tags, conversions,     custom protocol drivers
          custom payload framing         analytics / ML inference
  server: computed tags, alarm           bridges to other systems
          enrichment, notification
          channels, widget data feeds  External (REST / WebSocket API):
                                         BI tools, mobile apps, ERP sync
```

## WASM modules

A module is a WASM binary plus a manifest:

```yaml
# plugins/dew_point/plugin.yaml
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
  separation extends to plugins by construction
  ([read-write-separation.md](read-write-separation.md)).
- Derived tags produced by modules are first-class tags: they carry quality
  (worst-of-inputs by default), appear in the historian, can have alarms.
- Notification-channel modules receive alarm events and return outbound
  payloads; the host — not the module — performs delivery, so a plugin
  cannot exfiltrate data beyond its declared channel.
- Module updates deploy without recompiling or restarting the host
  (`iron plugin add ./dew_point.wasm`, hot-swapped between scan cycles).
- The same module binary MUST run unmodified inside `iron test --sim` —
  plugins are testable without hardware, like everything else.

## Satellite processes

For anything too heavy or too stateful for a sandbox — a custom protocol
driver, an ML model, a bridge to a legacy system — write a normal program in
any language and connect it to NATS with its own credentials:

```
satellite "vibration-ml" (Python):
  credentials: subscribe data.plant.line1.vibration.>
               publish   data.plant.line1.vibration_health.>
  → reads raw spectra, publishes a health-score derived tag
```

- Satellites authenticate like edge agents: per-identity keys, subject-scoped
  permissions, revocable individually ([security.md](security.md)). A
  satellite never receives `cmd.>` publish rights.
- A satellite that publishes tags MUST follow the tag value contract
  (value/quality/timestamp — [tag-model.md](tag-model.md)); `iron validate`
  checks declared satellite tags like any others.
- Custom *protocol drivers* are satellites by default (any language, crash in
  isolation); drivers graduate into iron-core (Rust) when they prove general
  demand — the satellite stage is the ecosystem's proving ground.

## REST / WebSocket API

The boring, stable layer for external consumers: read tags and history, query
alarms, subscribe to live updates, submit commands (full WRITE-path semantics:
auth, RBAC, audit — [command-path.md](command-path.md)). Versioned, documented
OpenAPI; the same API the UI itself uses, so it cannot rot.

## What is deliberately NOT extensible

- Elixir internals (GenServer layout, PubSub topics) — private, may change
  any release.
- The Command Service pipeline — no plugin hooks before or after
  authorization/audit. Extending the WRITE path means proposing a change to
  IRON itself, in the open.
- The alarm engine's state machine — plugins enrich and notify; they cannot
  suppress or acknowledge.

## Distribution

`iron plugin add <path|url>` installs into the project (vendored, committed
to Git like all configuration — a plant's deployment never depends on a
registry being online). A community index comes later; vendoring is the
mechanism either way.
