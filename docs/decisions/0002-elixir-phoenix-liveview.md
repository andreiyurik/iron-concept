---
status: accepted
type: decision
---

# ADR 0002 — Elixir / Phoenix LiveView for the Server

## Context

Every SCADA UI problem is a real-time state synchronization problem: a tag
updates → every watching screen must update; a client reconnects → it must get
current state immediately; thousands of tags fan out to dozens of operator
screens continuously, for years, without restarts.

## Decision

iron-web is Elixir/Phoenix. Each tag is a lightweight GenServer (~2KB);
supervision trees give per-tag fault isolation and automatic recovery; Phoenix
PubSub routes updates to subscribers only. LiveView renders ~95% of the UI:
WebSocket lifecycle, reconnection, and diff-based DOM updates are handled by
the framework, so application code is Elixir — not hand-rolled JavaScript
event plumbing. React appears only inside the design-time SVG editor
([specs/visual-system.md](../specs/visual-system.md)).

## Alternatives

- **Full React SPA + API backend** (the Ignition Perspective shape) — two
  applications, hand-managed real-time sync, doubles the required skill set,
  and contradicts "the engineer and the developer are equal": LiveView
  templates are far more approachable for non-frontend people.
- **Go/Rust backend + JS frontend** — same objections, plus losing OTP's
  supervision model, which maps unreasonably well onto "10,000 independent
  tag processes that must never take each other down".

## Trade-offs

Elixir is a niche hire — the single largest strategic risk in the stack. Two
deliberate mitigations: the public extension surface does not require Elixir
([ADR 0008](0008-extensions-beyond-elixir.md) — WASM plugins and NATS
satellites in any language, Elixir internals private), and the
GenServer-per-tag scaling hypothesis is retired by a published benchmark in
Phase 1, not assumed ([roadmap](../business/roadmap.md)). Further: LiveView
keeps per-client state on the server (memory scales with connected operators —
fine for plants, wrong for consumer scale); operator screens depend on server
availability — mitigated by the edge agent's local fallback page
([specs/edge-agent.md](../specs/edge-agent.md)) and HA deployment
([specs/deployment.md](../specs/deployment.md)).

## Revisit when

A real deployment shows LiveView latency or state cost failing on plant-scale
operator counts — the React-island escape hatch generalizes if needed.
