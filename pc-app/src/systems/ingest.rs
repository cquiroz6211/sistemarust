//! `Update` schedule system — inbound protocol ingestion.
//!
//! Drains `InboundProtocolQueue`, deserializes each `EventEnvelope`, and
//! dispatches the corresponding internal ECS event. Malformed or unsupported
//! envelopes are silently discarded. Command variants are ignored (direction guard).

use bevy::prelude::{EventWriter, ResMut};

use protocol::Message;

use crate::events::{
    CommandAcceptedReceived, CommandRejectedReceived, FaultReceived, OvenDiscovered,
    OvenStatusReceived,
};
use crate::resources::InboundProtocolQueue;

/// Drains the `InboundProtocolQueue` and emits typed internal events.
///
/// Only event variants (RPi → PC direction) are dispatched:
/// - `OvenDetected` → `OvenDiscovered`
/// - `OvenStatusUpdated` → `OvenStatusReceived`
/// - `FaultRaised` → `FaultReceived`
/// - `CommandAccepted` → `CommandAcceptedReceived`
/// - `CommandRejected` → `CommandRejectedReceived`
///
/// Command variants (`SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`,
/// `EmergencyStop`) are silently dropped — they should only appear in the
/// *outbound* queue.
pub fn ingest_inbound_protocol(
    mut inbound: ResMut<InboundProtocolQueue>,
    mut ev_discovered: EventWriter<OvenDiscovered>,
    mut ev_status: EventWriter<OvenStatusReceived>,
    mut ev_fault: EventWriter<FaultReceived>,
    mut ev_accepted: EventWriter<CommandAcceptedReceived>,
    mut ev_rejected: EventWriter<CommandRejectedReceived>,
) {
    for envelope in inbound.0.drain(..) {
        match envelope.payload {
            Message::OvenDetected(p) => {
                ev_discovered.send(OvenDiscovered {
                    oven_id: p.oven_id,
                    sensor_ref: p.sensor_ref,
                    output_ref: p.output_ref,
                    max_celsius: p.max_celsius,
                });
            }
            Message::OvenStatusUpdated(p) => {
                ev_status.send(OvenStatusReceived {
                    oven_id: p.oven_id,
                    current_celsius: p.current_celsius,
                    target_celsius: p.target_celsius,
                    enabled: p.enabled,
                    heating: p.heating,
                    output_level: p.output_level,
                    state: p.state,
                });
            }
            Message::FaultRaised(p) => {
                ev_fault.send(FaultReceived {
                    oven_id: p.oven_id,
                    fault_code: p.fault_code,
                    severity: p.severity,
                    message: p.message,
                });
            }
            Message::CommandAccepted(p) => {
                ev_accepted.send(CommandAcceptedReceived {
                    accepted_type: p.accepted_type,
                    oven_id: p.oven_id,
                    message: p.message,
                });
            }
            Message::CommandRejected(p) => {
                ev_rejected.send(CommandRejectedReceived {
                    rejected_type: p.rejected_type,
                    oven_id: p.oven_id,
                    reason: p.reason,
                    message: p.message,
                });
            }
            // Command variants in inbound queue — direction guard: silently ignore
            Message::SetTargetTemperature(_)
            | Message::SetOvenEnabled(_)
            | Message::RequestStatus(_)
            | Message::EmergencyStop(_) => {}
        }
    }
}
