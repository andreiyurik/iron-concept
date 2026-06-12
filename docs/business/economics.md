---
status: draft
type: business
---

# Economic Impact

> This page makes the buyer's case. It deliberately avoids the inflated
> claims this industry is famous for — an experienced plant manager smells
> "+10% OEE from a dashboard" instantly, and so should you.

## What a SCADA layer can honestly claim

**OEE** (Overall Equipment Effectiveness = Availability × Performance ×
Quality) is the metric plants live by. World average sits around 55–65%;
world class is 85%+. At a $10–15M/year plant, one OEE point is roughly
$80–150k of nearly pure margin, because the equipment is already paid for.

The honest causal chain: **visibility does not improve OEE by itself.**
People acting on visibility do. What a modern monitoring layer actually
delivers:

| Mechanism | Honest effect |
|---|---|
| Downtime is *measured* instead of guessed | The #1 prerequisite: most plants discover their real loss profile in the first month and it never matches the logbook |
| Anomalies seen as trends, not threshold breaches | Hours-to-days earlier intervention on degrading equipment |
| Historian answers "what happened at 14:23" in seconds | Investigations drop from days to minutes; recurring problems actually get fixed |
| Alarms reach the right person ([escalation](../specs/alarm-engine.md)) | Night incidents don't wait for the morning shift |
| Data survives crashes ([store-and-forward](../specs/edge-agent.md)) | No more "the system restarted and the data is gone" |

Industry literature attributes single-digit OEE-point improvements to
monitoring-driven programs — *when management acts on the data*. IRON's claim
is narrower and stronger: it removes the cost barrier to having the data at
all.

## The cost side — where the case is unambiguous

```
Traditional SCADA deployment (mid-size plant):
  Licenses (per-tag/per-client):        $15,000–80,000
  Vendor-specified hardware:            $5,000–15,000
  Certified integrator, 3–6 months:     the largest line item
  Annual license renewal:               recurring

IRON deployment (target):
  Licenses:                             $0
  Hardware (commodity, incl. HA pair):  $500–5,000
  Integrator, weeks not months:         spec-driven setup + iron field
  Annual:                               optional support contract
```

Even granting zero OEE improvement, replacing a $60k license renewal with a
$0 license and a local support contract pays for the migration. Every
operational improvement is upside on top.

## The people effect

- **Operators** — see the plant on a screen instead of walking rounds with a
  logbook: fewer transcription errors, faster response.
- **Instrumentation engineers** — the historian remembers what the pressure
  did three weeks ago; investigations stop being archaeology.
- **Plant owners** — real-time visibility from a phone, not Monday's report
  about Saturday's failure.
- **Workers** — planned maintenance instead of chaotic stops means stable
  shifts and fewer emergency callouts.

## Why CIS markets first

- **Low competition:** Ignition has minimal presence in Kazakhstan/Uzbekistan;
  the installed base is aging Siemens/Schneider and legacy Russian SCADA.
- **Data sovereignty:** self-hosted open source answers a question cloud-first
  Western vendors cannot.
- **Price sensitivity:** at a fraction of incumbent cost, the conversation
  starts from "why not pilot it".
- **Local moat:** CIS industrial domain knowledge + modern stack + European
  documentation standards is a genuinely rare combination.

## Regulatory tailwind

NIS2 (EU, in force since October 2024) and IEC 62443 turn OT cybersecurity
from a virtue into a budget line. A system with Git-based change management,
documented security architecture ([specs/security.md](../specs/security.md)),
and a standard patch path is demonstrably easier to bring through an audit
than a proprietary install untouched since 2019. This matters in the EU and
for every CIS exporter with European partners.
