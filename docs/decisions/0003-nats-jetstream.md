---
status: accepted
type: decision
---

# ADR 0003 — NATS JetStream as the Message Bus

## Context

iron-core (Rust, OT side) and iron-web (Elixir, IT side) need a transport that
survives network partitions, replays missed data, runs on a $150 box, and is
operable by a 5-person integrator — not a platform team.

## Decision

NATS JetStream: single ~15MB binary with no dependencies, ~30MB RAM at rest,
at-least-once delivery with replay, subject hierarchy that mirrors plant
topology (`plant.line_1.reactor_01.temperature`) with wildcard subscriptions,
and per-identity subject-level authorization — which is what makes the
READ/WRITE transport separation enforceable
([specs/read-write-separation.md](../specs/read-write-separation.md)).
Subjects implement the Unified Namespace: one canonical address per tag.

## Alternatives

- **Kafka** — the UNS-at-enterprise default (UMH uses it). Rejected: JVM +
  ZooKeeper/KRaft operational weight is exactly what IRON's audience cannot
  carry; partition-key ordering and consumer-group semantics are overkill for
  tag streams.
- **MQTT (Mosquitto/HiveMQ)** — the OT-native choice and a supported *field
  protocol* for device ingest; rejected as the internal bus: QoS levels are a
  weaker durability story than JetStream streams, no replay-from-offset, and
  subject-level authz is less granular. Sparkplug B interop remains on the
  roadmap.
- **Phoenix PubSub alone** — no durability, couples the edge to the Elixir
  cluster.
- **Redis Streams** — another stateful service to operate, weaker story for
  edge buffering and authorization.

## Trade-offs

At-least-once means duplicates: consumers deduplicate on `(tag, timestamp)`
([specs/historian.md](../specs/historian.md)). JetStream throughput is far
below core-NATS marketing numbers — still orders of magnitude above plant
needs. One more moving part than "just Postgres LISTEN/NOTIFY"; accepted for
the buffering and replay semantics.

## Revisit when

Sparkplug B/MQTT interop demand grows enough that running both becomes the
default deployment — then the bridge design deserves its own ADR.
