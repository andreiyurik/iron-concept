---
status: draft
type: vision
---

# Honest Comparison

A comparison that only beats up strawmen is marketing, not analysis. Here is
where IRON actually stands against the systems people really use — including
where those systems are better today and will remain better for years.
(Reminder: IRON is a concept; everything here compares *designs*, and every
IRON capability is a target until a prototype exists.)

## Ignition (Inductive Automation)

The strongest modern SCADA platform and the proof that the market rewards
breaking with legacy: unlimited tags per server license, web-deployed clients,
runs on Linux. Credit where due — and accuracy where due:

- Since 8.0, Ignition project resources are **JSON files on disk, designed for
  Git**; 8.3 extended config-as-code further. The old "binary blob" criticism
  is obsolete and IRON will not repeat it.
- Ignition Edge and the free Maker Edition run on a Raspberry Pi.

Where IRON differs by design, not by claim of superiority:

| | Ignition | IRON (target) |
|---|---|---|
| License | Proprietary, ~$10k+ per server, modules priced separately | Apache 2.0, $0 |
| Core workflow | Designer GUI first; files underneath | CLI and plain text first; GUI on top |
| Scripting | Jython 2.7 embedded throughout | Elixir/Rust extension points, YAML for config |
| Edge | Java gateway (Edge product) | Single Rust binary, SQLite buffer |
| Architecture | Monolithic gateway | Separated edge/broker/server, READ/WRITE split |
| Commissioning | Not addressed | [`iron field`](../specs/field-verification.md) |
| Who owns the deployment | Vendor relationship required | Client owns everything |

Honest bottom line: a funded plant choosing between a mature, supported
Ignition and a young IRON should choose Ignition today. IRON's bet is the long
bet Linux made against UNIX: open and composable compounds faster.

## The traditional proprietary incumbents

These are the names an automation engineer means by "SCADA vendor" — the
systems running most of the world's plants today. They are mature, supported,
certified, and installed at a scale IRON will not approach for years. That
context matters before any comparison: IRON's argument with them is about
*architecture and ownership*, never about maturity.

### Siemens WinCC (Unified)

The default in much of Europe and CIS industry, and deeply integrated with
SIMATIC PLCs. Siemens modernized hard with **WinCC Unified**: the runtime now
renders HTML5/SVG web clients and scripts in **JavaScript** (a V8 engine) —
a real generational leap from WinCC V7's VBScript, and IRON will not pretend
otherwise. Where it differs from IRON by design: engineering happens only in
**TIA Portal (Windows-only)**, runtime licensing is still **tag-count based
(PowerTags)**, the stack is proprietary, and it gravitates toward Siemens
hardware. IRON's pitch here is openness, config-in-Git, and no per-tag meter —
not a claim of matching WinCC's breadth or install base.

### Rockwell FactoryTalk View SE

The North American counterpart, and the tightest integration with
Allen-Bradley / Logix controllers — its real moat. Rockwell simplified
licensing in v13 (2022) to **unlimited displays**, and VBA scripting gained a
central repository (v12+). It remains **Windows-only**, proprietary, sold
through a **distributor network** (pricing never public), with per-client
access licensing. IRON differs on the same axes as with Siemens: open,
text-configured, self-owned — against a mature, deeply PLC-integrated, closed
platform.

### AVEVA / Wonderware (InTouch & System Platform)

The other global giant. AVEVA's **System Platform** gives a genuinely strong
object/asset model, and **InTouch Unlimited** (2023) removed the old per-tag
ceiling, with the **AVEVA Flex** subscription (credit-based, cloud + on-prem)
modernizing how it's bought. Credit where due. The design differences remain:
**Windows-only**, proprietary, QuickScript, and a subscription cost structure
that grows with the enterprise. IRON targets the opposite end — one CLI, two
containers, a $150 box — for the engineer and integrator who will never run an
enterprise license manager.

### Schneider Electric (EcoStruxure Geo SCADA · AVEVA Plant SCADA)

Schneider's SCADA shows up in two forms: **EcoStruxure Geo SCADA Expert**
(formerly ClearSCADA) — strong for telemetry and wide-area monitoring, with a
SQL Server backend and mature **store-and-forward** for RTUs over DNP3 — and
**AVEVA Plant SCADA** (formerly Citect, now under AVEVA after the
Schneider–AVEVA merger). Both are **Windows-based**, proprietary, and licensed
**per point / per node**. Geo SCADA's telemetry pedigree is real and IRON does
not claim to beat it there today; IRON's difference is again openness,
text-first configuration, and ownership without a per-point meter.

## United Manufacturing Hub

The closest open-source analog — UMH assembles a Unified Namespace stack
(Kafka/MQTT, Node-RED, Grafana, and notably the same TimescaleDB) for
manufacturing data. If your organization has a platform team that speaks
Kubernetes, UMH is a serious, more mature option today.

The difference is the audience and the unit of complexity:

- UMH: Helm charts, k3s, Kafka, microservices — built for enterprise IT teams.
- IRON: one CLI, two containers, a $150 mini-PC — built for the automation
  engineer and the 5-person integrator who will never run Kubernetes.

