---
status: draft
type: spec
depends-on: [architecture, cli, security, transport, config-lifecycle]
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
| Transport binding | In-process channel | NATS JetStream |
| Storage engine | SQLite | TimescaleDB |
| Processes | `iron server --role all`: edge + server + UI in one process | `--role edge` on OT hosts, `--role server` on the LAN |
| Tag specs, commands, alarms, UI, CLI | **identical** | **identical** |
| Scope ceiling (target, unmeasured) | ~2,000 tags, one host | plant scale, several hosts |

Transport and storage are **independent choices** behind two contracts —
[transport.md](transport.md) and the storage contract in
[historian.md](historian.md). "Default" and "plant" are the two conventional
combinations, not the only legal ones: a site with one remote edge may run
NATS and keep SQLite. The first remote edge does not force a database
migration.

Normative rules:

- The configuration format is byte-identical between modes. `iron validate`
  output for a project MUST NOT depend on the mode. `REQ-DEP-001 integration`
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

`iron dev --plant` starts the generated Compose stack (`iron server --role
server`, `iron server --role edge` with simulated sources, `timescaledb`,
`nats -js`) for developers
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
box: more than one host, or more than ~2,000 tags.

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
  db:
    image: timescale/timescaledb:latest-pg16
    host: 192.168.1.100
    port: "127.0.0.1:5432:5432"
    volumes: ["/var/lib/iron/postgres:/var/lib/postgresql/data"]
  nats:
    image: nats:latest
    host: 192.168.1.100
    port: "127.0.0.1:4222:4222"
    cmd: "-js --store_dir /data"
    volumes: ["/var/lib/iron/nats:/data"]
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
  migrations not flagged as such. `REQ-DEP-002 integration`
- Every deploy records the Git commit and the config hash on the target —
  this is what `iron diff` checks against. How configuration is applied,
  what survives an apply, and why a deploy refuses to overwrite un-pulled
  adjustments are specified in [config-lifecycle.md](config-lifecycle.md).

### Split roles (deferred)

The same binary can later run as `iron server --role ui` in a DMZ with no
command credentials and `--role core` in the protected zone — a stronger form
of the contour rule in [security.md](security.md). Deferred, with its
trigger, in [business/deferred.md](../business/deferred.md).

### When the server dies

> **Status: not designed yet.** IRON has no high-availability specification.
> What follows is what the architecture already gives a site, and what it
> does not.

What a single-server site has today, by design:

- **The plant keeps running.** Control lives in the PLCs; IRON is
  supervisory.
- **Nothing is lost.** Each edge buffers and replays
  ([edge-agent.md](edge-agent.md)), keeps detecting alarms, keeps its local
  notification channel, and serves its read-only fallback page.
- **Recovery is a restore.** Configuration is in Git and the server is
  designed to be killed and restarted ([ADR 0009](../decisions/0009-rust-for-the-server.md)).
  A second cheap box kept as a **cold or warm standby** — same binary,
  database replicated or restored from backup, switched over by a person —
  is the honest v1 answer, and at roughly $150 it is an affordable one.

What that does not give: automatic failover. Doing it correctly needs more
than two machines — PostgreSQL failover managers such as Patroni require a
consensus store with a quorum, so the minimum is **three** voters (the third
can be a small witness node), and two nodes alone cannot tell a dead peer
from a broken link. It also needs answers IRON has not written down: which
server is the single active Command Service, and how alarm state and the
command journal move with it. Automatic failover is *deferred* with its
trigger in [business/deferred.md](../business/deferred.md) and will get its
own spec rather than a paragraph here.

## Cloud (optional)

Same config, public IP, `ssl: true` (automatic Let's Encrypt). Use for remote
dashboards, multi-site management, or integrator-managed service. A cloud
instance can run as a read-only view against the plant's NATS/TimescaleDB —
it stores nothing.

Reference: Hetzner AX41 (€38/mo).

## Edge — the edge role in the OT zone

```yaml
servers:
  edge:
    hosts: ["192.168.10.5"]          # device in the OT/IT boundary
    cmd: iron server --role edge
    options: { network: host }       # direct OT network access
    areas: [reactor_01]                  # the areas this edge owns
    env:
      clear:
        IRON_EDGE_ID: edge-01
        NATS_URL: nats://192.168.1.100:4222
        IRON_CONFIG: /app/config
        IRON_BUFFER_PATH: /var/iron/buffer.db
```

Every area is owned by exactly one edge ([transport.md](transport.md)); the
edge polls the tags of its areas and executes their commands, and its broker
credentials are scoped to them ([security.md](security.md)).

```bash
iron deploy --target edge-01
# multi-arch image (ARM64 handled automatically) · restart in seconds (target)
# SQLite buffer preserved across restarts — no data loss
```

For harsh environments, an alternative packaging — *deferred*
([business/deferred.md](../business/deferred.md)) — is an immutable firmware
image with A/B partition rollback (Yocto/Buildroot + RAUC or Mender-style
OTA) — see [guides/hardware.md](../guides/hardware.md). The `iron` binary is
static and has no runtime dependencies, which is what makes that packaging
cheap.

## Air-gapped installation

Many OT networks have no internet. The procedure is `docker save` → USB/SCP →
`docker load`; Kamal detects present images and skips the pull:

```bash
# machine with internet:
docker pull ghcr.io/getiron/iron:0.3.0 --platform linux/arm64
docker save ghcr.io/getiron/iron:0.3.0 | gzip > iron-arm64.tar.gz
# transfer, then on the device:
docker load < iron-arm64.tar.gz
```

There is one image, because there is one binary; the role is chosen by the
command line.

## Environment variables

`iron dev` needs none of these. Setting `NATS_URL` selects the NATS transport
binding; setting `DATABASE_URL` selects TimescaleDB. Unset, the process uses
in-process transport and SQLite.

### Roles `all` and `server`

| Variable | Required | Description |
|---|---|---|
| `IRON_SECRET_KEY` | when serving beyond localhost | Session/token signing secret — any long random string |
| `IRON_HOST` | when serving beyond localhost | Hostname for cookie security |
| `DATABASE_URL` | plant mode | PostgreSQL connection string; unset = SQLite in `IRON_DATA_DIR` |
| `NATS_URL` | plant mode | NATS server; unset = in-process transport |
| `IRON_CONFIG` | — | Path to the `config/` directory (default `./config`) |
| `IRON_DATA_DIR` | — | SQLite database, journal, buffer (default `/var/iron`) |
| `IRON_TELEGRAM_TOKEN` | — | Alarm notification bot |
| `IRON_ADMIN_EMAIL` | — | Initial admin account on first boot |

### Role `edge`

| Variable | Required | Description |
|---|---|---|
| `IRON_EDGE_ID` | ✅ | This edge's identity; must match a target in `deploy.yml` |
| `NATS_URL` | ✅ | NATS server to connect to |
| `IRON_CONFIG` | — | Path to the `config/` directory (default `./config`) |
| `IRON_BUFFER_PATH` | — | SQLite buffer (default `/var/iron/buffer.db`) |
| `LOG_LEVEL` | — | `debug`/`info`/`warn`/`error` |

## Troubleshooting

```bash
kamal app logs --since 10m                      # container won't start
kamal accessory logs db --since 30m             # database issues
kamal app exec --host 192.168.10.5 "iron validate"     # source reachability from the edge: PLC answering? firewall?
kamal proxy logs                                # SSL: DNS pointing here? port 80 open?
```
