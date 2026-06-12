---
status: draft
type: spec
depends-on: [tag-model, alarm-engine, read-write-separation]
---

# Edge Agent (iron-core)

The runtime layer. Runs on the factory floor — on the same DIN-rail hardware as
the PLC, on a Raspberry Pi in the cabinet, or on an x86 mini-PC. It is not a
relay. Everything between the wire and the message broker is its responsibility.

The choice of Rust for this component is a safety requirement, not a
preference: [decisions/0001-rust-for-edge.md](../decisions/0001-rust-for-edge.md).

## Responsibilities

1. Poll field devices according to `config/tags/` (protocol drivers)
2. Convert raw values to engineering units, assign quality and timestamp
3. Filter by deadband, publish to NATS data subjects
4. **Evaluate first-level alarms locally** (see below)
5. Buffer everything when the network is down; replay when it returns
6. Execute authorized commands received on its command subjects
7. Serve a local read-only fallback page for operators

## Deadband filtering

A temperature sitting at 87.3°C ± 0.05°C generates no traffic at
deadband = 0.5°C. For typical analog tags, 80–90% of polls produce no publish.
Digital tags publish only on change.

Normative behavior:

- The agent MUST publish when `|current − last_published| >= deadband`.
- The agent MUST publish on any quality change, regardless of deadband.
- The agent MUST publish when the value crosses any alarm limit defined for
  the tag, regardless of deadband. A value creeping from 179.8 to 180.2 in
  0.3-steps with deadband 0.5 still crosses the HIGH limit at 180.0 — the
  crossing publishes immediately. **Deadband must never delay or mask an alarm.**
- The agent MUST publish a heartbeat sample at a configurable maximum interval
  (default 60s) even when nothing changed, so downstream consumers can
  distinguish "stable value" from "dead link".

```rust
// The filter knows about quality and alarm limits — not just the delta
pub fn should_publish(&self, current: Sample) -> bool {
    let value_moved   = (current.value - self.last_published.value).abs() >= self.deadband;
    let quality_moved = current.quality != self.last_published.quality;
    let limit_crossed = self.alarm_limits.iter()
        .any(|l| l.crossed_between(self.last_published.value, current.value));
    let heartbeat_due = self.since_last_publish() >= self.max_publish_interval;
    value_moved || quality_moved || limit_crossed || heartbeat_due
}
```

## Data quality

Every sample carries `GOOD / UNCERTAIN / BAD`. Downstream consumers never guess
whether the sensor is alive.

| Transition | Trigger |
|---|---|
| → `UNCERTAIN` | Raw value out of declared raw range (broken wire, sensor fault); stale data (1 missed scan) |
| → `BAD` | Read timeout (3 consecutive missed scans, configurable); protocol exception; device unreachable |
| → `GOOD` | Successful read with in-range raw value |

Quality transitions always publish (deadband does not apply).

## First-level alarm evaluation

The agent — not the server — evaluates alarm limits, because the agent is the
component guaranteed to be alive next to the process:

- The agent MUST evaluate every alarm limit defined in `config/tags/` on every
  scan, before deadband filtering.
- Alarm state transitions (ACTIVE / RTN) MUST be published as discrete alarm
  events on dedicated subjects (`data.<site>.<area>.alarms.>`), buffered with
  the same durability as samples.
- During a network outage the agent MUST continue evaluating alarms and MUST
  be able to deliver critical notifications through a configurable local
  channel (GSM modem, local relay/sounder output, SMTP on the OT network) —
  so a plant that loses its server does not lose its alarms.

What the agent does **not** do: acknowledgment, escalation, shelving,
notification routing — that is server-side aggregation. The split is specified
in [alarm-engine.md](alarm-engine.md). And the agent never *acts* on an alarm:
automatic protective reactions belong in the PLC.

## Store-and-forward buffer

When the network goes down, the agent writes samples and alarm events to a
local SQLite buffer. When the network returns, it replays in order.

- Replayed samples carry their original acquisition timestamps; the historian
  MUST accept out-of-order inserts ([historian.md](historian.md)).
- Buffer size is bounded by disk watermark; on overflow the agent drops the
  oldest *samples* first and alarm events last, and records a gap marker so
  the historian shows a documented gap instead of silently missing data.
- NATS JetStream gives at-least-once delivery, so duplicates are possible.
  Sample identity is `(tag, timestamp)`; consumers MUST deduplicate on it.

## Autonomous operation

The factory floor keeps working when the server is unreachable:

- Polling continues, buffering continues, local alarm evaluation continues.
- The agent serves a **local read-only fallback page** (plain HTML over HTTP on
  the OT network): current values, qualities, and active alarms for its tags.
  When iron-web is down, the operator is degraded — not blind. This page is
  READ-only by construction: the fallback server has no code path to the
  command executor.
- Command execution requires the server (commands originate only from the
  Command Service). Loss of server means loss of *remote* control — local
  control at the PLC/panel level is unaffected, which is exactly the
  degradation order a plant expects.

## Command execution

The agent subscribes to its own command subjects only (see
[read-write-separation.md](read-write-separation.md)). For each received
command it MUST:

1. Validate the command against `config/commands/` (unknown command → reject + NACK)
2. Execute the protocol write
3. Read back the target where the protocol allows, and publish an ACK/NACK
   result event with the observed value

The full command lifecycle is specified in [command-path.md](command-path.md).

## WASM modules

Custom logic — unit conversions, derived tags, custom protocol framing — can be
deployed as WASM modules without recompiling the agent. Modules run sandboxed:
they receive samples and return samples, with no I/O capabilities of their own.
Manifest format, capability rules, and the full extension surface (server-side
modules, NATS satellites): [extensions.md](extensions.md).

## Time

- The agent assigns timestamps at acquisition, in UTC.
- The agent SHOULD sync via NTP; on hardware with no reliable clock it MUST
  mark samples so the historian can flag suspect timestamps.
- Where the protocol provides device timestamps (OPC-UA), both device and
  agent timestamps are preserved.
