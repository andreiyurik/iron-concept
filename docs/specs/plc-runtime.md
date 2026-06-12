---
status: exploration
type: spec
depends-on: [architecture, testing]
---

# PLC Runtime (Long-Term Exploration)

> **Status: exploration.** IRON's committed scope is SCADA. This document
> describes a long-term direction, gated behind evidence from Phases 0–2.
> Nothing here is promised. The near-term position is pragmatic: IRON
> integrates with CODESYS, TwinCAT, and vendor PLCs as they are —
> see [decisions/0007-codesys-today-iron-plc-later.md](../decisions/0007-codesys-today-iron-plc-later.md).

## The opportunity

The PLC runtime layer — what executes IEC 61131-3 programs and directly
controls machines — is owned by CODESYS, Siemens, and Rockwell. CODESYS, the
de-facto "independent" runtime, is closed source, written in C, licensed
per device (~€200–500), with binary project files Git cannot diff and no way
to simulate a program without hardware.

An open, memory-safe, Git-native, simulation-first runtime would unblock things
the industry currently lacks entirely: unit tests for PLC programs, CI/CD for
automation engineers, PLC education without hardware.

`plc-lang/rusty` — an existing Apache 2.0 IEC 61131-3 compiler in Rust
targeting LLVM — is the foundation to build on. Not a fork, not from scratch.

```
.st / .il / .fbd  →  rusty (existing)  →  LLVM IR  →  Native x86/ARM64 (PREEMPT_RT)
                                                    →  WASM (iron test --sim)
```

## What `iron test --sim` would enable

```bash
iron test --sim programs/reactor_control.st
# ✅ Startup sequence: OK (4.2s simulated)
# ✅ High temperature alarm: triggered at 182°C (limit: 180)
# ✅ Shutdown sequence: activated at 197°C (limit: 195)
# ⚠️  Watchdog timeout: not covered — add a scenario
```

Structured Text compiled to WASM, run in a sandbox with simulated I/O against
the same scenario format as the rest of IRON ([testing.md](testing.md)).
Version-controlled, unit-tested, CI-checked PLC programs — something no
mainstream vendor offers.

When the runtime runs inside iron-core, every IEC 61131-3 variable is natively
a tag in the NATS namespace — no OPC-UA server, no Modbus gateway between the
control program and the historian.

## The honest hard problems

Compiling Structured Text is the *easy* part. These are the reasons no open
runtime has displaced CODESYS, in order of difficulty:

1. **Online change.** Modifying a running program without stopping the process
   — pointer fixups, state migration, bumpless transfer, all mid-scan-cycle.
   This is *the* feature plants pay CODESYS and Siemens for, no open-source
   runtime has it, and any serious attempt must design for it from day one.
2. **Fieldbus I/O stacks.** A runtime without I/O is a calculator. EtherCAT
   has an open path (`ethercrab` in Rust); PROFINET and EtherNet/IP stacks are
   commercially licensed or enormous reverse-engineering efforts. Realistic
   initial scope: EtherCAT + Modbus only.
3. **Online monitoring and debugging.** Engineers expect to watch live
   variable values inside running logic, force I/O, set breakpoints in a scan
   cycle. Table stakes for adoption, large surface area.
4. **Determinism.** Linux PREEMPT_RT achieves ~<1ms jitter on commodity
   hardware — sufficient for the vast majority of process control, not for
   motion control. Scope honestly: process automation, not robotics.
5. **Certification.** IEC 61131-3 conformance, and for anything
   safety-adjacent IEC 61508 — a multi-year, capital-intensive path. Not
   attempted until the runtime has years of non-critical production history.

## Prior art

| | CODESYS | OpenPLC | IRON runtime (target) |
|---|---|---|---|
| Language | C (closed) | C/C++ (GPLv3) | Rust (Apache 2.0) |
| Per-device license | ~€200–500 | Free | Free |
| WASM simulation | No | No | Yes — the differentiator |
| Online change | **Yes** | No | The hard goal |
| Fieldbus support | Broad, certified | Modbus-centric | EtherCAT + Modbus first |
| Git-friendly sources | No (binary project) | Partial | Yes (.st text) |
| Niche | Industry default | Education, research | Test-first development |

OpenPLC deserves respect: it proved demand for an open runtime. Its niche is
education and research; it lacks the determinism story, the toolchain, and the
online-change path for production plants. IRON's differentiators would be the
test-first toolchain (WASM sim in CI) and native integration with the IRON
data layer.

## Sequencing (evidence-gated)

```
Phase 3 — Contribute WASM target to plc-lang/rusty upstream
          iron test --sim for ST programs (proof of concept)
          Bridge: PLC variables → NATS tags
Phase 4 — iron-plc standalone: PREEMPT_RT scan cycle, EtherCAT via ethercrab
          Hardware-in-the-loop bench; vendor conversations (Kunbus, Advantech)
Phase 5+ — Online change design; conformance/certification path
```

The Phase 3 goal is not a product — it is evidence that the ST→WASM pipeline
works and that simulation-first PLC development is useful to real engineers.
The product decision comes after the evidence.
