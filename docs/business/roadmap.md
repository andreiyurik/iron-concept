---
status: draft
type: business
---

# Roadmap

How IRON gets from a documented concept to a platform used in production by
factories, farms, and integrators. Phases overlap; milestones — not calendar
dates — define completion.

This is a hard project. The intersection of Rust, Phoenix, and industrial
automation expertise is genuinely rare, and solo execution to maturity is
nearly impossible. The plan below is about creating the conditions where the
right people find the project — plus one structural advantage most open
source projects lack: [IndustrialPROFI](model.md), a revenue-generating
product in the same market, funds development time, supplies the warmest pilot
leads, and carries years of domain credibility.

## The honest challenge

```
Rust developer      — knows memory safety and async; has never seen a Modbus frame
Phoenix developer   — knows LiveView and OTP; has never been on a factory floor
Automation engineer — knows S7 and commissioning; does not know Git or Rust
```

People with all three are nearly nonexistent. You do not find that team — you
build in public until it assembles itself. The sequence every successful
project followed: publish → someone comments → opens an issue → first PR →
contributor → maintainer → co-founder.

## Phase 0 — Foundation (now)

**Goal:** make IRON findable by the right people before writing code. People
who engage with specs before code exists are the highest-quality future
contributors.

- [x] Concept documentation published (this knowledge base)
- [ ] README that stops a senior Rust/Elixir developer mid-scroll
- [ ] GitHub Discussions enabled — one thread per spec
- [ ] Reserve `getiron.dev`, `@getiron` handles
- [ ] First-contribution-in-30-minutes path in CONTRIBUTING.md
- [ ] NLnet Foundation grant application (€5–50k; best fit, lowest effort —
      apply first)

