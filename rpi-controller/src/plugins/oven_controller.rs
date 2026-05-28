//! OvenControllerPlugin — bundles all domain systems, resources, and event types.

use bevy::prelude::*;
use std::time::Duration;

use crate::events;
use crate::resources;
use crate::systems::{fixed_update, startup, update};

/// Main plugin for the oven controller domain.
///
/// Registers:
/// - All internal event types via `add_event`
/// - All resources via `insert_resource`
/// - All systems in correct schedule order
pub struct OvenControllerPlugin;

impl Plugin for OvenControllerPlugin {
    fn build(&self, app: &mut App) {
        // ── Internal event types ────────────────────────────────────────────
        app.add_event::<events::CommandReceivedEvent>();
        app.add_event::<events::CommandAcceptedEvent>();
        app.add_event::<events::CommandRejectedEvent>();
        app.add_event::<events::OvenDetectedEvent>();
        app.add_event::<events::FaultDetectedEvent>();
        app.add_event::<events::StatusPublishEvent>();

        // ── Resources ───────────────────────────────────────────────────────
        app.insert_resource(resources::EmergencyStopActive::default());
        app.insert_resource(resources::SimulationConfig::default());
        app.insert_resource(resources::InboundProtocolQueue::default());
        app.insert_resource(resources::OutboundProtocolQueue::default());
        app.insert_resource(resources::ControllerConfig::default());
        app.insert_resource(resources::TestRng::default());
        app.insert_resource(resources::OvenIndex::default());
        app.insert_resource(resources::LastStatusPublishTime(Duration::ZERO));

        // ── Startup schedule ─────────────────────────────────────────────────
        app.add_systems(Startup, startup::spawn_ovens);
        app.add_systems(Startup, startup::emit_oven_detected.after(startup::spawn_ovens));

        // ── Update schedule ─────────────────────────────────────────────────
        app.add_systems(Update, update::ingest_commands);
        app.add_systems(Update, update::route_command.after(update::ingest_commands));
        app.add_systems(Update, update::emit_protocol_responses.after(update::route_command));

        // ── FixedUpdate schedule ────────────────────────────────────────────
        app.add_systems(
            FixedUpdate,
            (
                fixed_update::thermal_drift,
                fixed_update::hysteresis_control,
                fixed_update::derive_oven_status,
                fixed_update::fault_detection,
                fixed_update::emit_fault_events,
            ),
        );

        app.add_systems(
            FixedUpdate,
            (
                fixed_update::periodic_status,
                fixed_update::emit_status_events,
            ),
        );
    }
}