//! The edge publish filter (`docs/specs/edge-agent.md`).

use crate::Quality;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Published {
    pub t_ms: i64,
    pub value: f64,
    pub quality: Quality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublishFilter {
    pub deadband: f64,
    pub max_interval_ms: i64,
    pub alarm_limits: Vec<f64>,
    pub last: Option<Published>,
}

impl PublishFilter {
    // REQ-EDG-001..004. The first sample always publishes.
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

fn crossed(a: f64, b: f64, limit: f64) -> bool {
    (a < limit) != (b < limit) || (a <= limit) != (b <= limit)
}