Promotion that works: a specific technical question on Elixir Forum
("Building real-time SCADA in LiveView — architecture feedback wanted"),
r/rust + r/PLC for both halves of the audience, direct outreach to
maintainers of `tokio-modbus`, `opcua`, `nerves` ("read your work on X,
would value your view on this decision" — never "please contribute").

**Done when:** 50+ stars, 3+ substantive spec discussions, 1 person genuinely
interested in contributing.

## Phase 1 — Working Prototype (months 1–6)

**Goal:** prove the idea end-to-end. Nothing recruits like a working demo.

The vertical slice, deliberately narrow:

- `iron new` → project scaffold → `iron dev` → LiveView dashboard with
  simulated data ([the five-minute target](../specs/cli.md))
- Modbus TCP only (diagslave/real PLC), deadband + quality on the edge
- One threshold alarm → Telegram notification
- Runs on a Raspberry Pi 4; deploys via Kamal to any Linux box
- `iron validate` for the implemented subset
- **The scaling benchmark — retire the core risk early.** The server
  architecture rests on one hypothesis: GenServer-per-tag + LiveView fan-out
  handles plant scale on modest hardware. Measure it as soon as the vertical
  slice exists: 100k simulated tags, 50 concurrent LiveView sessions, on a
  fixed reference machine — publish latency, memory, and methodology in the
  repo. This converts the [architecture targets](../specs/architecture.md)
  into facts (or redesigns the weak spot while it is still cheap), and the
  write-up doubles as the Phase 1 deep-dive that recruits Elixir contributors

Explicitly deferred: OPC-UA, S7, mimics, editor, RBAC depth, PLC runtime.
Protocol edge cases are the top schedule risk; one protocol done well beats
four half-done.

Contributor on-ramp: good-first-issues scoped to 2–4 hours, zero OT knowledge
required ("deadband filter for f32", "sparkline LiveView component"),
every PR answered within 48 hours.

**Done when:** the demo is filmed and reproducible in under 10 minutes; 3–5
external contributors merged; 500+ stars; Show HN posted (only after the
demo works — HN rewards running code, not RFCs); NLnet submitted.

## Phase 2 — First Real Deployment (months 6–12)

**Goal:** production at one real plant. A single production installation is
worth more than 10,000 stars — it surfaces what no simulation reveals and
becomes the reference case that shortens every later sale.

- OPC-UA client (S7-1200/1500, Beckhoff)
- Historian trends (continuous aggregates), alarm ack + escalation, basic RBAC
- Self-hosted installation guide: under 30 minutes on commodity hardware
- `iron lite` — the single-artifact, zero-services mode for greenhouses and
  workshops ([deployment Mode 0](../specs/deployment.md)): the Level 1
  audience gets a real on-ramp, and `iron migrate --to-full` keeps it honest
- First production plant: an IndustrialPROFI client first (trusted
  relationship, controlled failure modes), CIS market second, Brazil's
  orphaned ScadaBR community third

Co-founder search turns active in this phase (technical: senior Rust/Elixir
with shipped production systems — domain knowledge transfers; business: B2B
industrial sales). Values alignment over skills; vesting with a cliff is
non-negotiable.

Grants: Sovereign Tech Fund (€100k+, needs production users — this phase
unlocks it), Rust Foundation (when iron-core crates publish to crates.io).

**Done when:** 1 production installation with a written case study, 2–3
regular contributors, first revenue signal, 2,000+ stars.

## Phase 3 — Community and Enterprise (months 12–24)

- S7 driver (the CIS installed base), SVG mimic editor, and the extension
  surface ([spec](../specs/extensions.md), [ADR 0008](../decisions/0008-extensions-beyond-elixir.md)):
  WASM plugins + NATS satellites, so the ecosystem grows in any language —
  community drivers prove themselves as satellites, then graduate into core
- Enterprise tier launch: SSO, multi-site, compliance reporting — boundary
  per [model.md](model.md), defined publicly before the first sale
- PLC runtime exploration begins, strictly per
  [specs/plc-runtime.md](../specs/plc-runtime.md): WASM target upstreamed to
  `plc-lang/rusty`, `iron test --sim` proof of concept
- Community milestones: first community-contributed protocol driver, first
  integrator publicly building on IRON, monthly community call
- Non-cash sponsorships: Hetzner OSS program (CI + demo infra), hardware
  vendors (Advantech, Kunbus — co-marketing, reference hardware), Elixir
  ecosystem endorsements

**Done when:** 5,000+ stars, 10+ contributors, first enterprise contract,
monthly recurring revenue exists, docs site live.

## Phase 4 — Scale (years 2–3)

Complete protocol suite (DNP3, BACnet, PROFINET, EtherNet/IP), managed cloud
historian, mobile operator app, IEC 62443 documentation package, formal
integrator certification, i18n (PT, ES, DE, KK, UZ, HI). Case study per
vertical: agriculture, food & beverage, water treatment, light manufacturing,
HVAC.

The inflection metric: 50+ integrator partners deploying without core-team
involvement.

**Done when:** 50+ production installations, 100+ lifetime contributors,
$100k+ ARR funding a core team of 3–5.

## Funding strategy

IndustrialPROFI revenue is the base: a defined monthly allocation of
development time, independent of grant cycles. Grants accelerate; they do not
gate.

| Source | Amount | When |
|---|---|---|
| NLnet Foundation | €5–50k | Phase 0/1 — apply first |
| Rust Foundation | $10–20k | Phase 1/2 — when crates publish |
| Sovereign Tech Fund | €100k–1M | Phase 2 — needs production users |
| Prototype Fund / NGI | varies | Phase 2/3 — needs EU partner |

**No VC before ~$1M ARR.** Venture capital solves "scaling faster than
revenue allows"; before that problem exists it creates board pressure, exit
pressure, and growth dynamics that conflict with open source health.

## Build in public

The mechanism by which contributors find the project:

- **Weekly devlog** (GitHub Discussions): what was built, what failed, one
  decision with its trade-offs. 300–500 words, unpolished. The developer who
  reads that you spent three days on a Modbus framing edge case trusts you
  more than any landing page.
- **Monthly deep-dive** (Elixir Forum / dev.to): one specific problem, one
  specific solution. "Backpressure between a Rust poller and Phoenix PubSub"
  works; "IRON v0.1 released" does not — nobody cares yet.
- **Frame problems to attract engineers:** not "we need help with drivers"
  but "OPC-UA session reconnect fails like this, here is what we tried" —
  engineers cannot resist a well-framed problem.
- **Respond to everything within 24–48h** in Phases 0–1. The contributor who
  gets a thoughtful same-day reply comes back; the one who waits three weeks
  does not.

## Risk register

| Risk | P | Impact | Mitigation |
|---|---|---|---|
| Solo burnout before co-founder found | High | Critical | IndustrialPROFI income stability; sustainable pace; co-founder by Phase 2 |
| Protocol edge cases consume all time | High | Medium | v0.1 = Modbus TCP only |
| No external contributors by Phase 1 end | Med | High | Aggressive build-in-public; tiny first issues; direct outreach |
| First production install fails publicly | Med | High | Pilot with a trusted IndustrialPROFI client first |
| Funded competitor copies the idea | Med | Medium | Community moat; OT depth is not copyable quickly |
| Key contributor leaves | Med | High | Two people per critical subsystem by Phase 2 |
| Grant rejections | Med | Medium | Parallel applications; bootstrap fallback |
| Apache 2.0 hostile fork | Low | Medium | Niche + community moat ([ADR 0005](../decisions/0005-apache-2-license.md)) |

## Open questions

- **Legal entity:** where to incorporate (KZ / NL Stichting / EE e-Residency)
  — needed before the first grant payment.
- **Co-founder equity:** defensible ranges 40–50% pre-revenue, 10–25% after
  first enterprise contract; 1-year cliff non-negotiable; document before any
  conversation.
- **Governance:** BDFL is right for Phases 0–2; revisit at ~20 regular
  contributors.
- **CLA:** decide before the contributor base makes dual-licensing
  impossible ([ADR 0005](../decisions/0005-apache-2-license.md)).
