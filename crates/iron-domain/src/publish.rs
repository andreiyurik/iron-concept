//! The edge publish filter (`docs/specs/edge-agent.md`, "Deadband filtering").

use crate::Quality;

/// What was last published for a tag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Published {
    /// Time of the publish, in milliseconds on any monotonic scale.
    pub t_ms: i64,
    /// The published value.
    pub value: f64,
    /// The published quality.
    pub quality: Quality,
}

/// Decides whether a new scan result is published.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishFilter {
    /// Minimum change that publishes.
    pub deadband: f64,
    /// Heartbeat: the longest allowed silence, in milliseconds.
    pub max_interval_ms: i64,
    /// Every alarm limit defined for the tag.
    pub alarm_limits: Vec<f64>,
    /// The last published sample; `None` before the first publish.
    pub last: Option<Published>,
}

impl PublishFilter {
    /// Whether the sample must be published. Does not record it.
    ///
    /// Publishes when the value moved by at least the deadband (REQ-EDG-001),
    /// the quality changed (REQ-EDG-002), an alarm limit lies between the
    /// last published value and this one (REQ-EDG-003), or the heartbeat is
    /// due (REQ-EDG-004). The first sample always publishes.
    #[must_use]
    pub fn should_publish(&self, t_ms: i64, value: f64, quality: Quality) -> bool {
        let Some(last) = self.last else { return true };
        let value_moved = (value - last.value).abs() >= self.deadband;
        let quality_moved = quality != last.quality;
        let limit_crossed = self
            .alarm_limits
            .iter()
            .any(|&l| crossed(last.value, value, l));
        let heartbeat_due = t_ms - last.t_ms >= self.max_interval_ms;
        value_moved || quality_moved || limit_crossed || heartbeat_due
    }

    /// Runs the decision and, if it publishes, records the sample as the new
    /// last published one.
    pub fn offer(&mut self, t_ms: i64, value: f64, quality: Quality) -> bool {
        let publish = self.should_publish(t_ms, value, quality);
        if publish {
            self.last = Some(Published {
                t_ms,
                value,
                quality,
            });
        }
        publish
    }
}

/// True when moving from `a` to `b` reaches, leaves, or passes `limit`.
/// Touching the limit counts in both directions, so the filter never hides a
/// transition whichever side an alarm treats as inclusive.
fn crossed(a: f64, b: f64, limit: f64) -> bool {
    (a < limit) != (b < limit) || (a <= limit) != (b <= limit)
}
