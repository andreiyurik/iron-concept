---
status: accepted
type: decision
---

# ADR 0011 — AI-Native Surface: One Core, Two Interfaces

## Context

[vision/spec-driven.md](../vision/spec-driven.md) states the thesis: AI
generates YAML, `iron validate` decides deterministically, humans authorize.
Today that is a description of a workflow, not a product surface — there is
no contract for *how* an agent talks to IRON, and "AI-friendly" could quietly
degrade into a chat box bolted onto the dashboard.

Two facts force a decision now. IRON is being built *with* agents, so it
will be the first consumer of its own AI-readiness. And the same agents will
be used by every integrator adopting IRON: the quality of their output is
bounded by the quality of the surface IRON gives them.

## Decision

Five rules, normatively specified in
[specs/agent-interface.md](../specs/agent-interface.md):

1. **One core, two interfaces.** Every operation lives once, as a function
   in the `iron-core` library API. The `iron` CLI and the `iron agent mcp`
   server are thin wrappers with full parity: `iron validate --json` and the
   MCP tool `validate` call the same code, return the same structured result,
   with the same diagnostics. MCP ships in Phase 1, not at the end — the
   bootstrap loop requires it — and it ships small: three tools.
2. **Diagnostics teach.** Every validator message carries a stable code, an
   exact location, what is wrong, and a concrete fix — and reads as a
   ready-made prompt for a fixing agent. One diagnostic format across CLI,
   MCP, and the `iron dev` UI.
3. **Context packs.** `iron explain --area <scope>` (and the MCP equivalent)
   emits a compact pack for one plant area — its tags, alarms, commands,
   dashboards, device types, and subscribers — without neighbouring areas.
   The plant hierarchy is the agent's context-window boundary. The scaffold
   ships an `AGENTS.md` with IRON conventions, so agents start from the right
   dialect.
4. **One manifest, intent vs. fact.** `validate`, `explain`, `diff`, and
   MCP all read a single machine-readable plant manifest
   derived from the specs — never hand-written. Intent is the YAML in Git;
   fact is the running configuration; `iron diff` is the comparison, and
   `iron explain <tag|dashboard|command>` answers "where does this go and why".
5. **No LLM in the runtime.** IRON never calls, embeds, or ships a model.
   Agents live in the user's tooling (any IDE, any provider). Model output
   reaches the plant only through `iron validate`, and an LLM is never on the
   WRITE path — [command-path.md](../specs/command-path.md) is unchanged.

And one measurement, per release from the end of Phase 1: an agent adds a
plant area using only MCP + `AGENTS.md`; a new engineer does the same task by
hand from the quickstart. Both numbers are published — the AI-parity claim is
a metric, not a slogan.

## Alternatives

- **An embedded copilot / chat in the UI** — rejected: provider lock-in,
  non-determinism inside an OT product, and it invites the honesty failure
  this repo forbids. Third parties may build one as a plugin.
- **MCP as a later plugin** — rejected: without parity from day one the CLI
  and the agent surface drift, and IRON cannot use itself during Phase 1.
- **Prompts and docs only** (the status quo) — insufficient: nothing is
  enforced, nothing is measured.

## Trade-offs

- Parity is a tax on every new command: library function, CLI flags, MCP
  schema. Accepted — it is also what keeps the CLI honest. The scope is kept
  small on purpose (three tools in v1; the rest in
  [business/deferred.md](../business/deferred.md)).
- MCP is young and still changing; the wrapper is deliberately thin so the
  core API, not the protocol, is the stable part.
- Context packs must stay small to be useful; a size budget becomes a rule
  when the first site needs one, and keeping it is ongoing work.
- "AI-native" invites marketing. Every claim here is a target until the
  parity metric is published; the metric itself is the guard.

## Revisit when

MCP is superseded by another agent protocol — swap the wrapper, keep the
core. Or the parity metric shows agents still fail despite packs and `AGENTS.md` —
then IRON's convention set is too large, and the fix is fewer concepts, not
better prompts.
