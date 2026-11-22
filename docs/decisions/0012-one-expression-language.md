---
status: accepted
type: decision
---

# ADR 0012 — One Small Expression Language, Not a Scripting Language

## Context

IRON's configuration is declarative YAML, and declarative formats grow
computed values whether or not anyone plans for it. Three had already
appeared independently: arithmetic in device-type parameters
(`{base + 0x10}`), a condition in alarm suppression
(`suppressed_when: pump_01.running == false`), and derived tags — which the
specs sent to a separate plugin process even for `a - b`.

That last one broke the project's own rule. Widgets may not compute plant
values ([visual-system.md](../specs/visual-system.md)), so a temperature
difference had to be a tag; but making it a tag required writing, deploying,
and credentialing a program. The right thing was not the easy thing.

Left alone, each spec would have defined its own syntax, and IRON would have
repeated the best-known failure of YAML-configured systems: several
half-languages, none of them specified.

## Decision

One expression language, specified once in
[specs/expressions.md](../specs/expressions.md), implemented once in
`iron-domain`, and used in every spec that needs a computed value:
device-type expansion, derived tags, and alarm suppression.

It is deliberately not a programming language: two types (number, bool),
arithmetic, comparison, boolean logic, a fixed list of math functions, and
`if`. No loops, no user functions, no state, no time, no I/O. Every
expression is statically typed by `iron validate`, terminates, and cannot
panic. Quality propagates as worst-of-inputs.

Derived tags become a field on a tag (`derived:`), first-class in v1.

## Alternatives

- **Embed a scripting language** (Lua, Rhai, Starlark, JavaScript) —
  rejected. This is the Ignition/Jython path
  ([ADR 0008](0008-extensions-beyond-elixir.md)): the embedded language
  becomes the product's real API, cannot be type-checked against the tag
  model, can loop forever inside the scan path, and welds every site to one
  interpreter version.
- **Adopt CEL** — the closest existing fit and seriously considered.
  Rejected for v1: its type system (strings, lists, maps, timestamps, null)
  is mostly surface IRON would have to forbid, and the semantics that matter
  here — quality propagation and "no value" instead of an error — are not
  CEL's. The grammar is intentionally close to CEL's so the choice can be
  revisited.
- **Plugins for every computation** (the previous design) — rejected as the
  only mechanism: a process, credentials, and a deployment for a
  subtraction. Plugins remain the answer for anything that is a program.
- **Compute in widgets** — rejected by an existing rule: a value computed on
  a screen has no quality, no history, no alarms, and exists only on that
  screen.

## Trade-offs

- IRON now owns a grammar, a type checker, and an evaluator. Small, but it
  is a language, and languages attract feature requests forever. The spec
  lists what is excluded so that "no" has a place to point.
- No state means no rate-of-change, totalizer, or moving average in v1.
  Those are common in real plants and their absence will be felt; they are
  deferred with a trigger rather than added half-specified.
- Units are not checked. An author can declare a wrong `unit` on a derived
  tag and the validator will not notice.
- A user who needs slightly more than the language offers faces a step up
  to a plugin. The step is smaller than before, not gone.

## Revisit when

A second spec needs strings or collections in expressions — then CEL's
larger surface may earn its cost. Or the deferred stateful functions are
requested by more than one real site, which reopens the question of how
much language IRON wants to own.
