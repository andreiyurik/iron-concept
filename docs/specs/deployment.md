---
status: draft
type: spec
depends-on: [architecture, cli, security]
---

# Deployment

IRON runs on your hardware, in your network, under your control. Cloud is
optional — not the default.

```
Mode 1 — Development    iron dev                    Docker Compose, local machine
Mode 2 — Local server   iron deploy --target local  Kamal 2, LAN server (production default)
Mode 3 — Cloud (opt.)   iron deploy --target cloud  Kamal 2, any VPS
Mode 4 — Edge (OT)      iron deploy --target edge   Kamal 2, Pi / mini-PC in the cabinet
```

Modes 2–4 use identical tooling — [Kamal 2](https://kamal-deploy.org)
([decision rationale](../decisions/0006-kamal-for-deployment.md)). Only the
target IP and network location differ. All modes can run simultaneously.

## Why local-first

- **Data sovereignty** — sensor data, alarms, and history stay inside the plant
- **No internet dependency** — the plant keeps running if the uplink dies
- **Regulatory** — some industries prohibit operational data leaving the facility
- A $150 mini-PC or an existing office PC is a fully capable IRON server for
  most plants (sizing: [guides/hardware.md](../guides/hardware.md))

## Mode 1 — Development

```bash
iron new myplant && cd myplant && iron dev
# → http://localhost:4000 with simulated data
```

`iron dev` starts the generated Compose stack: `iron_web`, `iron_core`
(`SIMULATE: true`), `timescaledb`, `nats -js`. No PLC required.

## Mode 2 — Local server (production default)

Deploy to any Linux machine on the plant network.

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
  clear: { PHX_HOST: iron.local, NATS_URL: "nats://nats:4222" }
  secret: [SECRET_KEY_BASE, DATABASE_URL]
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
iron deploy --target local   # subsequent: zero-downtime
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

## Mode 3 — Cloud (optional)

Same config, public IP, `ssl: true` (automatic Let's Encrypt). Use for remote
dashboards, multi-site management, or integrator-managed service. A cloud
instance can run as a read-only view against the plant's NATS/TimescaleDB —
it stores nothing.

Reference: Hetzner AX41 (€38/mo) or two for HA (€76/mo).

## Mode 4 — Edge agent (OT zone)

```yaml
servers:
  edge:
    hosts: ["192.168.10.5"]          # device in the OT/IT boundary
    cmd: /app/bin/iron_core
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

For harsh environments, an alternative packaging is Nerves (immutable firmware,
A/B partition rollback) — see [guides/hardware.md](../guides/hardware.md).

## Air-gapped installation

Many OT networks have no internet. The procedure is `docker save` → USB/SCP →
`docker load`; Kamal detects present images and skips the pull:

```bash
# machine with internet:
docker pull ghcr.io/getiron/iron-core:0.3.0 --platform linux/arm64
docker save ghcr.io/getiron/iron-core:0.3.0 | gzip > iron-core-arm64.tar.gz
# transfer, then on the device:
docker load < iron-core-arm64.tar.gz
```

## Environment variables

### iron-web

| Variable | Required | Description |
|---|---|---|
| `SECRET_KEY_BASE` | ✅ | Phoenix secret (`mix phx.gen.secret`) |
| `DATABASE_URL` | ✅ | PostgreSQL connection string |
| `NATS_URL` | ✅ | NATS server |
| `PHX_HOST` | ✅ | Hostname for cookie security |
| `IRON_TELEGRAM_TOKEN` | — | Alarm notification bot |
| `IRON_ADMIN_EMAIL` | — | Initial admin account on first boot |

### iron-core

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
kamal app exec --host 192.168.10.5 "bin/iron_core nats ping"     # OT→IT firewall, port 4222?
kamal app exec --host 192.168.10.5 "bin/iron_core modbus ping 192.168.10.100"  # PLC reachable?
kamal proxy logs                                # SSL: DNS pointing here? port 80 open?
```
