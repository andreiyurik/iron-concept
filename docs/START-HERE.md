# Start Here

IRON is an open source concept for industrial automation built the way modern
software is built: configuration in Git, a CLI instead of a Windows wizard, a
simulator instead of mandatory hardware, and a five-minute path from zero to a
live dashboard. Status: **concept** — these documents are the blueprint;
the prototype comes next.

This knowledge base is plain Markdown with relative links. It reads perfectly
on GitHub and opens as an [Obsidian](https://obsidian.md) vault (open the
`docs/` folder) — same files, no plugins required.

## Pick your route

**🌱 "I have a greenhouse / workshop / small farm"**
Start with [the problem](vision/problem.md) — why this doesn't already exist
cheaply — then meet [Bakyt](vision/personas.md), whose situation is probably
yours. Then see what the hardware costs at
[Level 1 of the hardware guide](guides/hardware.md). Honest status: you can't
run IRON yet. Star the repo and watch; the first prototype targets exactly
your use case.

**🏭 "I'm an automation engineer or integrator"**
Read [beliefs](vision/beliefs.md) to see if we share values, then go straight
to the two things no vendor offers you:
[field verification as a product](specs/field-verification.md) and
[configuration drift detection](specs/cli.md). Then the
[honest comparison](vision/honest-comparison.md) — including where Ignition
beats us today. If you find an error in our domain assumptions, an issue from
you is worth more than a pull request:
[CONTRIBUTING.md](../CONTRIBUTING.md).

**💻 "I'm a developer (Rust / Elixir / curious)"**
[Architecture overview](specs/architecture.md) →
[READ/WRITE separation](specs/read-write-separation.md) →
[edge agent](specs/edge-agent.md) → the
[ADRs](decisions/README.md) for why each technology was chosen. The domain
context you're missing is in the [glossary](glossary.md), and
[Nikita's persona](vision/personas.md) explains what's in it for you.

**🤖 "I'm an LLM (or pointing one at this repo)"**
Normative truth lives in `specs/` — one concept per file, one fact in one
place. Read [glossary.md](glossary.md) first for terminology, then the spec
files relevant to your task; `vision/` and `business/` are narrative context,
not requirements. Frontmatter on every file declares `status`
(`draft`/`accepted`/`exploration` — nothing is `implemented` yet) and `type`
(`spec` is normative; everything else is not). Capability claims marked
"target" describe intent, not existing software. When generating IRON
configuration, the contract is: output must pass `iron validate`
([cli.md](specs/cli.md)).

## Map of the knowledge base

```
docs/
  START-HERE.md            ← you are here
  glossary.md              ← shared vocabulary (read me early)

  vision/                  ← WHY — narrative, for humans
    problem.md             what's broken in industrial software
    beliefs.md             the convictions the architecture is built on
    personas.md            Arman, Zarina, Bakyt, Nikita
    spec-driven.md         specs as source of truth; the AI workflow
    honest-comparison.md   vs Siemens, Rockwell, AVEVA, Schneider, Ignition, UMH, Node-RED

  specs/                   ← WHAT — normative, testable
    architecture.md        two layers, broker, scaling targets
    tag-model.md           the core abstraction
    device-types.md        templates + IO-list import
    read-write-separation.md   the sacred constraint
    edge-agent.md          deadband, quality, buffer, autonomy
    alarm-engine.md        ISA-18.2 lifecycle; alarms never act
    command-path.md        the only way IRON writes to the world
    security.md            zones, broker authz, what IRON does not claim
    historian.md           TimescaleDB, LOCF, retention
    cli.md                 contract of every iron command
    visual-system.md       widgets → SVG mimics → editor
    field-verification.md  iron field — commissioning as a product
    testing.md             unit / integration / simulation / field
    deployment.md          lite / dev / local / cloud / edge
    extensions.md          WASM plugins + satellites — extend in any language
    plc-runtime.md         long-term exploration (clearly gated)

  decisions/               ← WHY THIS TECH — one ADR per choice
  business/                ← model, economics, roadmap
  guides/                  ← hardware selection, TDD practice
```

## Reading orders

- **30 seconds:** [README](../README.md)
- **15 minutes:** [problem](vision/problem.md) → [beliefs](vision/beliefs.md)
  → [architecture](specs/architecture.md)
- **The full argument:** add [read-write-separation](specs/read-write-separation.md),
  [alarm-engine](specs/alarm-engine.md), [field-verification](specs/field-verification.md),
  [honest-comparison](vision/honest-comparison.md), [roadmap](business/roadmap.md)

## Conventions (for contributors)

- One concept per file; one fact lives in exactly one place — link, don't copy.
- Specs use MUST/SHOULD language and stay testable; narrative stays in `vision/`.
- Every file carries frontmatter: `status`, `type`, optional `depends-on`.
- Standard Markdown links only (GitHub-renderable); no Obsidian-only syntax.
- Performance numbers are either cited external benchmarks or explicitly
  labeled **targets** — never implied measurements of unwritten software.
