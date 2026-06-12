---
status: draft
type: business
---

# Business Model

## Open core

IRON is free for the vast majority of users. The core runtime, all protocol
drivers, the web UI, the historian, the alarm engine — everything needed to
run a real plant — is Apache 2.0
([why](../decisions/0005-apache-2-license.md)). No feature flags, no
telemetry, no crippled "community edition".

The paid tier exists for needs that only appear at enterprise scale, where the
features genuinely require ongoing infrastructure and support:

| | IRON (free) | IRON Enterprise |
|---|---|---|
| Core runtime, all drivers, unlimited tags | ✅ | ✅ |
| Web UI, historian, alarms, REST API | ✅ | ✅ |
| Self-hosted, Git-based config | ✅ | ✅ |
| Multi-site management | — | ✅ |
| SSO (SAML / OIDC / LDAP) | — | ✅ |
| Managed cloud historian | — | ✅ |
| SLA-backed support | — | ✅ |
| IEC 62443 / SOC 2 / ISO 27001 compliance reporting | — | ✅ |
| Dedicated onboarding, priority patches | — | ✅ |

This is the Grafana model. The boundary rule: if a small operator would miss
it, it belongs in free.

## Pricing philosophy

**No per-tag licensing. Ever.** A greenhouse with 40 tags and a refinery with
100,000 pay the same: nothing, for the core. Growth is never punished.

**Free means free.** A farmer runs IRON on a Raspberry Pi or a $50/month VPS
with no registration, no sales call, no credit card. The free tier is the full
product — it is also the marketing, the QA fleet, and the contributor pipeline.

**Enterprise pricing is value-based.** Plants evaluate against incumbent
deployments costing $150k+; an enterprise contract at $20–50k/year prices
against delivered value (see [economics.md](economics.md) for the honest
version of that argument), not cost-to-build.

## Revenue streams

1. **Enterprise subscriptions** — multi-site + cloud historian + SLA, for
   holdings running 5–50 plants.
2. **Support contracts** — tiered response SLAs for self-hosted enterprises
   that need a throat to choke at 3am.
3. **Integrator program** — certification, ecosystem directory, early access;
   IRON gets a distribution network without a sales team.
4. **Training and certification** — Git, YAML, and modern tooling taught
   through industrial use cases, for engineers crossing the bridge.

## The integrator is the real customer

SCADA is bought with accountability attached: someone local must answer when
it breaks. Integrators carry that liability — so IRON is designed to make
integrators structurally better off:

```
Ignition project:                     IRON project:
  License to client:  $40,000          License: $0
  Vendor's cut:       most of it       Support contract: integrator keeps it
  Client lock-in:     to the vendor    Client lock-in: none — client owns it
  Relationship owner: the vendor       Relationship owner: the integrator
```

Faster delivery compounds the margin story:
[device types + IO-list import](../specs/device-types.md) collapse project
setup; [`iron field`](../specs/field-verification.md) collapses commissioning
paperwork; [`iron diff`](../specs/cli.md) makes remote maintenance of many
client plants tractable for a 5-person shop.

## IndustrialPROFI — the structural advantage

[IndustrialPROFI](https://industrialprofi.com) is a workforce competency
platform for industrial enterprises, built by the same team and already in
revenue. **IRON manages equipment; IndustrialPROFI manages people.**

What it provides IRON: funding independent of grant cycles, warm pilot
customers who already trust the team, years of CIS-market domain credibility,
and an upsell path in both directions ("you trust us for competency
management — here is the equipment layer").

Integration points worth building once both exist: certification checks
before high-consequence commands (one API call: "is this operator's
qualification current?"), and incident correlation (equipment event × who was
on shift × their qualification) for root-cause and regulatory reporting.

## Target markets, in order

1. **CIS industrial SMEs** (Kazakhstan, Uzbekistan) — minimal Ignition
   presence, expensive Siemens incumbency, data-sovereignty preferences,
   existing relationships via IndustrialPROFI. Local expertise is a real moat.
2. **Greenfield projects** — no legacy to displace.
3. **System integrators globally** — Brazil (orphaned ScadaBR community),
   Poland/Czechia (mid-size manufacturers seeking non-German alternatives),
   India (agricultural monitoring boom).

Sequencing and milestones: [roadmap.md](roadmap.md).
