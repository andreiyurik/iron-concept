//! Raw → engineering conversion (`docs/specs/tag-model.md`, "Scaling").

use crate::Quality;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scaling {
    pub raw: (f64, f64),
    pub engineering: (f64, f64),
}

#[must_use]
pub fn convert(raw: f64, scaling: Option<Scaling>, range: Option<(f64, f64)>) -> (f64, Quality) {
    if !raw.is_finite() {
        return (raw, Quality::Bad);
    }
    match scaling {
        Some(s) => {
            let (r0, r1) = s.raw;
            let (e0, e1) = s.engineering;
            let (lo, hi) = if r0 <= r1 { (r0, r1) } else { (r1, r0) };
            let clamped = raw.clamp(lo, hi);
            let quality = if clamped == raw {
                Quality::Good
            } else {
                Quality::Uncertain
            };
            let span = r1 - r0;
            if span == 0.0 {
                return (e0, Quality::Bad);
            }
            (e0 + (clamped - r0) / span * (e1 - e0), quality)
        }
        None => {
            let inside = range.is_none_or(|(lo, hi)| raw >= lo && raw <= hi);
            (
                raw,
                if inside {
                    Quality::Good
                } else {
                    Quality::Uncertain
                },
            )
        }
    }
}
