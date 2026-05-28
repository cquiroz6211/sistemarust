//! Command authoring functions.
//!
//! Each function constructs the correct `EventEnvelope` with the proper `Message`
//! variant and pushes it to `OutboundProtocolQueue`. In v1 these are triggered
//! directly in tests; in v1.1 they'll be registered as Bevy systems reading
//! from a UI intent resource.

use protocol::{
    EmergencyStopPayload, EventEnvelope, Message, RequestScope, RequestStatusPayload,
    SetOvenEnabledPayload, SetTargetTemperaturePayload,
};

use crate::resources::OutboundProtocolQueue;

const SOURCE: &str = "pc-app";
const TARGET: &str = "rpi-controller";

/// Enqueues a `SetTargetTemperature` command for a specific oven.
pub fn author_set_target_temperature_command(
    oven_id: String,
    target_celsius: f64,
    outbound: &mut OutboundProtocolQueue,
) {
    let envelope = EventEnvelope::new(
        SOURCE,
        TARGET,
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id,
            target_celsius,
        }),
    );
    outbound.0.push(envelope);
}

/// Enqueues a `SetOvenEnabled` command for a specific oven.
pub fn author_set_oven_enabled_command(
    oven_id: String,
    enabled: bool,
    outbound: &mut OutboundProtocolQueue,
) {
    let envelope = EventEnvelope::new(
        SOURCE,
        TARGET,
        Message::SetOvenEnabled(SetOvenEnabledPayload { oven_id, enabled }),
    );
    outbound.0.push(envelope);
}

/// Enqueues a `RequestStatus` command for a single oven or all ovens.
pub fn author_request_status_command(
    oven_id: Option<String>,
    scope: RequestScope,
    outbound: &mut OutboundProtocolQueue,
) {
    let envelope = EventEnvelope::new(
        SOURCE,
        TARGET,
        Message::RequestStatus(RequestStatusPayload { oven_id, scope }),
    );
    outbound.0.push(envelope);
}

/// Enqueues an `EmergencyStop` command with a descriptive reason.
pub fn author_emergency_stop_command(
    reason: String,
    outbound: &mut OutboundProtocolQueue,
) {
    let envelope = EventEnvelope::new(
        SOURCE,
        TARGET,
        Message::EmergencyStop(EmergencyStopPayload { reason }),
    );
    outbound.0.push(envelope);
}
