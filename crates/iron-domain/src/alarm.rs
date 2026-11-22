//! Alarm detection (`docs/specs/alarm-engine.md`, "Detection semantics").

use crate::{Quality, Value};

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
    /// The direction of a numeric alarm comes from its name (REQ-ALM-004).
    #[must_use]
    pub fn level(name: &str, limit: f64) -> Option<Condition> {
        match name {
            "high" | "high_high" => Some(Condition::High(limit)),
            "low" | "low_low" => Some(Condition::Low(limit)),
            _ => None,
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
    pub fn update(
        &mut self,
        def: &AlarmDef,
        t_ms: i64,
        value: Value,
        quality: Quality,
    ) -> Option<Transition> {
        // REQ-ALM-008: a BAD sample changes nothing and cancels a delay.
        if quality == Quality::Bad {
            self.pending_since = None;
            return None;
        }
        let wanted = self.wanted(def, value)?;
        if wanted == self.active {
            self.pending_since = None;
            return None;
        }
        self.pending_since.get_or_insert(t_ms);
        self.poll(def, t_ms)
    }

    /// Completes a delay that has elapsed by `t_ms` (REQ-ALM-007).
    pub fn poll(&mut self, def: &AlarmDef, t_ms: i64) -> Option<Transition> {
        let since = self.pending_since?;
        let delay = if self.active {
            def.off_delay_ms
        } else {
            def.on_delay_ms
        };
        if t_ms - since < delay {
            return None;
        }
        self.active = !self.active;
        self.pending_since = None;
        Some(Transition {
            active: self.active,
            t_ms: since + delay,
        })
    }

    // REQ-ALM-005, REQ-ALM-006.
    fn wanted(&self, def: &AlarmDef, value: Value) -> Option<bool> {
        let band = def.deadband;
        Some(match (def.condition, value) {
            (Condition::High(limit), Value::Number(v)) if self.active && band > 0.0 => {
                v > limit - band
            }
            (Condition::High(limit), Value::Number(v)) => v >= limit,
            (Condition::Low(limit), Value::Number(v)) if self.active && band > 0.0 => {
                v < limit + band
            }
            (Condition::Low(limit), Value::Number(v)) => v <= limit,
            (Condition::State(state), Value::Bool(b)) => b == state,
            _ => return None,
        })
    }
}
