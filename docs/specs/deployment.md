---
status: draft
type: spec
depends-on: [architecture, cli, security]
---

# Deployment

IRON runs on your hardware, in your network, under your control. Cloud is
optional — not the default. And the default is **one binary, no services**:

```
Default    iron new && iron dev / iron server      one binary · SQLite · in-process bus · no Docker, no NATS, no Postgres
Plant      iron deploy --target plant              same binary + NATS JetStream + TimescaleDB · Kamal 2 · LAN server
Edge       iron deploy --target edge-01            the edge role on a Pi / mini-PC in the OT zone · Kamal 2
Cloud      iron deploy --target cloud              optional VPS · Kamal 2
```

The single binary is not a "lite edition" — it is IRON. Plant mode is the
same binary told where the broker and the database are, for when a site
outgrows one box. Rails ships SQLite by default and Postgres when you need
it; IRON does the same ([ADR 0009](../decisions/0009-rust-for-the-server.md)).

## Default — one binary, zero services

A greenhouse, a workshop, a pump station, and a developer's laptop all get the
same thing: one installable artifact and zero operational surface.

```bash
iron new greenhouse && cd greenhouse
iron dev                     # simulator on, hot reload, dashboard on :4000
iron server                  # the same thing on the Pi, pointed at real hardware
```

What changes when a site moves to plant mode — and what deliberately does not:

| | default (one binary) | plant |
|---|---|---|
| Transport | In-process channel | NATS JetStream |
| Storage | SQLite (same logical schema) | TimescaleDB |
| Processes | edge + server + UI in one process | edge on OT hosts, server on the LAN, separate containers |
| Tag specs, commands, alarms, UI, CLI | **identical** | **identical** |
| Scope ceiling | ~2,000 tags, one host, no HA | plant scale, HA |

Normative rules:

- The configuration format is byte-identical between modes. `iron validate`
  output for a project MUST NOT depend on the mode.
- `iron deploy --target plant` from a single-binary site migrates in place:
  same YAML, history exported into TimescaleDB, nothing relearned. The
  default is a starting point, never a trap.
- The default mode preserves the architectural invariants that matter at any
  size: READ/WRITE separation (in-process, the command executor remains a
  distinct module behind the same Command Service API), quality semantics,
  append-only command journal, LOCF history semantics.
- The default intentionally trades the "one database" principle
  ([ADR 0004](../decisions/0004-timescaledb.md)) for the "five minutes to
  first dashboard" principle — at 40 sensors, operational simplicity
  outranks engine uniformity.

## Development

`iron dev` is the default mode plus the simulator plus hot reload
([cli.md](cli.md)). It needs nothing installed but the `iron` binary — no
Docker, no Windows-vs-Linux difference, no services. This is the five-minute
path and it is the acceptance test of every release.

`iron dev --plant` starts the generated Compose stack (`iron_server`,
`iron_edge` with `SIMULATE: true`, `timescaledb`, `nats -js`) for developers
who need to exercise the plant topology locally. It is the exception, not
the entry point.

## Why local-first

- **Data sovereignty** — sensor data, alarms, and history stay inside the plant
- **No internet dependency** — the plant keeps running if the uplink dies
- **Regulatory** — some industries prohibit operational data leaving the facility
- A $150 mini-PC or an existing office PC is a fully capable IRON server for
  most plants (sizing: [guides/hardware.md](../guides/hardware.md))

## Plant — LAN server with NATS and TimescaleDB

Deploy to any Linux machine on the plant network when a site outgrows one
box: more than one host, more than ~2,000 tags, or HA.

```yaml
# config/deploy.yml
service: iron
image: ghcr.io/getiron/iron
servers:
  web: ["192.168.1.100"]
proxy:
  host: iron.local
  ssl: false                  # LAN; enable with a domain + cert
registry: { server: ghcr.io, username: you, password: [KAMAL_REGISTRY_PASSWORD] }
env:
  clear: { IRON_HOST: iron.local, NATS_URL: "nats://nats:4222" }
  secret: [IRON_SECRET_KEY, DATABASE_URL]
accessories:
  db:   { image: timescale/timescaledb:latest-pg16, host: 192.168.1.100,
          port: "127.0.0.1:5432:5432", volumes: ["/var/lib/iron/postgres:/var/lib/postgresql/data"] }
  nats: { image: nats:latest, host: 192.168.1.100,
          port: "127.0.0.1:4222:4222", cmd: "-js --store_dir /data",
          volumes: ["/var/lib/iron/nats:/data"] }
```

