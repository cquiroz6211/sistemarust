//! Bevy components for the PC-side oven read model.
//!
//! Each detected oven is represented as a Bevy entity carrying these components.
//! The PC is a read-model consumer — it displays and requests; the RPi validates
//! and decides.

use bevy::prelude::Component;
use protocol::{FaultCode, OvenState, Severity};

/// Unique identifier for an oven entity, matching the protocol's `oven_id`.
#[derive(Debug, Clone, Component)]
pub struct OvenId(pub String);

/// Sensor label associated with this oven.
#[derive(Debug, Clone, Component)]
pub struct SensorRef(pub String);

/// Actuator/relay label associated with this oven.
#[derive(Debug, Clone, Component)]
pub struct OutputRef(pub String);

/// Current temperature reading in °C.
#[derive(Debug, Clone, Component, Default)]
pub struct CurrentTemperature(pub f64);

/// Desired target temperature in °C.
#[derive(Debug, Clone, Component, Default)]
pub struct TargetTemperature(pub f64);

/// Safety temperature limit in °C.
#[derive(Debug, Clone, Component, Default)]
pub struct MaxTemperature(pub f64);

/// Whether the oven is logically enabled to operate.
#[derive(Debug, Clone, Component, Default)]
pub struct Enabled(pub bool);

/// Whether the heating element is currently active.
#[derive(Debug, Clone, Component, Default)]
pub struct Heating(pub bool);

/// Composite operational state of the oven.
/// Reuses `protocol::OvenState` — no PC-specific enum needed.
#[derive(Debug, Clone, Component)]
pub struct OvenStatus(pub OvenState);

/// Fault information stored when a `FaultRaised` event is received.
/// Persists across subsequent `OvenStatusUpdated` events.
#[derive(Debug, Clone, Component, Default)]
pub struct FaultState(pub Option<FaultInfo>);

/// Detailed fault data: code, severity, and human-readable message.
#[derive(Debug, Clone)]
pub struct FaultInfo {
    pub fault_code: FaultCode,
    pub severity: Severity,
    pub message: String,
}

/// Records the last command result (accepted or rejected) for this oven.
#[derive(Debug, Clone, Component, Default)]
pub struct LastCommandResult(pub Option<CommandResult>);

/// Outcome of a command, as reported by the RPi controller.
#[derive(Debug, Clone)]
pub enum CommandResult {
    Accepted {
        accepted_type: String,
        message: String,
    },
    Rejected {
        rejected_type: String,
        reason: FaultCode,
        message: String,
    },
}
