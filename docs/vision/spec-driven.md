---
status: draft
type: vision
---

# Spec-Driven Development

> *"Convention over configuration means AI has less to hallucinate."*

In traditional software, a specification is a document. Someone writes it,
someone else reads it, someone else implements it. Three people, three
interpretations, three chances for divergence.

In IRON, the specification is the configuration. The YAML you write to define
a tag is simultaneously:

- The source of truth for what the system should do
- The contract that `iron validate` enforces
- The input to code generation
- The basis for generated tests, simulation scenarios, dashboards, and field
  verification steps

**Write the spec, derive everything else from it.**

## One spec, many artifacts

From a single tag file ([format](../specs/tag-model.md)):

```
iron validate          → checks limits, ranges, sources, references
iron generate tests    → ExUnit stubs derived from the spec
iron generate scenarios→ simulation timelines from alarm definitions
iron dev               → dashboard widgets, trend charts, alarm panel
iron field             → verification protocol per signal type
```

No duplication. When the spec changes, the derived artifacts change with it.
Command contracts: [specs/cli.md](../specs/cli.md).

## Why this matters in the AI era

Large language models are excellent at generating structured text from natural
language. They are unreliable at generating correct imperative code for domains
they cannot verify.

```
Traditional approach:
  "Add a temperature sensor for reactor 01"
  → AI generates Elixir code, Rust structs, migrations, components
  → AI cannot verify correctness — hallucinations reach production

IRON approach:
  "Add a temperature sensor for reactor 01: 4-20mA, range 0-200°C,
   alarm at 180, critical at 195"
  → AI generates 20 lines of YAML
  → iron validate catches what the model got wrong, deterministically
  → iron generate derives the implementation from the verified spec
```

The AI produces what it is good at: structured data from natural language.
IRON derives what AI is unreliable at: correct Rust, Elixir, SQL. The problem
surface shrinks to one small YAML file checked by a deterministic validator.

Convention over configuration is the safety net: fewer fields means fewer
decisions for the AI, fewer decisions means fewer mistakes, and `iron validate`
catches the rest.

```bash
# The rule: nothing AI-generated reaches the plant floor without:
iron validate && iron generate tests && iron test --sim
```

## The payoff, concretely

A system integrator onboards a new plant:

```
Day 1:  AI generates config/tags/*.yaml for 200 tags from the IO list
        (or iron import io-list does it deterministically)
        iron validate → 12 errors found, all before touching a PLC
        fix, validate clean
        iron generate tests && iron generate scenarios
        iron test --sim → behavior matches spec
        iron deploy --target edge-01

Day 2:  iron field → 200 tags systematically verified, report in Git
```

No Excel. No paper checklists. No runtime surprises. This is what convention
over configuration means in the age of AI: not just a better developer
experience — a fundamentally more reliable path from specification to running
system.

## This repository follows its own rule

The documentation you are reading is structured the same way: normative,
testable specifications in [specs/](../specs/) (one concept per file, one fact
in one place, machine-checkable claims), separate from this narrative layer.
An LLM asked to implement IRON should be pointed at `specs/` and the
[glossary](../glossary.md) — see the conventions in
[START-HERE.md](../START-HERE.md).
