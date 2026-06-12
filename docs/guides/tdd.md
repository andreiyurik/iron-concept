---
status: draft
type: guide
---

# Test-Driven Development in IRON

TDD is not a methodology imposed on IRON from outside. It is the natural
consequence of choosing Rust and Elixir — two languages where testing is built
into the toolchain:

```bash
cargo test   # iron-core — zero configuration
mix test     # iron-web  — zero configuration
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

## Elixir: GenServers, LiveView, RBAC

```elixir
describe "quality transitions" do
  test "UNCERTAIN after missed scan cycles" do
    {:ok, pid} = TagServer.start_link(tag: "t", scan_rate: 100, timeout: 300)
    TagServer.update(pid, value: 87.5, quality: :good)
    Process.sleep(350)
    assert TagServer.get_state(pid).quality == :uncertain
  end
end
```

LiveView TDD needs no browser — `Phoenix.LiveViewTest` mounts the view,
drives clicks, asserts on DOM. And security behavior is tested explicitly,
because "probably works" is not an acceptable answer to "can a viewer start
a pump?":

```elixir
test "viewer role cannot send any command" do
  assert {:error, :unauthorized} =
    Commands.send(viewer_user(), command: "pump_01.start", value: true)
end
```

## Why this matters here more than anywhere

Without tests, the industrial feedback loop is: write code → deploy to edge →
connect to PLC → squint at screen → fix → redeploy. 20–40 minutes per cycle,
on hardware that production may need. With TDD it is `cargo test`, under ten
seconds, no hardware. On a factory floor, a developer who needs the PLC for
every iteration is a developer who blocks production.

## Layout convention

```
iron-core/src/deadband.rs        # code + #[cfg(test)] in the same file
iron-web/lib/iron_web/tag_server.ex
iron-web/test/iron_web/tag_server_test.exs    # mirrors lib/ exactly
test/integration/                # mix test --include integration
test/sim/scenarios/              # iron test --sim
```

One command for the whole logic layer: `cargo test && mix test`.
