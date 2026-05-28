//! Bevy components for the oven domain.
//!
//! Each simulated oven is a Bevy entity with these components.

use bevy::prelude::Component;
use protocol::OvenState;

/// Unique identifier for an oven entity.
#[derive(Debug, Clone, Component)]
pub struct OvenId(pub String);

/// Sensor label associated with this oven.
#[derive(Debug, Clone, Component)]
pub struct SensorRef(pub String);

/// Actuator/relay label associated with this oven.
#[derive(Debug, Clone, Component)]
pub struct OutputRef(pub String);

/// Current simulated temperature in °C.
#[derive(Debug, Clone, Component)]
pub struct CurrentTemperature(pub f64);

/// Desired target temperature in °C.
#[derive(Debug, Clone, Component)]
pub struct TargetTemperature(pub f64);

/// Safety temperature limit in °C.
#[derive(Debug, Clone, Component)]
pub struct MaxTemperature(pub f64);

/// Whether the oven is logically enabled to operate.
#[derive(Debug, Clone, Component)]
pub struct Enabled(pub bool);

/// Whether the heating element is currently active.
#[derive(Debug, Clone, Component)]
pub struct Heating(pub bool);

/// Composite operational state of the oven.
#[derive(Debug, Clone, Component)]
pub struct OvenStatus(pub OvenState);