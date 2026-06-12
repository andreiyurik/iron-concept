---
status: draft
type: spec
depends-on: [tag-model, edge-agent]
---

# Historian

One database for everything: configuration, time-series, alarm history, users,
permissions, audit. TimescaleDB — PostgreSQL with a time-series extension.
Rationale for the choice: [decisions/0004-timescaledb.md](../decisions/0004-timescaledb.md).

```
Traditional approach:              IRON:
  PostgreSQL  (config)        →     TimescaleDB (one database)
  InfluxDB    (time-series)   →     + ETS cache in Elixir for current values
  Redis       (current state) →
3 failure points, 3 sync paths     1 failure point, 0 sync paths
```

## Schema (core)

```sql
-- Hypertable, partitioned by time
CREATE TABLE samples (
  tag_id      integer       NOT NULL REFERENCES tags(id),
  ts          timestamptz   NOT NULL,   -- edge acquisition time, UTC
  value       double precision,
  quality     smallint      NOT NULL,   -- 0 GOOD / 1 UNCERTAIN / 2 BAD
  PRIMARY KEY (tag_id, ts)              -- dedup key for at-least-once delivery
);
SELECT create_hypertable('samples', 'ts');
```

- `(tag_id, ts)` is the identity: replayed/duplicate deliveries from JetStream
  upsert idempotently.
- Inserts MUST accept out-of-order timestamps — buffer replay after a network
  outage arrives late by design ([edge-agent.md](edge-agent.md)).
- Alarm events and command audit records are separate hypertables with the
  same conventions.

## Report-by-exception semantics

Deadband filtering means the historian stores *changes*, not a sample per scan.
This has consequences every consumer must respect:

- **Reading a value at time T** = last observation carried forward (LOCF): the
  most recent sample at or before T. "No row at 14:23" does not mean "no data"
  — it means "unchanged since the previous row".
- **Trend rendering** MUST draw stepped/LOCF interpolation between sparse
  points, not linear interpolation through gaps.
- **A real gap** (edge offline and buffer overflowed, or quality BAD) is
  distinguishable: gap markers from the edge agent and quality transitions are
  stored as rows. Trends MUST render documented gaps visibly, not paper over
  them.
- **Aggregates** (avg/min/max) over sparse data MUST be time-weighted, not
  row-weighted: a value that held for 50 minutes counts 50× more than one that
  held for one minute.

The heartbeat publish (max interval, default 60s) bounds how stale LOCF can be.

## Continuous aggregates

Pre-computed rollups make long trends cheap:

| Zoom level | Source | Typical points |
|---|---|---|
| Last 1 hour | Raw hypertable | ≤3,600 |
| Last 24 hours | 1-minute aggregate | 1,440 |
| Last 7 days | 5-minute aggregate | 2,016 |
| Last 30 days | 1-hour aggregate | 720 |
| Last 1 year | 1-day aggregate | 365 |

Aggregate rows store time-weighted avg, min, max, and last, plus a
quality-degraded flag so rollups never hide that the underlying hour contained
BAD data.

## Compression and retention

> **Design targets, not measurements.** TimescaleDB documents 8–12×
> compression on typical time-series workloads and millisecond-range queries
> on continuous aggregates. IRON will publish its own measured numbers when a
> prototype exists.

- Raw chunks compress after a configurable age (default 7 days).
- Retention is per-policy: raw data N months, 1-minute aggregates longer,
  daily aggregates indefinitely. Alarm and audit history follow their own
  (typically longer, sometimes regulatory) retention.

```yaml
# config/historian.yaml
retention:
  raw: 90d
  aggregates_1m: 2y
  aggregates_1d: forever
  alarms: 5y
  audit: forever
```

## Sizing reality check

10,000 tags at 1s scan rate is 315 billion *polls* per year — but after
deadband filtering the historian sees roughly a tenth of that, and compression
shrinks it further. A plant of this size fits a single mid-range server; this
is precisely the workload TimescaleDB is built for. The win over legacy
historians is not raw speed — it is that **the historian speaks SQL**: every
BI tool, every data scientist, every `psql` session can query it without a
vendor gateway.
