---
status: draft
type: spec
depends-on: [read-write-separation, command-path, transport]
---

# Security Architecture

Classical SCADA was designed for air-gapped networks that no longer exist.
Modern plants have IT/OT convergence, remote access, and vendor VPNs; the
attack surface is large and the legacy software was never designed for it.

IRON treats security as an architectural constraint. Every design decision is
evaluated against two questions:

1. Does this make it easier to read data without authorization?
2. Does this make it easier to send commands without authorization?

If either answer is yes, the design is wrong.

## Structural security: READ/WRITE separation

The primary security property is structural, not cryptographic. Which
layers enforce it depends on the deployment, and this table says so plainly:

| Layer | Enforcement | Default (one process) | Plant mode |
|---|---|:--:|:--:|
| Code | Write handles held only by the command executor; READ-path crates cannot depend on the Command Service (checked in CI); read-only WebSocket; separate config trees | ✅ | ✅ |
| Transport | Disjoint subject roots with per-identity authorization | — | ✅ |
| Network | OT/IT VLAN segmentation, OT→IT connection initiation only | — | ✅ |
| Hosts | Outward-facing role holds no command credentials (*deferred*) | — | optional |

The default deployment relies on the code layer alone. That is a real
boundary — it removes whole classes of bugs — but it is one process: an
attacker who gains code execution in it has everything. Sites that need
more than that need plant mode.

Full specification: [read-write-separation.md](read-write-separation.md).

## Broker authorization model

The broker carries both data and command subjects, so broker authorization is
a security boundary, not a configuration detail:

- Every connecting identity (each edge agent, iron-server, Command Service,
  each plugin) has its own credentials (NKeys / mTLS client certs),
  individually revocable.
- Subject roots and what travels on each are defined in
  [transport.md](transport.md). Permissions per identity, for a site `plant`
  and an edge `edge-line1` owning the area `line1`:

| Identity | May publish | May subscribe |
|---|---|---|
| Edge `edge-line1` | `data.plant.line1.>` · `alarm.plant.line1.>` · `result.plant.line1.>` · `edge.plant.edge-line1.>` | `cmd.plant.line1.>` |
| Command Service | `cmd.>` — the only identity that can | `result.>` |
| Server READ path (tag engine, historian, alarms) | derived-tag results on `data.>` · `alarm.>` for server-detected alarms | `data.>` · `alarm.>` · `edge.>` · `result.>` |
| Plugin | the data subjects its declaration names | the data subjects its declaration names |

- An edge cannot read another area's commands or forge another area's data.
- No stream may capture `cmd.>`: commands are never stored by the broker.
- All connections over TLS. Plants with elevated requirements MAY run a
  physically separate broker for `cmd.>` — the architecture does not assume
  shared transport, it merely permits it.

## Network segmentation

```
┌──────────────────────────────────────────┐
│  OT Zone (VLAN 10)                       │
│  PLCs · Sensors · I/O · Edge agent       │
│  No internet. No inbound from IT.        │
└──────────────────┬───────────────────────┘
                   │ Edge agent initiates outbound to NATS (OT → IT)
                   │ IT → OT: blocked by default
┌──────────────────▼───────────────────────┐
│  IT Zone (VLAN 20)                       │
│  NATS · TimescaleDB · iron-server · Audit│
└──────────────────┬───────────────────────┘
                   │ HTTPS / WSS, authenticated
┌──────────────────▼───────────────────────┐
│  Operators · Engineers · Management      │
└──────────────────────────────────────────┘
```

This is the IEC 62443 zones-and-conduits model: the only conduit from OT to IT
is the agent's outbound NATS connection, and the only path back into OT is a
command on an authorized subject, originated by the Command Service.

### Contours: the outward-facing role holds no command credentials

The same principle, applied one level up. Anything that faces operators,
office networks, or the internet is exposed by design; the Command Service
is what must stay inside. In the default single-process deployment the two
are separated by code alone; the *deferred* split-roles
deployment ([business/deferred.md](../business/deferred.md)) separates them
by hosts:

