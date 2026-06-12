---
status: accepted
type: decision
---

# ADR 0006 — Kamal 2 for Deployment

## Context

IRON deploys to plant LAN servers, optional cloud VPSes, and ARM edge devices
in OT cabinets — operated by integrators and engineers, not SREs. The
deployment story must be: one command, zero downtime, works air-gapped,
no orchestrator to babysit.

## Decision

`iron deploy` wraps Kamal 2 (the deployment tool from the Rails ecosystem):
SSH + Docker on any Linux host, health-check-gated container swaps with
automatic rollback, multi-arch images for ARM64 edge devices, identical
workflow for LAN/cloud/edge targets ([specs/deployment.md](../specs/deployment.md)).
The wrapper adds IRON's conventions: the two-container layout,
backwards-compatible-migration enforcement, config SHA stamping for
`iron diff`.

## Alternatives

- **Kubernetes / k3s** — the UMH route. Rejected as default: a cluster is a
  full-time pet for the exact audience IRON serves; k3s on a single mini-PC
  adds failure modes without adding value. Helm charts remain an enterprise
  option later.
- **Plain Docker Compose + SSH scripts** — no health-gated rollouts, no
  rollback story, everyone reinvents the same scripts badly.
- **Nerves firmware** — excellent for harsh-environment edge devices, kept as
  an advanced packaging option, not the default workflow
  ([guides/hardware.md](../guides/hardware.md)).
- **systemd + bare binaries** — attractive minimalism, loses the multi-arch
  image pipeline and uniform rollback; may return for the PLC runtime where
  Docker latency jitter matters.

## Trade-offs

Docker becomes a hard dependency on every target including edge devices;
registry access requires the air-gapped `docker save/load` path; Kamal is a
younger tool whose breaking changes we absorb on behalf of users.

## Revisit when

Edge determinism requirements (PLC runtime phase) conflict with
containerization, or Kamal's direction diverges from non-Rails use cases.
