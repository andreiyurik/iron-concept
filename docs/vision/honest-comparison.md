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

## Feature matrix

Kept honest: ✅ = exists today, 🎯 = IRON design target (nothing more).

| Capability | IRON | Ignition | UMH | Node-RED | FUXA |
|---|---|---|---|---|---|
| `new project` CLI → working dashboard | 🎯 | ❌ (installer + Designer) | ❌ (Helm) | partial | ❌ |
| Config as reviewable text in Git | 🎯 | ✅ (8.x file-based) | partial | partial (flow JSON) | ❌ |
| Runs on Raspberry Pi | 🎯 | ✅ (Edge/Maker) | ❌ (k3s heavy) | ✅ | ✅ |
| No per-tag licensing | 🎯 | ✅ (per-server instead) | ✅ | ✅ | ✅ |
| Built-in simulator as first-class workflow | 🎯 | partial | ❌ | ❌ | partial |
| Store-and-forward edge agent | 🎯 | ✅ (Edge) | ✅ | ❌ | ❌ |
| ISA-18.2-style alarm management | 🎯 | ✅ | ❌ | ❌ | partial |
| Audited command path, READ/WRITE separation | 🎯 | partial | ❌ | ❌ | ❌ |
| Field verification workflow | 🎯 | ❌ | ❌ | ❌ | ❌ |
| Open source | ✅ Apache 2.0 | ❌ | partial (open core) | ✅ | ✅ GPL |

The two rows where IRON is alone — audited command path as architecture, and
field verification as product — are the rows to build first.
