//! Alarm detection (`docs/specs/alarm-engine.md`, "Detection semantics").

use crate::{Quality, Value};

/// Numeric alarm names, in the order their limits must increase.
pub const LEVELS: [&str; 4] = ["low_low", "low", "high", "high_high"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Condition {
    High(f64),
    Low(f64),
    State(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmDef {
    pub condition: Condition,
    pub deadband: f64,
    pub on_delay_ms: i64,
    pub off_delay_ms: i64,
}

impl AlarmDef {
    // REQ-ALM-004
    #[must_use]
    pub fn level(name: &str, limit: f64) -> Option<Condition> {
        match LEVELS.iter().position(|l| *l == name)? {
            0 | 1 => Some(Condition::Low(limit)),
            _ => Some(Condition::High(limit)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub active: bool,
    pub t_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Detector {
    pub active: bool,
    pending_since: Option<i64>,
}

impl Detector {
    /// Up to two transitions: a delay that ran out before this sample is
    /// honoured first (REQ-ALM-007), then the sample itself is judged.
    pub fn update(&mut self, def: &AlarmDef, t_ms: i64, value: Value, quality: Quality) -> [Option<Transition>; 2] {
        let done = self.poll(def, t_ms);
        // REQ-ALM-008: an unreadable sample changes nothing and cancels a delay.
        let unreadable = quality == Quality::Bad || matches!(value, Value::Number(n) if !n.is_finite());
        match self.wanted(def, value).filter(|_| !unreadable) {
            Some(wanted) if wanted != self.active => {
                self.pending_since.get_or_insert(t_ms);
                [done, self.poll(def, t_ms)]
            }
            _ => {
                self.pending_since = None;
                [done, None]
            }
        }
    }

    // REQ-ALM-007
    pub fn poll(&mut self, def: &AlarmDef, t_ms: i64) -> Option<Transition> {
        let since = self.pending_since?;
        let delay = if self.active { def.off_delay_ms } else { def.on_delay_ms };
        if t_ms - since < delay {
            return None;
        }
        self.active = !self.active;
        self.pending_since = None;
        Some(Transition { active: self.active, t_ms: since + delay })
    }

    // REQ-ALM-005, REQ-ALM-006. With no deadband the clear threshold is the
    // raise threshold: a value exactly on the limit stays in alarm.
    fn wanted(&self, def: &AlarmDef, value: Value) -> Option<bool> {
        let band = def.deadband;
        Some(match (def.condition, value) {
            (Condition::High(limit), Value::Number(v)) if self.active && band > 0.0 => v > limit - band,
            (Condition::High(limit), Value::Number(v)) => v >= limit,
            (Condition::Low(limit), Value::Number(v)) if self.active && band > 0.0 => v < limit + band,
            (Condition::Low(limit), Value::Number(v)) => v <= limit,
            (Condition::State(state), Value::Bool(b)) => b == state,
            _ => return None,
        })
    }
}