UMH validates IRON's architecture thesis (broker-centric UNS + TimescaleDB).
IRON's differentiation is the Rails move: radical reduction of operational
complexity, plus the OT-native features UMH lacks (alarm management discipline,
command path with audit, field verification).

## Node-RED

The current king of "automation that is actually fun" — and the tool most
likely already running in Bakyt's greenhouse. Enormous ecosystem, gentle
learning curve, genuinely democratic.

But flows are not specifications: logic lives in a JSON graph that resists
review, diff, and test; there is no tag model, no quality propagation, no
alarm lifecycle, no audit trail, no RBAC around writes. Node-RED is a
brilliant integration duct tape and a poor system of record. IRON aims at the
step after Node-RED: when the greenhouse becomes a business and "it stopped
working and nobody knows why" stops being acceptable. (And Node-RED remains a
fine companion for edge integrations IRON doesn't cover.)

## FUXA

An active open-source web SCADA (Node.js/Angular): visual editor, Modbus/OPC-UA,
genuinely useful for small HMI projects. Closest in spirit among open SCADA
tools. Differences: GPL vs Apache 2.0, no edge/store-and-forward architecture,
no historian strategy beyond basic logging, configuration through the GUI
rather than reviewable text. FUXA competes with IRON's Layer 1+2 visuals;
it does not attempt the platform scope (CLI, specs, alarm discipline, audit).

## Legacy open SCADA (ScadaBR, RapidSCADA, OpenSCADA)

Proof that "open source SCADA" alone is not enough. Inactive or
maintenance-mode communities, 2000s-era UIs, configuration in opaque
databases. Their lesson is IRON's founding thesis: developer experience is
not a feature you add later — it decides whether the project lives.
ScadaBR's orphaned Brazilian community is also, frankly, a recruitment
opportunity ([roadmap](../business/roadmap.md)).

## OpenPLC

Not a SCADA — the open-source IEC 61131-3 *runtime*, and the obvious
comparison for IRON's [long-term PLC exploration](../specs/plc-runtime.md).
Respected in education and security research; not positioned for production
process control (no online change, limited fieldbus, no deterministic-runtime
story). Detailed comparison lives in the PLC runtime spec.

## The full comparison

The complete landscape in one place: IRON against the proprietary incumbents
everyone knows *and* the modern/open stacks that share its architecture. Read
it with the legend in mind — and read the **last row first**.

**Legend:** ✅ shipping today · ⚠️ partial or with real caveats · ❌ no ·
🎯 IRON *design target* — a concept-stage intention, not existing software.

| Capability | IRON | Ignition | WinCC Unified | AVEVA | Schneider | FactoryTalk | UMH | Node-RED |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| **Cost & licensing** | | | | | | | | |
| Open source, self-hostable free | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ✅ |
| No per-tag / per-point / per-client meter | ✅ | ✅ | ❌ | ⚠️ | ❌ | ⚠️ | ✅ | ✅ |
| Free at single-device / hobby scale | 🎯 | ✅ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ✅ |
| **Platform** | | | | | | | | |
| Server runs on Linux | 🎯 | ✅ | ⚠️ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Runs on ARM / Raspberry Pi | 🎯 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| No Windows required anywhere (incl. engineering) | 🎯 | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Architecture & workflow** | | | | | | | | |
| Config as reviewable text in Git | 🎯 | ✅ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ⚠️ |
| CLI-first, no mandatory GUI | 🎯 | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ |
| Simulation-first as a core workflow | 🎯 | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ |
| Edge store-and-forward | 🎯 | ✅ | ❌ | ✅ | ✅ | ⚠️ | ✅ | ❌ |
| Broker-centric / Unified Namespace | 🎯 | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ |
| READ/WRITE separation as architecture | 🎯 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Audited command path (authn+authz+log on every write) | 🎯 | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ❌ |
| **OT features** | | | | | | | | |
| ISA-18.2-grade alarm management | 🎯 | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Historian with full / open SQL | 🎯 | ✅ | ⚠️ | ⚠️ | ✅ | ⚠️ | ✅ | ❌ |
| Modern web UI, no browser plugins | 🎯 | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ |
| Field verification / commissioning workflow | 🎯 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Ownership & maturity** | | | | | | | | |
| You own it — no vendor / distributor gatekeeper | ✅ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Production-proven & supported today** | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

How to read this table honestly:

- **The IRON column is almost entirely 🎯.** Those are intentions backed by a
  detailed design, not features you can run. The only ✅ IRON earns today come
  from its *license* (Apache 2.0, no meter) and the fact that you own it — not
  from working software.
- **The last row is ❌ for IRON on purpose.** Every incumbent here is proven,
  supported, and certified; IRON is a concept. If maturity is your deciding
  factor, every other column wins today. That is the honest state of things.
- **Two rows are IRON-alone by design:** READ/WRITE separation as architecture,
  and field verification as a product. Those are the rows worth building first —
  the capabilities no incumbent offers, not the ones where IRON merely catches
  up.
- The incumbents are strong where the table can't show it: install base,
  hardware integration, decades of edge cases handled, training ecosystems, and
  regulatory track record. None of that is copied quickly, and IRON's plan
  respects it — the [roadmap](../business/roadmap.md) competes on architecture
  and developer experience, not on out-maturing a thirty-year platform.
