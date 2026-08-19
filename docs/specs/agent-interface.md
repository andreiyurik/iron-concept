---
status: draft
type: spec
depends-on: [cli, tag-model, command-path, read-write-separation]
---

# Agent Interface

How AI agents — and scripts, CI, and IDE tooling — interact with IRON. The
rationale is [ADR 0011](../decisions/0011-ai-native-surface.md); the thesis
is [vision/spec-driven.md](../vision/spec-driven.md): agents generate specs,
`iron validate` decides deterministically, humans authorize. This document
is the contract that makes the thesis a product surface.

```
                 ┌──────────────────────────────────────────┐
  human ──CLI──► │  iron-core library API                    │
                 │  validate · explain · generate · test ·   │
  agent ──MCP──► │  diff · new                               │ ──► plant manifest
                 └──────────────────────────────────────────┘       (derived, never hand-written)
                        same code · same result schema · same diagnostics
```

> **v1 scope — deliberately small.** The first release ships exactly this:
> `AGENTS.md` in every scaffold, `--json` on every read command, and
> `iron agent mcp` with **three tools** — `validate`, `explain`, `generate`.
> Everything else on this page is the design those three grow into; the
> rows marked *deferred* live in [business/deferred.md](../business/deferred.md)
> with their triggers.

## One core, two interfaces

Every operation exists once, as a function in the `iron-core` library. The
`iron` CLI and the `iron agent mcp` server are thin wrappers over it.

| Operation | CLI | MCP tool | Writes | v1 |
|---|---|---|---|---|
| Validate specs | `iron validate --json` | `validate` | — | ✅ |
| Where does X go and why; effective config; one area; the manifest | `iron explain <ref> \| --effective \| --area <scope> \| --manifest` | `explain` | — | ✅ |
| Derive artifacts from specs | `iron generate … --json` | `generate` | project files | ✅ |
| Scaffold from a template | `iron new … --json` | `new` | project files | later |
| Simulation scenarios | `iron test --sim --json` | `test` | — | later |
| Git ↔ running drift | `iron diff --json` | `diff` | — | later |

Normative rules:

- **Same result, same shape.** `--json` output and MCP tool results are the
  same JSON document. Exit codes follow [cli.md](cli.md) (0 / 1 / 2); MCP
  returns the same status in the result body.
- **Parity.** The CLI command table and the MCP tool list come from one
  registry in `iron-core`, so an operation cannot drift between interfaces.
  A CI test asserting this is *deferred* until the fourth tool exists; with
  three, eyes suffice.
- **Read-only by default.** Tools that write do so only inside the project
  directory (`config/`, `test/`, generated files) — never to a PLC, never to
  a running deployment. There is **no MCP tool that submits a command**, runs
  `iron field`, or deploys. Those stay human-initiated
  ([command-path.md](command-path.md)).
- MCP transport is stdio (`iron agent mcp`) for local agents; anything
  networked is out of scope until a real need appears.

## Diagnostics teach

A diagnostic is the unit of communication between IRON and whoever — or
whatever — is fixing the problem. One format everywhere: CLI, MCP, the
`iron dev` browser overlay.

```json
{
  "code": "IRON-E042",
  "severity": "error",
  "location": { "file": "config/tags/reactor_01.yaml", "path": "reactor_01.pressure.alarms.high_high.limit", "line": 23 },
  "message": "alarm high_high (220) exceeds range maximum (200)",
  "hint": "Either raise range to [0, 250] if the sensor supports it, or lower the limit. Alarm limits MUST lie inside range.",
  "docs": "specs/alarm-engine.md#limits"
}
```

Normative rules:

- Every diagnostic MUST carry a stable `code`, a `location` precise enough to
  open in an editor, a `message` stating what is wrong, and a `hint` stating
  what to do about it.
- Codes are stable across releases; a code is never reused for a different
  condition. `docs` links to the normative sentence the rule comes from.
- Diagnostics are written to be pasted, unchanged, into an agent's context as
  the next instruction. Validator output is the fix prompt.
- `iron validate --explain IRON-E042` prints the rule, the rationale, and an
  example — the same text the skill file contains.
- A machine-applicable `fix` field is *deferred*
  ([deferred.md](../business/deferred.md)).

## Context packs

An agent working on `reactor_01` should not read the whole plant. The plant
hierarchy is the agent's context-window boundary.

