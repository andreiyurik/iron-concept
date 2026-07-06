# Glossary

One definition per term, used consistently across every document. If a
document contradicts the glossary, the document is wrong — fix one of them.

| Term | Definition |
|---|---|
| **Tag** | A named mapping of one physical signal to one value in the system; the unit of configuration, history, and display. Spec: [specs/tag-model.md](specs/tag-model.md) |
| **Quality** | Per-sample trust marker: `GOOD` / `UNCERTAIN` / `BAD`. Part of the value, not metadata |
| **Deadband** | Minimum change required to publish a new sample; suppresses noise traffic. Never delays quality changes or alarm-limit crossings |
| **Scan rate** | Polling interval for a tag (e.g. `1s`) |
| **Edge agent** | iron-core: the Rust process next to the PLC — polls, converts, filters, buffers, evaluates first-level alarms, executes authorized commands |
| **Store-and-forward** | Edge buffering (SQLite) during network loss with ordered replay after reconnect; no data loss across outages |
| **Heartbeat** | Forced periodic publish (default 60s) so "stable value" is distinguishable from "dead link" |
| **READ path** | Sensors → edge → broker → storage → UI. Strictly one-directional. Spec: [specs/read-write-separation.md](specs/read-write-separation.md) |
| **WRITE path / command** | Operator → auth → RBAC → audit → Command Service → edge → machine. The only way IRON changes the physical world. Spec: [specs/command-path.md](specs/command-path.md) |
| **Command Service** | The single component allowed to publish on command subjects; the WRITE path's one entry point |
| **Alarm** | A defined abnormal condition that notifies humans. Detected at the edge, managed at the server, **never** triggers commands. Spec: [specs/alarm-engine.md](specs/alarm-engine.md) |
| **Ack / RTN / shelving** | Alarm lifecycle terms (ISA-18.2): operator acknowledgment; return-to-normal; temporary audited muting of a nuisance alarm |
| **Alarm flood** | Annunciation rate exceeding what an operator can process; the alarm engine detects and manages it |
| **Interlock** | Automatic protective logic in the PLC (or safety PLC) — observed by IRON, never implemented by it |
| **Safety PLC / SIL** | Certified controller / Safety Integrity Level for functions whose failure harms people. Out of IRON's scope, by design |
| **Historian** | The time-series store (TimescaleDB) for samples, alarms, and audit. Spec: [specs/historian.md](specs/historian.md) |
| **LOCF** | Last observation carried forward — how sparse (deadband-filtered) history is read: no row means "unchanged", not "unknown" |
| **Continuous aggregate** | Pre-computed time-weighted rollup (1m/5m/1h/1d) serving long trend queries |
| **Device type** | Reusable template of tags + alarms + commands + widgets, instantiated per equipment unit (cf. Ignition UDT). Spec: [specs/device-types.md](specs/device-types.md) |
| **IO list** | The instrument/signal spreadsheet from the electrical designer; importable via `iron import io-list` |
| **Mimic (mnemonic diagram)** | The P&ID-style operator screen; in IRON, an SVG with `data-iron-*` bindings. Spec: [specs/visual-system.md](specs/visual-system.md) |
| **Widget** | A pre-built dashboard component (gauge, trend, alarm panel…) bound to tags or commands |
| **UNS (Unified Namespace)** | The pattern where every data point has one canonical hierarchical address; IRON implements it as NATS subjects |
| **Subject** | A NATS topic, e.g. `data.plant.line1.reactor1.temperature`; authorization is per-subject |
| **OT / IT** | Operational technology (plant floor: PLCs, sensors, VLAN 10) / information technology (servers, VLAN 20) |
| **PLC** | Programmable logic controller — the device executing control logic in a deterministic scan cycle |
| **Scan cycle** | The PLC's read-inputs → execute-logic → write-outputs loop, typically 1–10ms |
| **IEC 61131-3** | The PLC programming language standard (Structured Text, Ladder, FBD…) |
| **Structured Text (ST)** | The textual IEC 61131-3 language; Git-friendly, the focus of [specs/plc-runtime.md](specs/plc-runtime.md) |
| **Online change** | Modifying a running PLC program without stopping the process — the hardest unsolved problem for open runtimes |
| **Commissioning** | On-site verification that every physical signal matches configuration; IRON's [`iron field`](specs/field-verification.md) workflow |
| **Drift** | Divergence between the configuration in Git and what actually runs; detected by `iron diff` |
| **WASM module** | Sandboxed, language-agnostic plugin (derived tags, conversions, notification channels) with manifest-declared capabilities; can never publish commands. Spec: [specs/extensions.md](specs/extensions.md) |
| **Satellite** | Out-of-process integration (custom driver, analytics, ML) in any language, connected to NATS with its own scoped credentials |
| **iron lite** | Single-device deployment mode: one artifact, in-process transport, SQLite storage, identical configuration format; upgradeable via `iron migrate --to-full`. Spec: [specs/deployment.md](specs/deployment.md) |
| **4–20mA** | The dominant analog signal standard: 4mA = range min, 20mA = max; below 4mA indicates a broken wire |
| **Modbus / OPC-UA / S7 / MQTT / IO-Link** | Field protocols the edge agent speaks; see [specs/tag-model.md](specs/tag-model.md) for source URI schemes |
| **OEE** | Overall Equipment Effectiveness = Availability × Performance × Quality; the plant productivity metric. Honest treatment: [business/economics.md](business/economics.md) |
| **iron-core / iron-web** | The two deliverables: Rust edge runtime / Elixir-Phoenix server (UI, historian, alarms, commands) |