Secrets live in `.kamal/secrets` (gitignored). Then:

```bash
kamal setup                  # first time: installs Docker, starts everything
iron deploy --target plant   # subsequent: zero-downtime
kamal app logs --since 1h
kamal rollback               # instant rollback
```

Deploy contract:

- The old container keeps running until the new one passes health checks; a
  failed health check aborts the deploy with the old version untouched.
- Database migrations run before the new container starts and MUST be
  backwards-compatible with the running version; `iron deploy` refuses
  migrations not flagged as such.
- Every deploy stamps the running config with its Git SHA — this is what
  `iron diff` checks against ([cli.md](cli.md)).

### Split roles (deferred)

The same binary can later run as `iron server --role ui` in a DMZ with no
command credentials and `--role core` in the protected zone — a stronger form
of the contour rule in [security.md](security.md). Deferred, with its
trigger, in [business/deferred.md](../business/deferred.md).

### High availability

Two cheap servers beat one expensive one:

```yaml
servers:
  web: ["192.168.1.100", "192.168.1.101"]
accessories:
  db:
    hosts: ["192.168.1.100", "192.168.1.101"]   # Patroni: PostgreSQL failover < 10s
```

Two Beelink EQ12 units ≈ $300 total. That is the honest answer to "what if the
server dies".

## Cloud (optional)

Same config, public IP, `ssl: true` (automatic Let's Encrypt). Use for remote
dashboards, multi-site management, or integrator-managed service. A cloud
instance can run as a read-only view against the plant's NATS/TimescaleDB —
it stores nothing.

Reference: Hetzner AX41 (€38/mo) or two for HA (€76/mo).

## Edge — the edge role in the OT zone

```yaml
servers:
  edge:
    hosts: ["192.168.10.5"]          # device in the OT/IT boundary
    cmd: /app/bin/iron_edge
    options: { network: host }       # direct OT network access
    env:
      clear:
        NATS_URL: nats://192.168.1.100:4222
        TAGS_CONFIG: /app/config/tags.yaml
        BUFFER_PATH: /var/iron/buffer.db
```

```bash
iron deploy --target edge-01
# multi-arch image (ARM64 handled automatically) · restart ~4s
# SQLite buffer preserved across restarts — no data loss
```

For harsh environments, an alternative packaging is an immutable firmware
image with A/B partition rollback (Yocto/Buildroot + RAUC or Mender-style
OTA) — see [guides/hardware.md](../guides/hardware.md). The `iron` binary is
static and has no runtime dependencies, which is what makes that packaging
cheap.

## Air-gapped installation

Many OT networks have no internet. The procedure is `docker save` → USB/SCP →
`docker load`; Kamal detects present images and skips the pull:

```bash
# machine with internet:
docker pull ghcr.io/getiron/iron-edge:0.3.0 --platform linux/arm64
docker save ghcr.io/getiron/iron-edge:0.3.0 | gzip > iron-edge-arm64.tar.gz
# transfer, then on the device:
docker load < iron-edge-arm64.tar.gz
```

## Environment variables

### iron-server

| Variable | Required | Description |
|---|---|---|
| `IRON_SECRET_KEY` | ✅ | Session/token signing secret (`iron secret generate`) |
| `DATABASE_URL` | ✅ | PostgreSQL connection string |
| `NATS_URL` | ✅ | NATS server |
| `IRON_HOST` | ✅ | Hostname for cookie security |
| `IRON_TELEGRAM_TOKEN` | — | Alarm notification bot |
| `IRON_ADMIN_EMAIL` | — | Initial admin account on first boot |

### iron-edge

| Variable | Required | Description |
|---|---|---|
| `NATS_URL` | ✅ | NATS server to publish to |
| `TAGS_CONFIG` | ✅ | Path to tag specs |
| `BUFFER_PATH` | — | SQLite buffer (default `/var/iron/buffer.db`) |
| `LOG_LEVEL` | — | `debug`/`info`/`warn`/`error` |

## Troubleshooting

```bash
kamal app logs --since 10m                      # container won't start
kamal accessory logs db --since 30m             # database issues
kamal app exec --host 192.168.10.5 "bin/iron_edge nats ping"     # OT→IT firewall, port 4222?
kamal app exec --host 192.168.10.5 "bin/iron_edge modbus ping 192.168.10.100"  # PLC reachable?
kamal proxy logs                                # SSL: DNS pointing here? port 80 open?
```
