//! Bevy resources for the oven controller domain.

use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::{Entity, Resource};
use rand::rngs::StdRng;

use protocol::EventEnvelope;

/// Whether an emergency stop is currently active.
/// When true, all commands except EmergencyStop are rejected.
#[derive(Debug, Clone, Resource)]
pub struct EmergencyStopActive(pub bool);

impl Default for EmergencyStopActive {
    fn default() -> Self {
        Self(false)
    }
}

/// Simulation constants for temperature dynamics per FixedUpdate tick.
#[derive(Debug, Clone, Resource)]
pub struct SimulationConfig {
    /// °C increase per tick when heating.
    pub heating_drift: f64,
    /// °C decrease per tick when cooling.
    pub cooling_drift: f64,
    /// Minimum ambient temperature (°C). CurrentTemperature never goes below this.
    pub room_temp: f64,
    /// ±°C uniform random noise per tick.
    pub noise_amplitude: f64,
    /// °C hysteresis band width (ADR 002).
    pub hysteresis: f64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            heating_drift: 4.0,
            cooling_drift: 1.0,
            room_temp: 20.0,
            noise_amplitude: 2.0,
            hysteresis: 5.0,
        }
    }
}

/// Queue of incoming protocol messages to be processed in `Update`.
#[derive(Debug, Clone, Resource)]
pub struct InboundProtocolQueue(pub Vec<EventEnvelope>);

impl Default for InboundProtocolQueue {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// Queue of outbound protocol messages to be published.
#[derive(Debug, Clone, Resource)]
pub struct OutboundProtocolQueue(pub Vec<EventEnvelope>);

impl Default for OutboundProtocolQueue {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// General controller configuration.
#[derive(Debug, Clone, Resource)]
pub struct ControllerConfig {
    /// FixedUpdate interval in milliseconds (default 50 ms).
    pub tick_interval_ms: u64,
    /// Interval for periodic status publication in milliseconds.
    pub status_publish_interval_ms: u64,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            tick_interval_ms: 50,
            status_publish_interval_ms: 1000,
        }
    }
}

/// Seeded RNG for deterministic test behavior.
/// In production (no seed), uses `rand::thread_rng()` implicitly.
/// In tests, inject `TestRng(Some(StdRng::seed_from_u64(N)))`.
#[derive(Debug, Clone, Resource)]
pub struct TestRng(pub Option<StdRng>);

impl Default for TestRng {
    fn default() -> Self {
        Self(None)
    }
}

/// Fast lookup index: oven_id → Bevy Entity.
/// Maintains consistency between protocol (oven_id strings) and ECS (Entity).
#[derive(Debug, Clone, Resource, Default)]
pub struct OvenIndex(pub HashMap<String, Entity>);

/// Number of simulated ovens to spawn at startup.
/// Consumed by `spawn_simulated_ovens` in `Startup` schedule.
#[derive(Debug, Clone, Resource)]
pub struct SimulateOvenCount(pub usize);

impl Default for SimulateOvenCount {
    fn default() -> Self {
        Self(0)
    }
}

/// Tracks the last time a periodic status publish was emitted.
/// Stored as `Duration` since FixedUpdate tick start.
#[derive(Debug, Clone, Resource)]
pub struct LastStatusPublishTime(pub Duration);