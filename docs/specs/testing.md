---
status: draft
type: spec
depends-on: [edge-agent, alarm-engine, cli, field-verification]
---

# Testing

An automation engineer should never need physical access to a PLC to develop,
test, or validate a system. Traditional SCADA testing — configure in the vendor
tool, connect a real PLC, eyeball the screen, sign a paper checklist — is not
repeatable, not versioned, and not automatable. IRON replaces it with four
levels; only the last one touches hardware.

| Level | Command | Hardware | When |
|---|---|---|---|
| 1 Unit | `cargo test` / `npm test` (UI) | No | Every commit |
| 2 Integration | `cargo test --features integration` | No (Docker) | Every commit, CI |
| 3 Simulation | `iron test --sim` | No | Development, CI |
| 4 Field | `iron field` | Yes | On-site commissioning ([field-verification.md](field-verification.md)) |

The TDD workflow these levels support is described in
[guides/tdd.md](../guides/tdd.md).

## Level 1 — Unit tests

Pure logic, no external dependencies. What gets tested and why it matters:

```rust
// Deadband — including the alarm-crossing override (see edge-agent.md)
#[test]
fn publishes_when_alarm_limit_crossed_within_deadband() {
    // 179.8 → 180.2 is only 0.4 with deadband 0.5 — but it crosses HIGH at 180.0
    let f = filter_with(deadband: 0.5, alarm_limits: &[180.0]);
    assert!(f.should_publish(sample(180.2)));
}

// Engineering conversion — unit errors have caused real accidents
#[test]
fn converts_midpoint() { assert_eq!(convert_ai(12.0, 4.0, 20.0, 0.0, 200.0), 100.0); }

#[test]
fn out_of_range_raw_clamps_and_degrades_quality() {
    let s = convert(3.5 /* broken wire */);
    assert_eq!(s.value, 0.0);
    assert_eq!(s.quality, Quality::Uncertain);
}
```

```rust
// Tag engine: update state, notify exactly the subscribers of that tag
#[tokio::test]
async fn tag_update_notifies_subscribers_only() {
    let engine = TagEngine::new();
    let mut rx = engine.subscribe("reactor_01.temperature");
    let mut other = engine.subscribe("reactor_01.pressure");
    engine.update(sample("reactor_01.temperature", 87.5, Quality::Good));
    assert_eq!(rx.recv().await.unwrap().value, 87.5);
    assert!(other.try_recv().is_err());
}

// RBAC behavior is tested explicitly — "probably works" is not acceptable
#[test]
fn viewer_role_cannot_send_any_command() {
    let svc = CommandService::for_test();
    let res = svc.submit(viewer_user(), "pump_01.start", Value::Bool(true));
    assert!(matches!(res, Err(CommandError::Unauthorized)));
}
```

The same `iron-domain` crate is tested once and reused by edge, server, and
WASM modules — the alarm state machine and deadband tests above run in every
target. UI components are tested without a browser engine via Vitest +
Testing Library: mount the component, feed it a tag update, assert on DOM:

```ts
test('BAD quality renders a visual warning', async () => {
  const { container } = render(Gauge, { tag: 'reactor_01.temperature' })
  tagStore.apply({ tag: 'reactor_01.temperature', value: 0.0, quality: 'BAD' })
  await tick()
  expect(container.querySelector('[data-tag="reactor_01.temperature"]'))
    .toHaveClass('quality-bad')
})
```

## Level 2 — Integration: the chain test

Runs against real NATS + TimescaleDB + iron-edge (simulate mode) in Docker
Compose. The most important test in the system traces one signal through every
layer:

```
Simulated sensor value
  → iron-edge: conversion, quality, deadband
  → NATS JetStream publish
  → iron-server: tag engine → subscription fan-out → WebSocket frame
  → browser DOM: value visible
```

