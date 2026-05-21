//! Pure command validation functions.
//!
//! Each validator returns `Result<(), FaultCode>` — `Ok(())` means valid,
//! `Err(FaultCode)` means rejected with that reason.

use protocol::{FaultCode, OvenState};

use crate::state::Oven;

/// Validates that a target temperature is within safe bounds for the given oven.
pub fn validate_temperature_range(target: f64, max_celsius: f64) -> Result<(), FaultCode> {
    if target < 0.0 {
        return Err(FaultCode::InvalidTemperature);
    }
    if target > max_celsius {
        return Err(FaultCode::InvalidTemperature);
    }
    Ok(())
}

/// Validates SetTargetTemperature command against the target oven state.
pub fn validate_set_target(oven: &Oven, target: f64) -> Result<(), FaultCode> {
    validate_temperature_range(target, oven.max_celsius)
}

/// Validates SetOvenEnabled command against the target oven state.
pub fn validate_set_enabled(oven: &Oven) -> Result<(), FaultCode> {
    // Ovens in Faulted or EmergencyStopped state cannot be re-enabled via command.
    // They require manual reset or emergency recovery.
    match oven.state {
        OvenState::Faulted | OvenState::EmergencyStopped => {
            Err(FaultCode::OutputUnavailable)
        }
        _ => Ok(()),
    }
}

/// Checks if emergency stop is currently active (guards all non-emergency commands).
pub fn is_emergency_stop_active(emergency_active: bool) -> Result<(), FaultCode> {
    if emergency_active {
        Err(FaultCode::EmergencyStopActive)
    } else {
        Ok(())
    }
}

/// Guard: rejects any non-EmergencyStop command when emergency stop is active.
pub fn guard_emergency_stop<T>(
    emergency_active: bool,
    _cmd: &T,
) -> Result<(), FaultCode> {
    is_emergency_stop_active(emergency_active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Oven;

    // ── validate_temperature_range ──────────────────────────────────────────

    #[test]
    fn validate_temperature_range_negative_rejected() {
        let result = validate_temperature_range(-10.0, 300.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::InvalidTemperature);
    }

    #[test]
    fn validate_temperature_range_exceeds_max_rejected() {
        let result = validate_temperature_range(350.0, 300.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::InvalidTemperature);
    }

    #[test]
    fn validate_temperature_range_at_zero_accepted() {
        let result = validate_temperature_range(0.0, 300.0);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_temperature_range_at_max_accepted() {
        let result = validate_temperature_range(300.0, 300.0);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_temperature_range_below_max_accepted() {
        let result = validate_temperature_range(150.0, 300.0);
        assert!(result.is_ok());
    }

    // ── validate_set_target ────────────────────────────────────────────────

    #[test]
    fn validate_set_target_valid() {
        let oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        assert!(validate_set_target(&oven, 150.0).is_ok());
    }

    #[test]
    fn validate_set_target_negative_rejected() {
        let oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        let result = validate_set_target(&oven, -5.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::InvalidTemperature);
    }

    #[test]
    fn validate_set_target_exceeds_max_rejected() {
        let oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        let result = validate_set_target(&oven, 350.0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::InvalidTemperature);
    }

    // ── validate_set_enabled ───────────────────────────────────────────────

    #[test]
    fn validate_set_enabled_normal_oven_accepted() {
        let oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        assert!(validate_set_enabled(&oven).is_ok());
    }

    #[test]
    fn validate_set_enabled_faulted_oven_rejected() {
        let mut oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        oven.state = OvenState::Faulted;
        let result = validate_set_enabled(&oven);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::OutputUnavailable);
    }

    #[test]
    fn validate_set_enabled_emergency_stopped_oven_rejected() {
        let mut oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        oven.state = OvenState::EmergencyStopped;
        let result = validate_set_enabled(&oven);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::OutputUnavailable);
    }

    #[test]
    fn validate_set_enabled_idle_oven_accepted() {
        let mut oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        oven.state = OvenState::Idle;
        assert!(validate_set_enabled(&oven).is_ok());
    }

    #[test]
    fn validate_set_enabled_heating_oven_accepted() {
        let mut oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        oven.state = OvenState::Heating;
        assert!(validate_set_enabled(&oven).is_ok());
    }

    // ── is_emergency_stop_active ───────────────────────────────────────────

    #[test]
    fn is_emergency_stop_active_inactive_ok() {
        assert!(is_emergency_stop_active(false).is_ok());
    }

    #[test]
    fn is_emergency_stop_active_active_rejected() {
        let result = is_emergency_stop_active(true);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::EmergencyStopActive);
    }

    // ── guard_emergency_stop ───────────────────────────────────────────────

    #[test]
    fn guard_emergency_stop_allows_normal() {
        assert!(guard_emergency_stop::<i32>(false, &0).is_ok());
    }

    #[test]
    fn guard_emergency_stop_blocks_when_active() {
        let result = guard_emergency_stop::<i32>(true, &0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FaultCode::EmergencyStopActive);
    }
}