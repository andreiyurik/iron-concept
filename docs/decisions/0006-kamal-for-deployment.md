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
The wrapper adds IRON's conventions: the role layout (one image, the role
chosen by the command line), backwards-compatible-migration enforcement,
commit and config-hash recording for `iron diff`, and the refusal to
overwrite un-pulled adjustments
([specs/config-lifecycle.md](../specs/config-lifecycle.md)).

## Alternatives

- **Kubernetes / k3s** — the UMH route. Rejected as default: a cluster is a
  full-time pet for the exact audience IRON serves; k3s on a single mini-PC
  adds failure modes without adding value. Helm charts remain an enterprise
  option later.
- **Plain Docker Compose + SSH scripts** — no health-gated rollouts, no
  rollback story, everyone reinvents the same scripts badly.
- **Immutable firmware images** (Yocto/Buildroot + A/B OTA) — excellent for harsh-environment edge devices, kept as
  an advanced packaging option, not the default workflow
  ([guides/hardware.md](../guides/hardware.md)).
- **systemd + bare binaries** — attractive minimalism, loses the multi-arch
  image pipeline and uniform rollback; may return for the PLC runtime where
  Docker latency jitter matters.

## Trade-offs

Docker becomes a hard dependency on every target including edge devices;
registry access requires the air-gapped `docker save/load` path; Kamal is a
younger tool whose breaking changes we absorb on behalf of users.

Two costs this ADR originally understated. Kamal is a Ruby tool, so
`iron deploy` needs Ruby or a Kamal container on the workstation — which
sits badly beside "nothing installed but the `iron` binary". And the thing
being deployed is a static binary with no runtime dependencies: a container
adds a daemon, an image pipeline, and a registry to ship one file. The
decision stands for now because it is untested, not because these costs are
small.

## Revisit when

**The first deployment of the single-binary default to a real Pi** — the
earliest evidence available. If SSH + systemd + an atomic binary swap with a
health check and rollback does the job in a few hundred lines inside
`iron deploy`, that becomes the default and containers remain for the
accessories (NATS, TimescaleDB) that need them. Also: edge determinism
requirements (PLC runtime phase) conflict with containerization, or Kamal's
direction diverges from non-Rails use cases.
