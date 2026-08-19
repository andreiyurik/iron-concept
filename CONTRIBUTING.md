# Contributing to IRON

IRON is in the **concept and architecture phase**.
There is no code yet. This is intentional.

Code written without architectural conviction becomes technical debt on day one.
The first goal is conviction — shared, tested, argued over.

Orientation: [docs/START-HERE.md](docs/START-HERE.md) maps the knowledge base.
The normative specifications live in [docs/specs/](docs/specs/); technology
choices are recorded in [docs/decisions/](docs/decisions/) — including how to
propose a new decision. Documentation conventions (one fact in one place,
frontmatter, claims-as-targets) are at the bottom of START-HERE.

---

## What we need most right now

### Domain expertise

If you have worked in industrial automation — PLC programming, process engineering,
system integration, OT security — your feedback is worth more than any pull request.

Tell us what we got wrong. Tell us what constraints we haven't considered.
Tell us what a real plant deployment looks like from your experience.

Open an Issue with label `domain-expertise`.

### Architectural challenges

Every decision in this architecture has trade-offs.
We would rather discover them in a GitHub discussion than after 50,000 lines of code.

Open an Issue with label `architecture` if you believe:
- A technology choice is wrong
- A principle breaks under a specific condition
- There is a simpler solution we missed

### Proof of concept components

If you want to validate a specific component:

| Component | Description |
|---|---|
| Rust Edge Agent | Modbus TCP reader → deadband filter → NATS publisher |
| Tag Engine | Rust sharded tag state + per-subscription WebSocket fan-out |
| Svelte Widgets | Gauge / trend / alarm panel against the widget contract, quality rendered |
| MCP Server | `iron validate` exposed as an MCP tool with CLI parity |
| Visual Editor | Drag-and-drop widget placement on a canvas |
| Simulator | Mock PLC generating realistic industrial data |
| TimescaleDB Schema | Hypertables + continuous aggregates + retention policies |

Open an Issue first to discuss scope. A working PoC for any one component is extremely valuable.

### The contribution ladder (once there is code)

Most people who can make IRON better will never write Rust — and should not
have to. Each rung is a real contribution; pick the one that matches what you
already know:

| Rung | You contribute | You need to know | Example first step |
|---|---|---|---|
| 1 | A **template** or **device type** (YAML) | your plant, not programming | the IO list of a greenhouse you run, as `iron new --template` material |
| 2 | A **symbol** for the mimic library (SVG with `data-iron-*`) | Inkscape / Figma | a pump that turns green when `running` is true |
| 3 | A **plugin** in any language (`iron new plugin --lang python`) | Python (or anything with a NATS client) | a dew-point calculator; a driver for a sensor we don't support |
| 4 | A **verified driver** (`iron test --driver` passes) | the protocol + the plugin above | your Modbus RTU plugin against diagslave, with the recording |
| 5 | A **core driver or domain change** in Rust | Rust, tests, the specs | graduating rung 4 into `iron-core` |

Rungs 1–3 need no OT background *and* no Rust; rung 5 needs both. The
ladder, the trust tiers, and the driver harness are specified in
[docs/specs/extensions.md](docs/specs/extensions.md).

### Evangelism

If this resonates with you — share it.
Industrial software people are not always on GitHub.
LinkedIn, industry forums, local automation communities.

The idea needs to reach the people who understand the problem firsthand.

---

## Principles for contribution

**Honest over optimistic.** If something is hard or unsolved, say so.
This architecture has real challenges. Document them, don't hide them.

**Simple over complete.** A deployed system beats a theoretically perfect one.
Resist adding components.

**Both audiences.** Every decision should consider both the automation engineer
who does not know Rust and the developer who does not know Modbus.

---

## Language

Documentation is English; issues and discussions in English or Russian are
welcome. When IRON has code, it will use English identifiers and comments.

---

## Code of conduct

Be respectful. Domain experts and software generalists often talk past each other.
The goal of IRON is to bridge that gap.

If you are a developer: the automation engineer has seen failure modes
that would never occur to you. Listen first.

If you are an automation engineer: the developer has practices
that will make your work more reliable and maintainable. Stay curious.
