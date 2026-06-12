---
status: accepted
type: decision
---

# ADR 0007 — Integrate CODESYS Today, Explore Our Own Runtime Later

## Context

IRON's vision document criticizes CODESYS sharply (closed C runtime,
per-device licensing, binary project files) — yet IRON's hardware guidance
recommends CODESYS-based controllers (WAGO PFC200) and TwinCAT for control
logic. Without an explicit decision, this reads as a contradiction.

## Decision

Two positions, both held deliberately:

1. **Today (Phases 0–2):** IRON is SCADA. The PLC layer is whatever the plant
   has or buys — CODESYS on WAGO, TwinCAT on Beckhoff, Siemens S7, CLICK PLUS.
   IRON reads from and writes to them via standard protocols and never asks a
   plant to replace a working, certified controller. Where the controller
   runs Linux (WAGO, CLICK PLUS), iron-core happily coexists on the same
   hardware, with the vendor runtime owning the scan cycle.
2. **Long-term (Phase 3+, evidence-gated):** explore an open Rust runtime on
   `plc-lang/rusty`, because the bottom of the stack deserves the same
   openness as the top ([specs/plc-runtime.md](../specs/plc-runtime.md)).
   This begins as upstream contributions and a simulation toolchain, not as a
   product promise.

The criticism of CODESYS's licensing and closedness stands. Recommending it
as today's pragmatic control layer also stands. These are compatible the same
way "Linux will replace UNIX" was compatible with running UNIX in 1992.

## Alternatives

- **Attack the runtime first** — maximal vision, near-certain failure: years
  of work before any user gets value, against the hardest entrenchment in the
  industry.
- **Never do the runtime** — leaves the per-device tax and the
  binary-project-file workflow permanently in place under an otherwise open
  stack.

## Trade-offs

Holding both positions invites "so do you hate CODESYS or ship it?" — the
answer is this document. Integration effort spent on vendor PLCs is partially
disposable if the runtime succeeds; accepted, because that effort is exactly
what earns the production credibility the runtime would need.

## Revisit when

Phase 3 evidence (WASM target upstream, `iron test --sim` adoption) says the
runtime is or is not worth pursuing as a product.
