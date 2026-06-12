---
status: draft
type: vision
---

# Who IRON Serves

Four people, four entry points, one system.

## Arman — Automation Engineer

18 years at a food processing plant in Almaty. Knows Siemens S7 cold. Has run
Wonderware, WinCC, and MasterSCADA on three plants and knows exactly where each
one breaks.

**Problem:** SCADA license renewal is $60,000. The vendor requires a certified
integrator for any configuration change. The historian loses data when the
Windows service crashes every few weeks.

**IRON for Arman:** YAML configuration he owns, in Git. A CLI that validates
his tag definitions before they touch the plant. A historian built on
PostgreSQL — a technology with 30 years of documentation. No vendor to call
at 2am, because nothing about his system is a secret to him.
Entry level: 2 (YAML). He never has to learn Elixir.

## Zarina — System Integrator

Runs a 5-person automation consultancy in Tashkent: food factories, water
treatment, small manufacturers. Currently resells Ignition and Weintek HMIs.

**Problem:** License costs make her bids lose. Every project takes 3–4 months
because she configures the same things from scratch each time. The vendor owns
her client relationship.

**IRON for Zarina:** `iron new` during the first client meeting — live
dashboard before the coffee is finished. [Device types](../specs/device-types.md)
and [IO-list import](../specs/device-types.md) turn project setup from weeks
into days. [`iron field`](../specs/field-verification.md) turns commissioning
paperwork into a Git-versioned report. She sells implementation and support
instead of licenses — and her clients own their systems outright.
The integrator economics: [business/model.md](../business/model.md).

## Bakyt — Greenhouse Owner

2-hectare greenhouse operation near Bishkek, tomatoes year-round. 40 sensors:
temperature, humidity, CO₂, soil moisture. Logs everything manually, twice a
day. Budget: $2,000 total.

**IRON for Bakyt:** A Raspberry Pi with the edge agent, a dashboard on his
phone, a Telegram alert when the temperature drops at night. 40 sensors cost
the same as 4 — nothing. He doesn't know what Rust is and never needs to.
Entry level: 1 (mouse only), with a neighbor's son or a local integrator doing
the one-day setup. Hardware path: [guides/hardware.md](../guides/hardware.md),
Level 1.

## Nikita — Backend Developer

6 years of Go and Elixir in fintech. Has never been inside a factory. Sees
IRON on GitHub and realizes Elixir, NATS, and TimescaleDB are tools he already
knows — applied to a domain he doesn't.

**IRON for Nikita:** Real distributed-systems problems — at-least-once
delivery, store-and-forward, WebSocket fan-out, alarm state machines — with
physical consequences instead of another CRUD API. A codebase that uses
familiar tools in an unfamiliar domain, and a contribution path that starts
with a pure-Rust deadband filter and ends with production OT experience that
perhaps a few hundred developers in the world have.
Start at [CONTRIBUTING.md](../../CONTRIBUTING.md).
