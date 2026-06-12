---
status: accepted
type: decision
---

# ADR 0008 — Extensions Don't Require Elixir

## Context

Elixir is the technically ideal fit for the server core
([ADR 0002](0002-elixir-phoenix-liveview.md)) — and the stack's biggest
strategic risk. Elixir developers are rare globally and nearly absent in
IRON's first markets. If "Level 3: extend the system" means "write Elixir",
the integrator in Tashkent, the Python-speaking data engineer, and the
contributor from r/PLC are all locked out. A platform's ecosystem grows from
its extension surface; that surface must not require the core team's language.

The cautionary tale is Ignition: scripting was bound to embedded Jython, and
the entire ecosystem is now welded to Python 2.7 — a dead language version —
because the extension mechanism was an implementation detail that leaked.

## Decision

The public extension surface of IRON is **explicitly not Elixir**. Three
sanctioned mechanisms, specified normatively in
[specs/extensions.md](../specs/extensions.md):

1. **WASM modules** — sandboxed, language-agnostic plugins (write in Rust,
   Go, Python, JS, …) running inside iron-core and iron-web: derived tags,
   conversions, alarm enrichment, notification channels.
2. **Satellite processes over NATS** — out-of-process integrations in any
   language, holding their own scoped credentials: custom protocol drivers,
   analytics, ML inference.
3. **REST/WebSocket API** — for everything that lives outside the
   deployment.

Elixir module internals, GenServer layouts, and PubSub topics are **not** a
public API. The core stays small, maintained by people who chose Elixir; the
ecosystem grows in whatever language each contributor already knows.

## Alternatives

- **Elixir-only plugins** — maximal power, minimal audience; concentrates
  bus-factor risk in the rarest skill.
- **Embedded scripting (Lua, Python interpreter)** — the Ignition/Jython
  trap: pins the platform to one interpreter version forever, and embedded
  interpreters have a poor sandboxing story for OT.
- **"Just fork it"** — Apache 2.0 permits it, but forks fragment instead of
  compounding; an extension API exists precisely so people don't have to fork.

## Trade-offs

WASM toolchains (WASI, Component Model) are young: debugging is harder than
native code, host-interface design is real work, and there is a per-call
overhead (irrelevant at tag-update rates, measurable in tight loops). Two
in-process mechanisms (edge WASM + server WASM) plus satellites means three
documented contracts to keep stable. Accepted: this is the price of an
ecosystem that doesn't gate on a niche language — and the WASM bet compounds,
since the same modules run in `iron test --sim` sandboxes unchanged.

## Revisit when

The WASM Component Model stabilizes enough to replace today's simpler
WASI-style interface (migrate deliberately, once), or profiling shows a
plugin class that genuinely needs native in-process speed.