```
┌────────────────────────────────────────────┐
│  DMZ / office VLAN                         │
│  iron server --role ui                     │   NATS: subscribe data.> only
│  dashboards · REST read · WS · MCP (read)  │   cmd.> credentials: none on this host
└──────────────────┬─────────────────────────┘
                   │ NATS only (authenticated request subject for commands)
┌──────────────────▼─────────────────────────┐
│  IT zone (protected)                       │
│  iron server --role core                   │   NATS: the only cmd.> publisher
│  Command Service · historian · alarms      │   no inbound from the DMZ except NATS
│  NATS · TimescaleDB · Audit                │
└──────────────────┬─────────────────────────┘
                   │ OT → IT outbound only (as above)
┌──────────────────▼─────────────────────────┐
│  OT zone · edge agents · PLCs              │
└────────────────────────────────────────────┘
```

Rule, for a split-roles deployment: a host that is reachable from outside
the protected zone MUST NOT hold credentials that can publish on `cmd.>`.
(The default deployment is one host and cannot satisfy this; that is the
limit stated in the table above.) A compromise of the `ui` role then
yields read access to plant data — serious, and exactly the incident the
READ/WRITE separation is designed to bound — but no ability to write to a
machine. Citadel egress: the `core` role never initiates connections toward
the DMZ. `REQ-SEC-001 integration`

## Authentication and authorization

- All human access requires authentication; TOTP or hardware key recommended
  for write-capable roles (but see the urgency rule in
  [command-path.md](command-path.md): never 2FA on emergency commands).
- API access uses short-lived tokens. Session cookies are HttpOnly; tokens are
  never stored in localStorage.
- Enterprise tier: SAML 2.0 / OIDC / LDAP.

### Role-based access control

Roles are assigned **per object**, not globally — an operator can start pumps
on Line 1 and only view Line 2, with no custom code:

| Role | Read data | Ack alarms | Send commands | Configure | Manage users |
|---|---|---|---|---|---|
| Viewer | ✅ | ❌ | ❌ | ❌ | ❌ |
| Operator | ✅ | ✅ | ✅ (authorized objects) | ❌ | ❌ |
| Engineer | ✅ | ✅ | ✅ | ✅ | ❌ |
| Admin | ✅ | ✅ | ✅ | ✅ | ✅ |

## Audit

Every WRITE action is journaled append-only before transmission, with the
availability-first policy specified in [command-path.md](command-path.md).
Configuration changes are audited by construction: configuration lives in Git
— author, diff, timestamp, and review are properties of the medium.

## Memory safety at the edge

The component with direct access to PLCs is written in Rust. Memory-safety
vulnerabilities account for the majority of critical CVEs in C/C++ codebases
(per Google's Android and Microsoft MSRC analyses); in an OT context a memory
corruption in the process that writes PLC registers is a *physical* safety
issue. Rationale: [decisions/0001-rust-for-edge.md](../decisions/0001-rust-for-edge.md).

## Standards alignment

IRON's architecture maps to IEC 62443 requirements:

| IEC 62443 requirement | IRON implementation |
|---|---|
| SL1 — Authentication | All access authenticated |
| SL1 — Audit log | Append-only command journal + Git config history |
| SL2 — RBAC | Object-level roles |
| SL2 — Use control | READ/WRITE separation at three layers |
| SL2 — Data integrity | TLS everywhere, PostgreSQL checksums |
| SL3 — MFA for critical ops | TOTP on administrative operations |
| SL3 — Zones and conduits | VLAN model, single authorized conduit |
| SL3 — Update management | Git-based config, signed releases |

**IRON does not claim IEC 62443 certification.** Certification is a formal
assessment process, not a property of code. The architecture is designed to
make that assessment straightforward. The same applies to NIS2: Git-based
configuration management, documented security architecture, and a standard
update mechanism address NIS2's risk-management and incident-handling
expectations, but compliance is an operator-level process that includes
people and procedures, not just software.

## What IRON does not claim

- **IRON is not a safety system.** Safety-critical functions (emergency stops,
  SIL-rated interlocks) must remain in dedicated safety PLCs that operate
  independently of IRON. IRON can monitor safety system state but must never
  be the primary safety mechanism.
- **IRON does not eliminate all attack vectors.** A compromised operator
  workstation, a malicious insider with valid credentials, or a vulnerable
  dependency can still cause harm. IRON is one layer of defense in depth.
