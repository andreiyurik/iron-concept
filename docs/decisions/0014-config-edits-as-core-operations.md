---
status: accepted
type: decision
---

# ADR 0014 — Configuration Edits Are Core Operations on a Lossless YAML Subset

## Context

`iron-core` could read configuration but nothing specified how
configuration is *changed* by a program. Yet three v1 features already
write spec files: `iron diff --pull`, `iron generate from-io-list`, and
`iron upgrade`. Each would have grown its own way of producing YAML.

The larger reason is the audience. Most automation engineers do not live in
a text editor. A form for an alarm limit, or clicking a pump on a mimic and
picking its tag, is how they expect to work — and both are deferred. If
editing is not a capability of the core, those features become a second
configuration system beside the files when they arrive.

Programmatic editing of YAML has one well-known failure: the rewritten file
loses its comments and formatting, the Git diff becomes unreadable, and
people stop trusting the tool with their files.

## Decision

1. Every change to configuration made by a program is an **edit operation
   in `iron-core`** — a small, closed set (set a value, add an entry, remove
   an entry). CLI commands, a future MCP tool, and future UI forms are
   facades over the same operations
   ([specs/config-lifecycle.md](../specs/config-lifecycle.md)).
2. Edit operations write files **losslessly**: everything they did not
   change stays byte-for-byte identical, comments included.
3. To make that tractable, IRON's configuration files are restricted to a
   **subset of YAML** that can be edited without loss. `iron validate`
   rejects files outside it.

No form, binding tool, or editor is built now. What is fixed now is that
they will have nothing to invent when they are.

## Alternatives

- **Regenerate whole files from the parsed model** — rejected. Simple, and
  it destroys comments and layout on the first edit. Unacceptable for files
  that people review in Git.
- **A database as the source of truth, files as an export** (the classical
  SCADA shape) — rejected; it reverses the project's central invariant.
- **Full YAML with a general lossless editor** — rejected. Anchors, merge
  keys, tags, and multi-document files make lossless edits a research
  project and add nothing a plant configuration needs.
- **A different format** (TOML, JSON, a custom DSL) — rejected for now.
  YAML is what the audience has seen and what language models write well;
  the subset removes the dangerous parts.
- **Defer until the first form is built** — rejected. `--pull` needs it in
  v1 anyway, and retrofitting "all edits go through the core" after three
  commands have written files their own way is the expensive order.

## Trade-offs

- IRON depends on a library for lossless reading and writing of its YAML
  subset. Checked before any code was written: two published crates,
  `yaml-edit` 0.3.6 and `oneyaml` 1.2.0, each reproduced every file of
  `examples/greenhouse` byte-for-byte and changed exactly one line when one
  alarm limit was set. That retires the feasibility risk; it does not choose
  between them. The choice is made in `iron-core` on maintenance history,
  licence, and behavior on `add` and `remove`, which the check did not
  exercise in depth.
- Users lose YAML features some of them like, anchors above all. Device
  types are the intended answer to repetition.
- The closed set of operations will feel small. Each addition is a new
  thing every facade must expose and test.

## Revisit when

No maintained library can edit the subset losslessly and writing one costs
more than the features it enables — then the subset shrinks further, or the
format question reopens. Or a real integrator needs a YAML feature outside
the subset for something device types cannot express.
