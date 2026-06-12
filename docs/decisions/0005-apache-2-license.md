---
status: accepted
type: decision
---

# ADR 0005 — Apache 2.0 License

## Context

The license decides who can adopt IRON, who fears it legally, and whether the
open-core business model works. The realistic fears: a large company forking
and competing; enterprise legal departments blocking adoption; patent claims
in a litigious industry.

## Decision

Apache 2.0 for everything in the core. Farmers and SMEs use IRON forever,
free. Integrators build commercial services without restriction. Enterprises
adopt without legal review friction. Every contribution carries an explicit,
irrevocable patent grant — which matters in industrial automation, where
patent litigation is a real risk.

## Alternatives

- **MIT** — simpler, but no patent grant. For this industry that clause is
  worth the extra license text.
- **AGPL v3** — the right choice if the primary fear is a cloud vendor
  strip-mining the project. For IRON it is not: managed SCADA is not an AWS
  product category, the enterprise value lives in support/SLA/multi-site
  features rather than the runtime, and AGPL legal policies block adoption in
  exactly the markets IRON targets first. Friction where we need adoption,
  protection where we face no threat.
- **Dual licensing (Apache + commercial)** — viable future path; requires a
  CLA from every contributor, a legal entity, and a sales process. Pure
  overhead at concept stage. The CLA question should be settled *before* the
  contributor base grows large — tracked as an open question in the
  [roadmap](../business/roadmap.md).

## Trade-offs

A well-funded competitor can legally fork and out-execute us. Accepted: the
moat is community velocity and OT domain depth, not the license. Every
successful open-core infrastructure company at scale (Grafana — Apache 2.0
historically, GitLab — MIT core) made the same bet.

## Revisit when

A funded fork offers a hosted IRON without contributing back, or enterprise
revenue passes the point (~$500k ARR) where a commercial license tier
justifies CLA overhead.
