---
status: draft
type: spec
depends-on: [read-write-separation, command-path]
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

The primary security property is structural, not cryptographic — and it is
enforced at three layers:

| Layer | Enforcement |
|---|---|
| Code | Separate processes, separate config trees, one Command Service entry point |
| Transport | Disjoint NATS subject namespaces with per-identity authorization |
| Network | OT/IT VLAN segmentation, OT→IT connection initiation only |

Full specification: [read-write-separation.md](read-write-separation.md).

## Broker authorization model

The broker carries both data and command subjects, so broker authorization is
a security boundary, not a configuration detail:

- Every connecting identity (each edge agent, iron-web, Command Service) has
  its own credentials (NKeys / mTLS client certs), individually revocable.
- Edge agent `edge-line1`: publish `data.plant.line1.>` only; subscribe
  `cmd.plant.line1.>` only. It cannot read other areas' commands or forge
  other areas' data.
- Command Service: the only identity with publish rights on `cmd.>`.
- Dashboards/historian identities: subscribe `data.>`, no publish rights at all.
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
│  NATS · TimescaleDB · iron-web · Audit   │
└──────────────────┬───────────────────────┘
                   │ HTTPS / WSS, authenticated
┌──────────────────▼───────────────────────┐
│  Operators · Engineers · Management      │
└──────────────────────────────────────────┘
```

This is the IEC 62443 zones-and-conduits model: the only conduit from OT to IT
is the agent's outbound NATS connection, and the only path back into OT is a
command on an authorized subject, originated by the Command Service.

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
