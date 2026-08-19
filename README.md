<div align="center">

# ⚙️ IRON

### Industrial automation for the rest of us

**Mission: plant-grade monitoring for everyone who was priced out of it** —
the greenhouse, the workshop, the small water plant, the factory tired of
license renewals. The incumbents had twenty years without competitive
pressure. IRON is the pressure.

**The goal, stated falsifiably: five minutes from `iron new` to a live
dashboard.** One open-source Rust binary on a $150 box — no Windows, no
vendor IDE, no per-tag meter. Unlimited tags, forever, Apache 2.0.

**And one guarantee no vendor designs for: a dashboard bug cannot command
a machine.** READ and WRITE are separate paths with separate credentials —
architecture, not code review. Specs are YAML in Git, validated
deterministically: built for AI agents, with no AI inside.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-concept_→_prototype-orange.svg)](docs/business/roadmap.md)
[![Docs](https://img.shields.io/badge/docs-knowledge_base-brightgreen.svg)](docs/START-HERE.md)
[![PRs Welcome](https://img.shields.io/badge/contributions-welcome-brightgreen.svg)](CONTRIBUTING.md)

[**Start Here**](docs/START-HERE.md) ·
[Architecture](docs/specs/architecture.md) ·
[Why IRON](docs/vision/problem.md) ·
[vs Siemens / Rockwell / AVEVA / Ignition](docs/vision/honest-comparison.md) ·
[Roadmap](docs/business/roadmap.md)

</div>

---

```bash
iron new myplant --template greenhouse && cd myplant
iron dev                      # live dashboard with simulated sensors — one binary, no Docker, no services
iron validate                 # spec errors caught before they reach the plant floor
iron deploy --target edge-01  # zero-downtime deploy to a $150 mini-PC in the cabinet
iron field                    # commissioning checklist as a product, results in Git
```

**Monitoring and control for factories, water treatment, greenhouses, and
workshops** — without six-figure licenses, per-tag pricing, Windows-only
runtimes, or a vendor between you and your own plant.

> 📐 **Where the project stands:** IRON is in its architecture phase — this
> repository is a complete engineering blueprint (16 normative specs, 11
> decision records, honest competitive analysis), published openly *before*
> the code, the way serious infrastructure gets built. The prototype
> (`iron new` → Modbus TCP → live dashboard) is the current milestone.
> **This is the best moment to influence the design** — and the earliest
> possible moment to say "I was here before v0.1". The first release is
> deliberately small; what waits, and what brings it back, is in
> [deferred.md](docs/business/deferred.md).

---

## Why this exists

Walk into a typical plant in 2026 and you will find a SCADA system on
Windows XP, configuration in binary files on one aging PC, a historian that
takes minutes to answer simple questions, and an invoice with more zeros than
features. Not because the problem is hard — because the incumbents haven't
felt competitive pressure in twenty years. [The full argument →](docs/vision/problem.md)

IRON applies the Ruby on Rails playbook to industrial automation: assemble
proven technology into one coherent, opinionated, joyful-to-use stack — and
make the right thing the easy thing.

```
PLCs & sensors ──► iron-edge ──────► NATS JetStream ──────► iron-server ──► any browser
Modbus·OPC-UA·S7   Rust edge agent   unified namespace      Rust · Svelte UI · alarms
                   deadband·buffer   at-least-once·replay   TimescaleDB historian
                   local alarms                             audited command path · MCP
```

**How one tag makes the round trip** — and why a dashboard can never command a
machine by accident:

![How IRON works: READ path and WRITE path](docs/assets/data-flow.svg)

The full layered stack — with the parts that ship first drawn solid and the
long-term exploration drawn dashed:

![IRON Architecture](docs/assets/architecture.svg)

## What makes it different

| | |
|---|---|
| 🔒 **READ/WRITE separation as architecture** | A dashboard bug *cannot* command a machine — separate code paths, separate broker permissions, separate network rules. [spec →](docs/specs/read-write-separation.md) |
| 🛰️ **Intelligence at the edge** | The Rust agent filters, buffers, and evaluates alarms next to the PLC — and keeps working when the network dies. [spec →](docs/specs/edge-agent.md) |
| 📋 **Specs, not click-marathons** | Tags, alarms, dashboards — plain YAML in Git. Reviewable, diffable, AI-generatable, deterministically validated. [spec →](docs/vision/spec-driven.md) |
| 🤖 **Built for agents — no AI inside** | Every CLI command is also an MCP tool over the same core; validator errors are written as fix prompts; context packs per plant area. The product never calls a model, and a model is never on the WRITE path. [spec →](docs/specs/agent-interface.md) |
| 🧪 **Simulation-first** | Build and test a full plant screen with zero hardware. The simulator is the same binary, not a toy. [spec →](docs/specs/testing.md) |
| ✅ **Commissioning as a product** | `iron field` turns signal checkout — today's Excel-and-paper ritual — into a guided, audited, Git-versioned workflow. **No other platform has this.** [spec →](docs/specs/field-verification.md) |
| 📉 **Drift detection** | `iron diff` proves the plant runs exactly what Git says. Remote fleet maintenance for integrators. [spec →](docs/specs/cli.md) |
| 🏷️ **Unlimited tags, forever** | Apache 2.0. A 40-sensor greenhouse and a 100,000-tag refinery pay the same: nothing. [ADR →](docs/decisions/0005-apache-2-license.md) |

## How IRON compares

The systems everyone knows, plus the modern stacks that share IRON's
architecture — honestly. **Read the last row first.**

**Legend:** ✅ shipping today · ⚠️ partial / caveats · ❌ no ·
🟢 IRON *design target* (concept stage, not built yet)

| Capability | IRON | Ignition | WinCC&nbsp;Unified | AVEVA | Schneider | FactoryTalk | UMH | Node-RED |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| Open source, self-hostable free | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ✅ |
| No per-tag / per-point / per-client meter | ✅ | ✅ | ❌ | ⚠️ | ❌ | ⚠️ | ✅ | ✅ |
| Runs on ARM / Raspberry Pi | 🟢 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Config as reviewable text in Git | 🟢 | ✅ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ⚠️ |
| CLI-first, no mandatory GUI | 🟢 | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ |
| READ/WRITE separation as architecture | 🟢 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Field verification / commissioning workflow | 🟢 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Open agent interface (MCP) with CLI parity | 🟢 | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ⚠️ |
| **Production-proven & supported today** | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

IRON's column is mostly 🟢 — *targets*, not shipping features — and the last
row is ❌ on purpose: every incumbent here is mature and IRON is a concept. The
bet is architecture and ownership, not out-maturing a thirty-year platform.
[Full 20-capability breakdown, with every vendor credited where it leads →](docs/vision/honest-comparison.md)

## Built on boring, proven technology

| Layer | Technology | Why |
|---|---|---|
| Edge agent | **Rust** | Memory safety next to PLCs, no GC pauses, single binary, ARM64/x86 — [ADR 0001](docs/decisions/0001-rust-for-edge.md) |
| Server | **Rust** (same workspace) | One runtime language; `iron-domain` logic written once for edge, server, WASM modules; one `iron` binary = CLI + server + simulator + MCP — [ADR 0009](docs/decisions/0009-rust-for-the-server.md) |
| Browser UI | **Svelte 5 + TypeScript** | Runes map onto tags, SVG mimics are native, small bundle embedded in the binary — [ADR 0010](docs/decisions/0010-svelte-for-the-ui.md) |
| Agent surface | **MCP with CLI parity** | Same core, same schemas, same diagnostics; no LLM in the runtime — [ADR 0011](docs/decisions/0011-ai-native-surface.md) |
| Message bus | **NATS JetStream** | 15MB binary, subject-level auth, replay, unified namespace — [ADR 0003](docs/decisions/0003-nats-jetstream.md) |
| Historian | **TimescaleDB** | It's PostgreSQL — full SQL, compression, continuous aggregates, one backup — [ADR 0004](docs/decisions/0004-timescaledb.md) |
| Deployment | **Kamal 2** | One command, health-gated, air-gap friendly, no Kubernetes required — [ADR 0006](docs/decisions/0006-kamal-for-deployment.md) |

Runs on a Raspberry Pi, a $150 fanless mini-PC, or your existing servers.
High availability = two cheap boxes + Patroni, not one expensive box.
[Hardware guide →](docs/guides/hardware.md)

## Works with your PLCs — three positions on the control layer

**Today:** IRON never asks a plant to replace a working controller — it
speaks Modbus, OPC-UA, and S7 to whatever already runs the machines: CODESYS,
TwinCAT, Siemens, CLICK
([ADR 0007](docs/decisions/0007-codesys-today-iron-plc-later.md)).

**Also today, fully open:** OpenPLC + IRON is an open-source stack from relay
to dashboard — control logic in OpenPLC on a Pi, monitoring in IRON, no
vendor anywhere. A niche for education, makers, and small sites — factories
stay on vendor PLCs, honestly.

**The long game, clearly gated as exploration and not a promise:** the
bottom of the stack deserves the same openness as the top. An open
IEC 61131-3 runtime in Rust — built on
[`plc-lang/rusty`](https://github.com/PLC-lang/rusty), starting with
Structured Text compiled to WASM so PLC programs get what they have never
had: **unit tests, simulation without hardware, and CI**.
[The exploration, honestly →](docs/specs/plc-runtime.md)

## Who it's for

- 🏭 **Automation engineers** drowning in license renewals and binary config files
- 🔧 **System integrators** who want to own their client relationships, not rent them from a vendor
- 🌱 **Farmers & makers** who deserve plant-grade monitoring at greenhouse prices
- 🦀 **Rust & Svelte developers** looking for distributed-systems problems with physical consequences

Meet [Arman, Zarina, Bakyt and Nikita →](docs/vision/personas.md)

## Honest by policy

This project's documentation rules forbid inflated claims: every performance
number is a cited benchmark or an explicit **target**, the
[competitive analysis](docs/vision/honest-comparison.md) credits Ignition and
UMH where they are better today, and the specs state plainly what IRON is
not (a safety system, for one — [see what IRON does not claim](docs/specs/security.md)).
If you find marketing where engineering should be, open an issue: that's a bug.

## 📚 The knowledge base

The `docs/` tree is a structured knowledge base — plain Markdown, also opens
as an Obsidian vault — written for two readers: **humans** deciding whether to
join, and **LLMs** implementing against normative specs.

```
docs/
├── START-HERE.md        routes: farmer · engineer · developer · LLM
├── glossary.md          shared vocabulary, one definition per term
├── vision/              why — problem, beliefs, personas, honest comparison
├── specs/               what — 16 normative, testable specifications
├── decisions/           why this tech — 11 ADRs with trade-offs
├── business/            model · economics · roadmap · deferred
└── guides/              hardware selection · TDD practice
```

## 🤝 Contributing

The most valuable contributions right now are **conversations, not code**:

- **Challenge the architecture** — find the flaw before it's 50,000 lines
- **Lend domain expertise** — tell us what the factory floor knows that we don't
- **Build a proof of concept** — any single spec, implemented and tested
- **⭐ Star the repo** — it's how the next contributor finds this

[CONTRIBUTING.md →](CONTRIBUTING.md)

## License

[Apache 2.0](LICENSE) — own it, fork it, deploy it, build a business on it.

---

<div align="center">

*The tools to build better industrial software finally exist.*
*Someone needs to put them together. If you share that belief — welcome.*

**[Start here →](docs/START-HERE.md)**

</div>
