---
status: draft
type: spec
depends-on: [tag-model, edge-agent, transport]
---

# Historian

One database for everything: time-series, alarm history, users, permissions,
audit. Two engines implement it: **SQLite** in the default single-binary
deployment and **TimescaleDB** — PostgreSQL with a time-series extension —
in plant mode ([deployment.md](deployment.md)). Rationale for TimescaleDB:
[decisions/0004-timescaledb.md](../decisions/0004-timescaledb.md).

## The storage contract

Everything in this document except the sections marked *TimescaleDB* is a
requirement on **both** engines. Application code is written against the
contract; `iron-server` has one storage interface with two implementations,
and the same test suite runs against each.

| Requirement | SQLite | TimescaleDB |
|---|---|---|
| Idempotent insert on `(tag, ts)`, out-of-order accepted | ✅ | ✅ |
| LOCF reads, documented gaps, time-weighted aggregates | ✅ computed by IRON | ✅ |
| Rollups for long trends | Rollup tables maintained by `iron-server` | Continuous aggregates |
| Retention | Deletion by age, run by `iron-server` | Retention policies |
| Compression | — | Native |

The cost is stated plainly: rollups, retention, and time-weighted math exist
twice, once in IRON's code for SQLite and once as TimescaleDB features. The
shared test suite is what keeps the two from disagreeing, and the ceiling of
the SQLite engine is a **target until measured**
([architecture.md](architecture.md)).

The diagram below describes plant mode.

```
Traditional approach:              IRON:
  PostgreSQL  (config)        →     TimescaleDB (one database)
  InfluxDB    (time-series)   →     + in-memory tag state in iron-server for current values
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

- `(tag_id, ts)` is the identity: replayed or duplicate deliveries upsert
  idempotently ([transport.md](transport.md)).
- Every tag type is stored as a double: `bool` as 0/1, and all integer types
  exactly, since none exceeds 32 bits. The tag's declared `type` is restored
  on read.
- Derived tags are stored like any other tag; the historian does not
  recompute them.
- Inserts MUST accept out-of-order timestamps — buffer replay after a network
  outage arrives late by design ([edge-agent.md](edge-agent.md)). `REQ-HIS-001 integration`
- Alarm events and command audit records are separate hypertables with the
  same conventions.

## Report-by-exception semantics

Deadband filtering means the historian stores *changes*, not a sample per scan.
This has consequences every consumer must respect:

- **Reading a value at time T** = last observation carried forward (LOCF): the
  most recent sample at or before T. "No row at 14:23" does not mean "no data"
  — it means "unchanged since the previous row".
- **Trend rendering** MUST draw stepped/LOCF interpolation between sparse
  points, not linear interpolation through gaps. `REQ-HIS-002 integration`
- **A real gap** (edge offline and buffer overflowed, or quality BAD) is
  distinguishable: gap markers from the edge agent and quality transitions are
  stored as rows. Trends MUST render documented gaps visibly, not paper over
  them. `REQ-HIS-003 integration`
- **Aggregates** (avg/min/max) over sparse data MUST be time-weighted, not
  row-weighted: a value that held for 50 minutes counts 50× more than one that
  held for one minute. `REQ-HIS-004 integration`

The heartbeat publish (max interval, default 60s) bounds how stale LOCF can be.

## Rollups

Pre-computed rollups make long trends cheap — continuous aggregates in
TimescaleDB, rollup tables in SQLite:

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

Compression is a TimescaleDB feature; the SQLite engine applies retention
only.

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
