---
status: draft
type: vision
---

# Beliefs

Rails has a concept: software should be optimized for programmer happiness.
Industrial automation has never had this. It has been optimized for
certification, for vendor margins, for backwards compatibility with decisions
made in 1995.

IRON is built around a different set of beliefs. They are heretical in the OT
world. We think they are obviously correct.

**Your configuration belongs in Git.**
Not in a binary file on one machine. Not in a database only the vendor's tool
can read. In a text file, in a repository, with history, authors, and diffs.
When something breaks at 3am, you want to know what changed last Tuesday and
who changed it.

**Windows is not required.**
It never was. It was a choice the industry made in 1995 and never revisited.
IRON runs on a Raspberry Pi, a $150 mini-PC, or any Linux server.

**Per-tag licensing is exploitation.**
You pay more as your factory grows. The software becomes a tax on operational
visibility. IRON has unlimited tags. Growth is never punished.

**A $150 computer is enough.**
Reliability comes from redundancy architecture, not from a price tag. Two $150
mini-PCs with automatic failover beat one $15,000 server with one power supply.
The idea that monitoring requires expensive certified hardware is a story
vendors tell to protect margins. (Where certified hardware *is* warranted —
safety systems — IRON doesn't play; see
[what IRON does not claim](../specs/security.md).)

**You should be able to demo before the client signs.**
`iron new myplant` + `iron dev` + a laptop: a working dashboard with simulated
data in five minutes. If you cannot show something working before the
contract, you are selling faith, not software.

**The automation engineer and the developer deserve equal respect.**
The engineer who has configured Siemens S7 for 20 years is not less valuable
than the developer who knows Rust. IRON serves both, and neither should have
to learn the other's tools to do their job:

```
Level 1 — Mouse only         visual dashboard builder, no files, no code
Level 2 — YAML               tags, alarms, objects as text in Git
Level 3 — Elixir + Rust      custom widgets, drivers, logic
```

These are not separate modes — they are layers of one system. A Level 1 user
sees the dashboard a Level 3 developer built; a Level 2 user's YAML is what
the Level 1 user's screen shows.

**The simulator is not optional.**
A developer should never need physical access to a PLC to build a screen, test
alarm logic, or train an operator. Simulation-first development cuts
commissioning surprises and makes the field accessible to people who have
never stood on a factory floor.

**Open source is more trustworthy than a black box.**
A proprietary SCADA you cannot inspect is not more secure because it is
certified. When the vendor raises prices or goes bankrupt, the factory is
hostage. IRON is Apache 2.0: read it, audit it, fork it, run it forever.

**Happiness lives in the tooling, not in the guarantees.**
This is where the Rails analogy stops. A Rails deploy failure is an exception
page; an OT failure is physical. IRON optimizes for joy in the development
loop — the CLI, the simulator, the five-minute first dashboard — and for
boring, conservative, explicit behavior in the runtime: quality flags,
buffered delivery, audited commands, alarms that never depend on the happy
path. Fun to build with. Dull and predictable in production. Both, deliberately.

---

These are not features. They are the beliefs the architecture is built on.
If they resonate — you are in the right place. Start at
[START-HERE.md](../START-HERE.md).