```rust
#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore)]
async fn signal_flows_from_edge_to_browser_full_chain() {
    let stack = TestStack::up().await;            // NATS + TimescaleDB + iron-edge (sim)
    let mut ws = stack.websocket("/dashboard/reactor_01").await;
    stack.simulator.set_tag("reactor_01.temperature", raw(14.4), Quality::Good);
    // 14.4mA on 4–20mA / 0–200°C → 130.0°C
    let frame = ws.next_update("reactor_01.temperature").await;
    assert_eq!(frame.value, 130.0);
    assert_eq!(frame.quality, Quality::Good);
}

#[tokio::test]
#[cfg_attr(not(feature = "integration"), ignore)]
async fn quality_degrades_to_bad_when_polling_stops() {
    let stack = TestStack::up().await;
    let mut ws = stack.websocket("/dashboard/reactor_01").await;
    stack.simulator.stop_tag("reactor_01.temperature");
    let frame = ws.next_update_after(tag_timeout()).await;
    assert_eq!(frame.quality, Quality::Bad);
}
```

The browser half of the chain — "the frame became a visible value" — is
covered at the widget level by Vitest (above). Browser system tests
(Playwright) against the same stack are *deferred* — slow, brittle, and so
far unnecessary — with their trigger in
[business/deferred.md](../business/deferred.md).

Other required integration scenarios: network loss → SQLite buffer → replay
with no historian gap; alarm ACTIVE at edge → escalation notification →
acknowledgment propagates to all connected clients; command lifecycle
ISSUED→CONFIRMED including feedback timeout → FAILED.

## Level 3 — Simulation scenarios

Deterministic, time-accelerated YAML timelines. Same scenario, same result,
every time. Scenarios are also *generated* from tag specs
(`iron generate scenarios` — see [vision/spec-driven.md](../vision/spec-driven.md)).

```yaml
# test/sim/scenarios/high_temp_alarm.yaml
name: high_temp_alarm
timeline:
  - t: 0s
    set: { reactor_01.temp_raw: 19.2 }     # 190°C — above HIGH (180)
  - t: 2s
    set: { reactor_01.temp_raw: 20.0 }     # 200°C — above HIGH_HIGH (195)
assertions:
  - at: 1s
    expect: { reactor_01.alarm_high: active, reactor_01.alarm_high_high: normal }
  - at: 3s
    expect: { reactor_01.alarm_high_high: active }
```

```yaml
# test/sim/scenarios/network_loss.yaml
timeline:
  - { t: 5s,  fault: nats_disconnect }
  - { t: 5s–25s, set: { reactor_01.temp_raw: 17.0 } }
  - { t: 25s, fault: nats_reconnect }
assertions:
  - after: nats_reconnect
    expect_historian_gap: false
    expect_buffer_cleared: true
```

Fault injection vocabulary: `nats_disconnect/reconnect`, `plc_timeout`,
`sensor_out_of_range`, `clock_skew`. When the PLC runtime exists
([plc-runtime.md](plc-runtime.md)), Structured Text programs compile to WASM
and run inside these same scenarios.

```bash
iron test --sim                          # all scenarios
iron test --sim --object reactor_01
# ✅ reactor_startup    (1.8s)
# ✅ high_temp_alarm    (0.4s)
# ❌ sensor_failure     — quality expected BAD, got UNCERTAIN at t=8s
```

## CI pipeline

```yaml
# .github/workflows/test.yml
jobs:
  unit:        { steps: [cargo test, npm test] }
  integration: { services: [nats, timescaledb], steps: [cargo test --features integration] }
  simulation:  { steps: [iron test --sim] }
  drift:       { schedule: nightly, steps: [iron diff --target production] }
```

`iron field` is never in CI — it requires physical hardware and runs on-site.

## Coverage map

```
Layer                    Unit   Integration   Simulation   Field
─────────────────────────────────────────────────────────────────
Browser DOM              Vitest     (deferred)    —          —
WebSocket protocol       ✅         ✅            —          —
Tag engine / Alarm mgmt  ✅         ✅            ✅          —
iron-domain (shared)     ✅         ✅            ✅          —
NATS transport           —          ✅            ✅          —
Edge logic (Rust)        ✅         ✅            ✅          —
Protocol drivers         ✅ (mocks) ✅ (diagslave)  —          —
Physical signals          —          —            —          ✅
```

Protocol drivers test against software simulators (diagslave for Modbus,
open62541 server for OPC-UA, snap7 for S7) — core drivers in `cargo test`,
community plugin drivers through `iron test --driver`
([extensions.md](extensions.md)). Phase 3 adds hardware-in-the-loop:
a physical PLC bench wired to CI, exercised on every driver change.
