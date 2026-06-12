# Architecture Decision Records

Every significant technology or design choice in IRON is recorded as an ADR:
what was decided, why, what was rejected, and what would change our mind.
ADRs are short and final-until-revisited; the living specifications they
support are in [../specs/](../specs/).

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-rust-for-edge.md) | Rust for the edge agent | Accepted |
| [0002](0002-elixir-phoenix-liveview.md) | Elixir / Phoenix LiveView for the server | Accepted |
| [0003](0003-nats-jetstream.md) | NATS JetStream as the message bus | Accepted |
| [0004](0004-timescaledb.md) | TimescaleDB as the single database | Accepted |
| [0005](0005-apache-2-license.md) | Apache 2.0 license | Accepted |
| [0006](0006-kamal-for-deployment.md) | Kamal 2 for deployment | Accepted |
| [0007](0007-codesys-today-iron-plc-later.md) | Integrate CODESYS today, explore own runtime later | Accepted |

## Proposing a decision

1. Open an Issue describing the problem in 3–5 sentences — surface objections
   before writing a thousand words.
2. Copy the template below to `docs/decisions/XXXX-short-title.md`.
3. Open a PR; discussion happens there. Acceptance requires maintainer
   approval and no unresolved blocking objections.

```markdown
---
status: proposed
type: decision
---

# ADR XXXX — Title

## Context        — the problem; why a decision is needed
## Decision       — what we choose, concretely
## Alternatives   — what was rejected and why
## Trade-offs     — what this costs us; if you can't name any, think harder
## Revisit when   — explicit conditions that reopen this decision
```
