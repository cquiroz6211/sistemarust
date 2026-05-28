//! PcAppPlugin — bundles all domain systems, resources, and event types.
//!
//! Schedule layout:
//! - `Update`: `ingest_inbound_protocol` (drains inbound queue, dispatches events)
//! - `FixedUpdate`: `apply_oven_detected`, `apply_oven_status_updated`,
//!   `apply_fault_raised`, `record_command_result` (state mutation)
//!
//! Command authoring systems are standalone functions (not registered in a schedule)
//! because they are triggered directly in v1 tests. In v1.1 they'll read from
//! a UI intent resource and be registered in `Update`.

use bevy::prelude::*;

use crate::events;
use crate::resources;
use crate::systems;

pub struct PcAppPlugin;

impl Plugin for PcAppPlugin {
    fn build(&self, app: &mut App) {
        // ── Internal event types ────────────────────────────────────────────
        app.add_event::<events::OvenDiscovered>();
        app.add_event::<events::OvenStatusReceived>();
        app.add_event::<events::FaultReceived>();
        app.add_event::<events::CommandAcceptedReceived>();
        app.add_event::<events::CommandRejectedReceived>();

        // ── Resources ───────────────────────────────────────────────────────
        app.insert_resource(resources::InboundProtocolQueue::default());
        app.insert_resource(resources::OutboundProtocolQueue::default());
        app.insert_resource(resources::OvenIndex::default());
        app.insert_resource(resources::GlobalFault::default());

        // ── Update schedule: protocol I/O ──────────────────────────────────
        app.add_systems(Update, systems::ingest::ingest_inbound_protocol);

        // ── FixedUpdate schedule: state mutation ───────────────────────────
        app.add_systems(FixedUpdate, systems::state::apply_oven_detected);
        app.add_systems(FixedUpdate, systems::state::apply_oven_status_updated);
        app.add_systems(FixedUpdate, systems::state::apply_fault_raised);
        app.add_systems(FixedUpdate, systems::state::record_command_result);
    }
}
