//! Headless Bevy application bootstrap for the rpi-controller.
//!
//! Sets up `MinimalPlugins` + `ScheduleRunnerPlugin::run_loop(FixedInterval(50ms))`
//! and registers the `OvenControllerPlugin`.

use std::time::Duration;

use bevy::prelude::{App, MinimalPlugins, PluginGroup};
use bevy::app::ScheduleRunnerPlugin;

pub use crate::plugins::oven_controller::OvenControllerPlugin;
pub use crate::resources::SimulationConfig;

/// Default tick interval in milliseconds for FixedUpdate schedule.
pub const DEFAULT_TICK_INTERVAL_MS: u64 = 50;

/// Builds and configures the headless Bevy `App` for the oven controller.
pub fn build_app(simulate_count: usize) -> App {
    let mut app = App::new();

    // Insert simulation config with spec values
    app.insert_resource(SimulationConfig {
        heating_drift: 4.0,
        cooling_drift: 1.0,
        room_temp: 20.0,
        noise_amplitude: 2.0,
        hysteresis: 5.0,
    });

    // If simulate count > 0, insert the resource
    if simulate_count > 0 {
        app.insert_resource(crate::resources::SimulateOvenCount(simulate_count));
    }

    // ── Headless runtime ───────────────────────────────────────────────────────
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_millis(DEFAULT_TICK_INTERVAL_MS),
    )));

    // Register all domain plugins
    app.add_plugins(OvenControllerPlugin);

    app
}
