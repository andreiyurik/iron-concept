---
status: accepted
type: decision
---

# ADR 0004 — TimescaleDB as the Single Database

## Context

A SCADA platform needs configuration storage, a time-series historian, alarm
and audit history, and user/RBAC data. The traditional answer is three systems
(relational + time-series + cache) with three failure modes and continuous
synchronization.

## Decision

One TimescaleDB instance holds everything. TimescaleDB *is* PostgreSQL — every
tool, every ORM, every DBA skill, and 30 years of documentation apply.
Hypertables, native compression, and continuous aggregates cover the historian
workload ([specs/historian.md](../specs/historian.md)); cross-domain JOINs
("show alarms for tags configured by user X last week") are single queries.
Current-value reads are served from an ETS cache in Elixir, not from the
database.

## Alternatives

- **InfluxDB** — purpose-built TSDB, but a second engine, a second query
  language, license churn (v2/v3), and no relational home for config/RBAC.
- **ClickHouse** — superb analytics at scales IRON doesn't target; weak fit
  for transactional config and upsert-style dedup.
- **QuestDB / VictoriaMetrics** — same "second system" objection; VM is a
  metrics store, not a general database.
- **Plain PostgreSQL with partitioning** — viable fallback! TimescaleDB adds
  compression and continuous aggregates for free; if licensing of the
  community extension ever becomes a concern, the schema degrades gracefully
  to vanilla partitioned Postgres.

## Trade-offs

Single-node Postgres ceilings apply (HA via Patroni, not horizontal sharding)
— sufficient for 100k-tag plants, honest limit for "all sites in one DB"
enterprise scenarios, which the multi-site tier addresses differently.
The Timescale license (TSL for some features) needs tracking in packaging.

## Revisit when

A production deployment actually saturates a well-tuned single node, or TSL
licensing conflicts with distribution requirements.
