//! `Update` schedule system — UI command dispatch.
//!
//! Reads `UiIntent` and translates it into protocol commands using the
//! existing authoring functions. Resets `UiIntent` after processing.

use bevy::prelude::ResMut;

use protocol::RequestScope;

use crate::resources::{OutboundProtocolQueue, UiIntent};
use crate::systems::commands::{
    author_emergency_stop_command, author_request_status_command,
    author_set_oven_enabled_command, author_set_target_temperature_command,
};

pub fn ui_command_dispatch(
    mut intent: ResMut<UiIntent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    if let Some((oven_id, enabled)) = intent.set_enabled.take() {
        author_set_oven_enabled_command(oven_id, enabled, &mut outbound);
    }

    if let Some((oven_id, temp)) = intent.set_temperature.take() {
        author_set_target_temperature_command(oven_id, temp, &mut outbound);
    }

    if let Some(maybe_oven_id) = intent.request_status.take() {
        let scope = if maybe_oven_id.is_some() {
            RequestScope::Single
        } else {
            RequestScope::All
        };
        author_request_status_command(maybe_oven_id, scope, &mut outbound);
    }

    if intent.emergency_stop {
        intent.emergency_stop = false;
        author_emergency_stop_command("Activado desde UI".to_string(), &mut outbound);
    }
}
