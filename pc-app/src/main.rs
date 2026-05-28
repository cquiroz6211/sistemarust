//! pc-app entry point — headless Bevy ECS application.
//!
//! Usage:
//! ```
//! cargo run --package pc-app
//! ```
//!
//! The app runs headlessly with `MinimalPlugins` + `ScheduleRunnerPlugin` at 50ms.
//! No real transport in v1 — queues are in-memory Resources.

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::{App, MinimalPlugins, PluginGroup};

use pc_app::PcAppPlugin;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(50),
        )))
        .add_plugins(PcAppPlugin)
        .run();
}
