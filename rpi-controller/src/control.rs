//! Hysteresis-based heating control decision.
//!
//! Implements the 5°C hysteresis band per ADR 002:
//! - `current < target - 5.0` → heating = true
//! - `current >= target` → heating = false
//! - `(target - 5.0) .. target` → maintain previous heating state

use crate::state::HYSTERESIS;

/// Determines the next heating state based on current temperature and hysteresis rules.
///
/// Rules (per ADR 002):
/// - `current < target - HYSTERESIS` → `true` (turn on)
/// - `current >= target` → `false` (turn off)
/// - In between → maintain `previous` state (dead band)
pub fn heating_decision(current: f64, target: f64, enabled: bool, previous_heating: bool) -> bool {
    if !enabled {
        return false;
    }

    let lower_bound = target - HYSTERESIS;

    if current < lower_bound {
        // Below hysteresis band: turn heating ON
        true
    } else if current >= target {
        // At or above target: turn heating OFF
        false
    } else {
        // In the hysteresis band: maintain previous state
        previous_heating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Boundary: exactly target - HYSTERESIS (5.0) ──────────────────────

    #[test]
    fn heating_turns_on_at_hysteresis_lower_bound() {
        // target=150, current=145 (exactly target-5) → heating ON
        let result = heating_decision(145.0, 150.0, true, false);
        assert!(result, "At exactly target - 5°C, heating should be ON");
    }

    #[test]
    fn heating_turns_on_below_hysteresis_bound() {
        // target=150, current=140 → heating ON
        let result = heating_decision(140.0, 150.0, true, false);
        assert!(result, "Below hysteresis band, heating should be ON");
    }

    // ── Boundary: exactly target ───────────────────────────────────────────

    #[test]
    fn heating_turns_off_at_target() {
        // target=150, current=150 (exactly target) → heating OFF
        let result = heating_decision(150.0, 150.0, true, true);
        assert!(!result, "At exactly target, heating should be OFF");
    }

    #[test]
    fn heating_turns_off_above_target() {
        // target=150, current=160 → heating OFF
        let result = heating_decision(160.0, 150.0, true, true);
        assert!(!result, "Above target, heating should be OFF");
    }

    // ── Hysteresis band: target - 5.0 .. target ───────────────────────────

    #[test]
    fn hysteresis_maintains_on_when_was_on() {
        // target=150, current=147, previous heating=true → maintain ON
        let result = heating_decision(147.0, 150.0, true, true);
        assert!(result, "In band with previous=true, should maintain ON");
    }

    #[test]
    fn hysteresis_maintains_off_when_was_off() {
        // target=150, current=147, previous heating=false → maintain OFF
        let result = heating_decision(147.0, 150.0, true, false);
        assert!(!result, "In band with previous=false, should maintain OFF");
    }

    #[test]
    fn hysteresis_band_147_triggers_on() {
        // target=150, current=147, previously OFF → should remain OFF
        // (this is a boundary check: 147 is in the band)
        let result = heating_decision(147.0, 150.0, true, false);
        assert!(!result);
    }

    #[test]
    fn hysteresis_band_148_maintains() {
        // target=150, current=148 → in band
        let result = heating_decision(148.0, 150.0, true, true);
        assert!(result, "In band, should maintain previous=true");
    }

    // ── Disabled oven ─────────────────────────────────────────────────────

    #[test]
    fn disabled_oven_always_off() {
        // Even if current is way below target
        let result = heating_decision(100.0, 150.0, false, true);
        assert!(!result, "Disabled oven must have heating OFF regardless of temp");
    }

    #[test]
    fn disabled_oven_ignores_previous() {
        // Previous heating doesn't matter when disabled
        let result = heating_decision(100.0, 150.0, false, true);
        assert!(!result);
    }

    // ── Edge cases ────────────────────────────────────────────────────────

    #[test]
    fn heating_stays_on_in_band_144() {
        // target=150, current=144 is above lower_bound (145) but below target
        // 144 IS in [145, 150) band
        let result = heating_decision(144.0, 150.0, true, true);
        assert!(result, "144 is in hysteresis band (145-150), maintaining true");
    }

    #[test]
    fn just_below_hysteresis() {
        // target=150, current=144.9 → below band
        let result = heating_decision(144.9, 150.0, true, false);
        assert!(result, "Below lower bound, should turn on");
    }

    #[test]
    fn transition_from_heating_to_target() {
        // Scenario: oven heating up, crosses from band to target
        // First tick at 149 (band, maintain ON)
        let in_band = heating_decision(149.0, 150.0, true, true);
        assert!(in_band);
        // Next tick at 150 (target), should turn OFF
        let at_target = heating_decision(150.0, 150.0, true, true);
        assert!(!at_target);
    }
}