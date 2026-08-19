---
status: accepted
type: decision
supersedes: 0002
---

# ADR 0009 — Rust for the Server (One Runtime Language)

## Context

[ADR 0002](0002-elixir-phoenix-liveview.md) chose Elixir/Phoenix for the
server because OTP supervision maps well onto "10,000 independent tag
processes" and LiveView delivers real-time UI without a JavaScript framework.
Both reasons were true and still are. Three pressures outweigh them now:

1. **IRON is built with AI agents, and so will its contributors' work be.**
   Of the languages in the stack, Elixir has by far the thinnest LLM training
   corpus; generation quality for Rust and TypeScript is materially higher.
   The spec-driven thesis ("AI generates YAML, IRON derives the rest" —
   [vision/spec-driven.md](../vision/spec-driven.md)) still needs someone to
   write the derivation — and that someone increasingly works with an agent.
2. **Two runtime languages duplicate the domain.** Deadband, quality
   propagation, and the ISA-18.2 alarm state machine are needed on the edge
   (first-level evaluation) *and* on the server (aggregation). Rust edge +
   BEAM server means writing and testing that logic twice, and the default
   single-binary deployment ([deployment.md](../specs/deployment.md)) becomes
   a Rust binary and a BEAM release bundled into "one artifact".
3. **Hiring.** ADR 0002 itself named Elixir "the single largest strategic
   risk in the stack"; ADR 0008 exists to route around it.

For the record: this is not "Phoenix is declining". Phoenix remains the most
admired web framework and Elixir a top-three admired language in the
[2025 Stack Overflow survey](https://survey.stackoverflow.co/2025/technology),
with small usage — a healthy niche. The decision is about IRON's constraints,
not Phoenix's health.

## Decision

The server is Rust, in the same Cargo workspace as the edge agent. One
language for the entire runtime; TypeScript only where a browser requires it
([ADR 0010](0010-svelte-for-the-ui.md)).

```
iron            one binary: CLI + server + simulator + MCP server (ADR 0011)
├── iron-domain pure logic — tag model, quality, deadband, ISA-18.2 alarm
│               state machine, command validation. No I/O, no async, no
│               framework. Compiles to native AND wasm32.
├── iron-core   the operations library: manifest derivation, validator,
│               explain, generate — the one registry that CLI, MCP, and a
│               future LSP wrap (ADR 0011)
├── iron-edge   protocol drivers, local buffer, first-level alarms
├── iron-server tag state, historian writer, alarm aggregation, Command
│               Service, REST, real-time fan-out (axum / tokio)
├── iron-cli    thin wrapper over the core API
├── iron-mcp    the same wrapper, second interface
└── iron-ui     Svelte assets, embedded in the binary
```

What replaces the OTP properties ADR 0002 relied on:

| OTP property | IRON in Rust |
|---|---|
| GenServer per tag | Sharded in-memory tag state + per-subscription broadcast channels. Not one actor per tag — a map entry per tag. |
| Supervision / restart | Process-level restart (systemd, Kamal health checks) + JetStream replay for catch-up ([ADR 0003](0003-nats-jetstream.md)). The server is designed to be killed and restarted at any time without data loss. |
| Per-process fault isolation | Panics isolated per connection/task; `iron-domain` is panic-free by construction (`Result` everywhere, fuzzed). A bad tag value cannot take down the server; a bad client disconnects only itself. |
| LiveView | Typed WebSocket protocol + Svelte client (ADR 0010). |
| Hot code upgrade | Zero-downtime deploy via Kamal ([ADR 0006](0006-kamal-for-deployment.md)). |

`iron-domain` is the load-bearing piece: it is what makes "the simulator is
the same binary" extend to "the server, the edge, the WASM modules, and the
browser-side simulation run the same crate". It is also the contributor
on-ramp with zero OT knowledge required.

## Alternatives

- **Keep Elixir/Phoenix** — technically excellent fit for the server alone;
  rejected for the three reasons above. The cost of two runtime languages is
  paid on every domain change, forever.
- **TypeScript/Bun server** (the "architecture-as-code framework" shape) —
  largest AI corpus and hiring pool, but a second runtime language again: no
  single-binary default, domain logic duplicated, and no upside over Rust
  once the edge is Rust anyway. TypeScript belongs in the browser.
- **Go** — rejected for the same reasons as in [ADR 0001](0001-rust-for-edge.md)
  (fieldbus ecosystem, FFI) plus the cost of a second language for no gain.
- **Rust with an actor framework (ractor/actix/kameo) to emulate OTP** —
  premature. May be adopted *inside* `iron-server` if profiling calls for it;
  invisible outside the crate either way.

## Trade-offs

- OTP supervision trees, hot upgrades, and the BEAM's decades of "never
  restart" operations are lost. Accepted: a supervisory SCADA server that can
  be killed and replayed is *more* boring, which is what
  [beliefs.md](../vision/beliefs.md) asks for in production.
- LiveView's "real-time UI without a JS framework" is lost; reconnect and
  state sync become IRON's code (hundreds of lines, not a framework — ADR 0010).
- The Elixir community was a planned recruiting channel
  ([roadmap](../business/roadmap.md)); Rust and TypeScript communities replace
  it. The "Rust & Elixir developers" persona becomes "Rust & TypeScript".
- Async Rust is harder for contributors than Elixir; compile times are real.
  Mitigated by keeping `iron-domain` synchronous and framework-free — most
  contributions land there.
- The scaling hypothesis changes shape. The Phase 1 benchmark becomes:
  100k simulated tags, 50 concurrent operator sessions, sharded state + WS
  fan-out, fixed reference hardware — **a target until measured**, per
  [architecture.md](../specs/architecture.md).

## Revisit when

Profiling shows the tag engine needs per-entity isolation or backpressure
that tokio tasks and channels cannot provide cheaply — then an actor library
inside `iron-server`, not a language change. Or: the Rust contributor pool
proves too narrow for the server — unlikely, since the edge was Rust already.
