---
status: accepted
type: decision
---

# ADR 0001 — Rust for the Edge Agent

## Context

The edge agent is the component with direct access to PLC hardware. It runs
unattended for years on cheap ARM/x86 devices in cabinets, and a memory
corruption in a process that writes PLC registers is a physical safety issue,
not just a security issue. Memory-safety bugs account for the majority of
critical CVEs in large C/C++ codebases (Google Android, Microsoft MSRC data).

## Decision

iron-core is written in Rust: memory safety without garbage collection, no GC
pauses in the polling loop, single static binary for trivial air-gapped
deployment, first-class ARM64/x86 cross-compilation, mature ecosystem for the
domain (`tokio-modbus`, `opcua`, `ethercrab`, `linux-embedded-hal`).

## Alternatives

- **C/C++** — the industry default; rejected for memory safety.
- **Go** — GC pauses are tolerable for polling but the embedded/fieldbus
  ecosystem is thinner, and FFI to protocol stacks is more painful.
- **Elixir everywhere (Nerves)** — attractive operationally, but soft-realtime
  BEAM is the wrong layer for tight protocol framing and for the future PLC
  runtime; Nerves remains a supported *packaging* option for edge devices.

## Trade-offs

Rust's learning curve shrinks the contributor pool relative to Go; compile
times are real; some protocol crates are young and will need hardening — that
work is part of IRON's value to the ecosystem.

## Revisit when

A protocol stack we critically need exists only as a C library with an
unmaintainable FFI surface — then we isolate it in a separate process rather
than abandon the language choice.
