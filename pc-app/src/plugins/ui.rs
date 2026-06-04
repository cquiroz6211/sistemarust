//! UiPlugin — registers all UI systems and resources.
//!
//! Schedule layout:
//! - `Startup`: `spawn_ui_camera`
//! - `Update`: `log_capture_events` (after `ingest_inbound_protocol`),
//!             `ui_command_dispatch`, `connection_monitor`
//! - `EguiPrimaryContextPass`: `ui_render`

use bevy::prelude::*;

use crate::resources::{BulkSelection, BulkValidation, ConnectionState, EmergencyStopConfirm, EventLog, OvenEditStates, TemperatureValidation, UiIntent};
use crate::systems::ui::connection::connection_monitor;
use crate::systems::ui::dispatch::ui_command_dispatch;
use crate::systems::ui::log_capture::log_capture_events;
use crate::systems::ui::render::ui_render;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Resources
        app.insert_resource(EventLog::default());
        app.insert_resource(ConnectionState::Disconnected);
        app.insert_resource(UiIntent::default());
        app.insert_resource(OvenEditStates::default());
        app.insert_resource(EmergencyStopConfirm::default());
        app.insert_resource(TemperatureValidation::default());
        app.insert_resource(BulkSelection::default());
        app.insert_resource(BulkValidation::default());

        // Startup: spawn camera for bevy_egui
        app.add_systems(Startup, spawn_ui_camera);

        // Update: capture events for log (after ingest so events are emitted)
        app.add_systems(
            Update,
            log_capture_events.after(crate::systems::ingest::ingest_inbound_protocol),
        );

        // Update: dispatch commands from UI
        app.add_systems(Update, ui_command_dispatch);

        // Update: monitor connection state
        app.add_systems(Update, connection_monitor);

        // Update: render UI (after egui BeginPass runs in PreUpdate)
        app.add_systems(Update, ui_render);
    }
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
