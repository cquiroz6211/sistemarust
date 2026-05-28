//! rpi-controller library crate.
//!
//! Bevy headless ECS application for oven domain control.
//! Replaces the Tokio-first approach with `MinimalPlugins` + `ScheduleRunnerPlugin`.
//!
//! ## Architecture
//!
//! - `components.rs` — Bevy components for oven entity state
//! - `resources.rs` — Bevy resources (queues, config, RNG, index)
//! - `events.rs` — Internal ECS event types
//! - `systems/startup.rs` — `Startup` schedule: spawns ovens, emits `OvenDetected`
//! - `systems/update.rs` — `Update` schedule: command ingestion, routing, responses
//! - `systems/fixed_update.rs` — `FixedUpdate` schedule: thermal, hysteresis, faults
//! - `plugins/oven_controller.rs` — Domain plugin bundling all systems

pub mod components;
pub mod events;
pub mod resources;
pub mod plugins;
pub mod systems;
pub mod bevy_app;

// Re-exports
pub use crate::components::{Enabled, Heating, MaxTemperature, OvenStatus, CurrentTemperature, TargetTemperature, OvenId, SensorRef, OutputRef};
pub use crate::resources::{SimulationConfig, TestRng, InboundProtocolQueue, OutboundProtocolQueue, EmergencyStopActive, ControllerConfig, OvenIndex};
pub use crate::plugins::oven_controller::OvenControllerPlugin;