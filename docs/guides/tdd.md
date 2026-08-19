---
status: draft
type: guide
---

# Test-Driven Development in IRON

TDD is not a methodology imposed on IRON from outside. It is the natural
consequence of choosing Rust for the whole runtime — testing is built into the
toolchain, and the pure `iron-domain` crate has no I/O to mock:

```bash
cargo test   # iron-domain, iron-edge, iron-server — zero configuration
npm test     # iron-ui (Vitest) — zero configuration
```

The test *levels* (unit → integration → simulation → field) are specified in
[specs/testing.md](../specs/testing.md). This guide shows the working rhythm.

## The cycle, industrial edition

RED — write a test describing desired behavior. GREEN — minimum code to pass.
REFACTOR — clean up under the test's protection. What changes in this domain
is the *content* of "desired behavior":

- 14.4mA on a 4–20mA / 0–200°C input produces exactly 130.0°C
- deadband 0.5 suppresses 87.3 → 87.4, passes 87.3 → 87.9
- the HIGH alarm triggers at 180.0, not 179.9
- a tag silent for 3 scan cycles becomes UNCERTAIN

These are precise, testable, and consequential. A wrong conversion misleads an
operator; a wrong limit misses a real event. TDD forces "correct" to be
defined before the code that produces it. Unit conversion errors have caused
real industrial accidents — a test pinning 4mA, 20mA, and midpoint is a
permanent record of what correct means.

## Rust: the deadband filter, by the book

```rust
// RED — the test exists before the implementation
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppresses_change_within_deadband() {
        assert!(!should_publish(87.3, 87.4, 0.5));
    }

    #[test]
    fn publishes_on_exact_boundary() {
        assert!(should_publish(87.3, 87.8, 0.5));   // >= — boundary publishes
    }

    #[test]
    fn always_publishes_quality_change() {
        // value identical, quality degraded — must publish
        assert!(filter().should_publish(sample(87.3, Quality::Uncertain)));
    }

    #[test]
    fn publishes_alarm_limit_crossing_inside_deadband() {
        // 179.8 → 180.2: delta 0.4 < deadband 0.5, but HIGH(180.0) is crossed
        let mut f = filter_with(deadband: 0.5, limits: &[180.0]);
        f.should_publish(sample(179.8, Quality::Good));
        assert!(f.should_publish(sample(180.2, Quality::Good)));
    }
}
```

GREEN is a few lines; REFACTOR extracts the `DeadbandFilter` struct — and the
tests, unchanged, prove the refactor safe. Note how the fourth test encodes a
normative rule from [specs/edge-agent.md](../specs/edge-agent.md): the spec
sentence and the test are the same fact in two notations.

## Rust: protocol drivers against mocks

Drivers are where TDD pays most — real-PLC testing is slow, shared, and
sometimes impossible:

```rust
#[test]
fn reads_float32_from_two_consecutive_registers() {
    let mut client = MockModbusClient::new();
    client.set_register(0x1000, 0x4334);   // 180.0 as IEEE 754, high word
    client.set_register(0x1001, 0x0000);   // low word
    assert!((read_float32(&client, 0x1000).unwrap() - 180.0).abs() < 0.001);
}

#[test]
fn returns_error_on_timeout() {
    let client = TimeoutModbusClient::new(Duration::from_millis(100));
    assert!(matches!(read_holding(&client, 0x1000), Err(ModbusError::Timeout)));
}
```

Against real protocol simulators (diagslave, snap7, open62541 — table in
[specs/testing.md](../specs/testing.md)) the same suite runs as integration
tests behind `#[ignore]` gates.

## Rust: the tag engine, quality transitions, RBAC

Server logic is tested the same way — and because `iron-domain` is
synchronous and clock-injected, time-based rules need no sleeping:

```rust
#[test]
fn uncertain_after_missed_scan_cycles() {
    let clock = TestClock::new();
    let mut tag = TagState::new(scan_rate_ms(100), timeout_ms(300), &clock);
    tag.update(sample(87.5, Quality::Good));
    clock.advance_ms(350);
    assert_eq!(tag.quality(), Quality::Uncertain);
}
```

Security behavior is tested explicitly, because "probably works" is not an
acceptable answer to "can a viewer start a pump?":

```rust
#[test]
fn viewer_role_cannot_send_any_command() {
    let svc = CommandService::for_test();
    let res = svc.submit(viewer_user(), "pump_01.start", Value::Bool(true));
    assert!(matches!(res, Err(CommandError::Unauthorized)));
}
```

## Svelte: widgets without a browser engine

Widgets are tested with Vitest + Testing Library — mount, feed a tag update,
assert on DOM. The contract rule "BAD is never shown as a plausible number"
is a test, not a guideline:

```ts
test('BAD quality never renders a numeric value', async () => {
  const { getByTestId } = render(Numeric, { tag: 'reactor_01.temperature' })
  tagStore.apply({ tag: 'reactor_01.temperature', value: 87.5, quality: 'BAD' })
  await tick()
  expect(getByTestId('value').textContent).not.toMatch(/87\.5/)
  expect(getByTestId('value')).toHaveClass('quality-bad')
})
```

## Why this matters here more than anywhere

Without tests, the industrial feedback loop is: write code → deploy to edge →
connect to PLC → squint at screen → fix → redeploy. 20–40 minutes per cycle,
on hardware that production may need. With TDD it is `cargo test`, under ten
seconds, no hardware. On a factory floor, a developer who needs the PLC for
every iteration is a developer who blocks production.

## Layout convention

```
crates/iron-domain/src/deadband.rs      # code + #[cfg(test)] in the same file
crates/iron-server/src/tag_engine.rs    # same convention
crates/iron-server/tests/integration/   # cargo test --features integration
ui/src/widgets/Gauge.svelte
ui/src/widgets/Gauge.test.ts            # mirrors src/ exactly
test/sim/scenarios/                     # iron test --sim
```

One command for the whole logic layer: `cargo test`; add `npm test` for the UI.