```bash
iron explain --area reactor_01
# → tags (effective, after device-type expansion)
#   alarms, commands (READ and WRITE specs, clearly separated)
#   dashboards and mimics that reference these tags
#   device types in use
#   plugins reading these tags
#   nothing about line_2
```

Normative rules:

- A context pack is derived from the plant manifest; it is never
  hand-maintained.
- A pack MUST NOT include internals of neighbouring areas. Cross-area
  references appear as names only.
- `--format json|markdown`; Markdown is the default because it is what agents
  read best and humans can read at all.
- A size budget with hierarchical splitting is *deferred* until a real site
  needs it ([deferred.md](../business/deferred.md)); v1 packs are whatever the
  area contains.

### What `iron new` puts in the project for agents

One file: `AGENTS.md` — conventions, the validate rule, where things live, and
the IRON dialects (the YAML schema, Svelte 5 runes for widgets). The template
a project was scaffolded from is its reference material: a template is a
complete, validated, simulated site, so the examples are already in
`config/`. `iron agent skill` prints the same conventions for tools that keep
instructions elsewhere. Nothing in `AGENTS.md` is runtime configuration — it
is documentation that happens to be machine-shaped, versioned with the CLI
that generated it.

## One manifest: intent vs. fact

```
intent   config/**/*.yaml in Git        what the plant should run
         │
         ▼  iron-core: parse → expand device types → resolve refs → graph
manifest machine-readable, derived      tags · alarms · commands · dashboards ·
         (iron explain --manifest)      mimics · plugins · targets
         │
         ├─ iron validate           manifest ↔ invariants
         ├─ iron explain <ref>      manifest ↔ one question
         ├─ iron explain --area     manifest ↔ one area
         ├─ iron generate           manifest → artifacts
         └─ iron diff               manifest ↔ running config on a target   = fact
```

Normative rules:

- The manifest is never hand-written. The only inputs are the spec files.
  Publishing its schema as versioned JSON Schema is *deferred* until an
  external tool reads it ([deferred.md](../business/deferred.md)).
- `validate`, `explain`, `generate`, `diff`, and MCP all read the manifest —
  not the YAML directly — so every tool sees the same effective configuration
  ([device-types.md](device-types.md) expansion happens once).
- `iron explain <ref>` answers, for a tag: source and scan rate, effective
  deadband, alarms and priorities, which dashboards/mimics show it, which
  plugins consume it, historian retention; for a dashboard: every bound
  tag and command; for a command: who may issue it, the feedback tag, the
  timeout, the journal location. Output is the manifest subgraph, not prose —
  prose is `--human`.
- Fact is the running configuration stamped on every target by `iron deploy`;
  `iron diff` compares it to the manifest at Git HEAD ([cli.md](cli.md)).

## No LLM in the runtime

- IRON MUST NOT call, embed, bundle, or depend on a language model. Agents run
  in the user's tooling — any IDE, any provider, or none.
- Model-generated content reaches the plant only through `iron validate`.
  There is no "AI mode" that relaxes a check.
- An LLM is never on the WRITE path. No agent surface submits commands,
  acknowledges alarms, or authorizes field steps; the Command Service and
  `iron field` have no machine-facing entry point that skips a human.
- Third parties may build an assistant as a plugin or an external tool
  over this interface ([extensions.md](extensions.md)). IRON does not ship one.

## The parity metric (deferred to the end of Phase 1)

Per release, two numbers, published in the repo:

| Task | Actor | Allowed inputs | Measured |
|---|---|---|---|
| Add a plant area (tags, alarms, one dashboard) that passes `iron validate` and `iron test --sim` | an agent | MCP + `AGENTS.md` + `iron explain --area` | attempts, wall time, diagnostics hit |
| Same task | a new engineer | quickstart + docs | wall time, questions asked |

> **Status: target.** The metric is defined so that the AI-readiness claim is
> falsifiable. No measurement exists yet; the first one is the entry
> criterion for Phase 2 ([roadmap](../business/roadmap.md),
> [deferred.md](../business/deferred.md)).

If agents fail despite context packs and the skill file, the diagnosis is
"too many concepts", and the fix is fewer conventions — not better prompts.

## Bootstrap loop

IRON's own development uses this surface: contributors are expected to work
with agents pointed at `AGENTS.md` and the MCP server of the current build.
A missing or wrong diagnostic is found by the people writing IRON before any
integrator sees it. This is why `iron agent mcp` ships in Phase 1, not last —
and why it ships with three tools, not ten.
