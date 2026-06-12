---
status: draft
type: vision
---

# The Problem

Walk into any industrial plant today and you will find:

- A SCADA system running on Windows XP because the vendor's driver only
  supports it — and replacing the driver is a $40,000 upgrade project
- An HMI built with tools from 1998, with a UX that has not changed since
- No version control — configuration lives in binary files on a single
  machine; when that machine dies, so does the configuration history
- A historian that takes 3 minutes to answer "what was the temperature of
  reactor 3 last Tuesday at 14:23?"
- Proprietary scripting — VBScript in Wonderware, CimEdit in iFIX — skills
  that exist nowhere else in software
- Per-tag licensing: you pay more as your factory grows; the software is a tax
  on operational visibility
- A $200,000 invoice for software that does less than a modern web application
- Zero structural separation between reading sensor data and sending commands
  to machines

This is not a niche problem. This is the global standard. The world's factories
run on software that would be unacceptable in any other domain — and we accept
it because "that's how industrial software works."

**IRON rejects this premise.**

## Why it got this way

Honesty requires saying this part too: not all of the complexity is a scam.

Plants can kill people. Determinism, certification, and conservative change
management exist because of hard-won lessons, and the parts of the stack that
carry that weight — safety PLCs, SIL-rated interlocks, hardwired E-stops — are
not what IRON replaces ([what IRON does not claim](../specs/security.md)).

But the *supervisory* layer — visualization, historian, alarming, configuration
management — carries almost none of that safety weight, and yet it inherited
all of the cost, closedness, and 1990s architecture. That layer is overpriced
and overcomplicated not by physics but by vendor lock-in, per-tag rent
extraction, and two decades without competitive pressure. That layer is what
IRON rebuilds.

## Why existing alternatives didn't win

Open source SCADA exists (ScadaBR, RapidSCADA, OpenSCADA). It failed to win
not because the idea was wrong, but because developer experience was an
afterthought: manual setups, dated UIs, configuration in opaque databases,
documentation that assumes you already know the product.

Nobody built the Rails of industrial automation — the system where the right
thing is the easy thing, where `new project` to working dashboard is minutes,
and where the documentation makes you want to build something.

**IRON is that thing.** What that means concretely: [beliefs.md](beliefs.md),
and the honest comparison with current alternatives:
[honest-comparison.md](honest-comparison.md).

## The bottom of the stack has the same problem

Below SCADA sits the PLC runtime — CODESYS, TIA Portal, Studio 5000. CODESYS,
the de-facto "open" option, is closed source C with per-device licensing
(~€200–500) and binary project files Git cannot diff. It is not open in any
meaningful sense; it is just less closed than TIA Portal.

IRON's committed scope is SCADA. The long-term exploration of an open PLC
runtime — and an honest accounting of why that is much harder than it sounds —
lives in [specs/plc-runtime.md](../specs/plc-runtime.md).

## The window

- **NIS2** (EU, since October 2024) mandates OT cybersecurity — proprietary
  black boxes that haven't seen an update since 2019 are now a compliance
  liability
- **Data sovereignty** concerns favor self-hosted, auditable systems
- A generation of automation engineers is retiring, replaced by people who
  expect Git, a CLI, and documentation that doesn't require a training course
- The building blocks — Rust, Erlang/OTP, TimescaleDB, NATS — are mature and
  production-proven; the Unified Namespace movement has already evangelized
  the broker-centric pattern

The pieces exist. Someone needs to assemble them into one cohesive, open,
deployable system — with the developer experience as the foundation, not the
afterthought.
