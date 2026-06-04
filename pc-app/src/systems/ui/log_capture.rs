//! `Update` schedule system — log capture.
//!
//! Reads inbound ECS events and outbound protocol queue to fill `EventLog`.
//! Runs after `ingest_inbound_protocol` so events are already emitted.

use bevy::prelude::{EventReader, Res, ResMut};

use protocol::Message;

use crate::events::{
    CommandAcceptedReceived, CommandRejectedReceived, FaultReceived, OvenDiscovered,
    OvenStatusReceived,
};
use crate::resources::{EventLog, LogDirection, LogEntry, OutboundProtocolQueue};

pub fn log_capture_events(
    mut ev_discovered: EventReader<OvenDiscovered>,
    mut ev_status: EventReader<OvenStatusReceived>,
    mut ev_fault: EventReader<FaultReceived>,
    mut ev_accepted: EventReader<CommandAcceptedReceived>,
    mut ev_rejected: EventReader<CommandRejectedReceived>,
    mut log: ResMut<EventLog>,
    outbound: Res<OutboundProtocolQueue>,
) {
    for ev in ev_discovered.read() {
        log.push(LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::In,
            message_type: "OvenDetected".into(),
            summary: format!(
                "{} (sensor: {}, output: {})",
                ev.oven_id, ev.sensor_ref, ev.output_ref
            ),
        });
    }

    for ev in ev_status.read() {
        log.push(LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::In,
            message_type: "OvenStatusUpdated".into(),
            summary: format!("{}: {:.1} °C", ev.oven_id, ev.current_celsius),
        });
    }

    for ev in ev_fault.read() {
        let oven_str = ev
            .oven_id
            .as_deref()
            .unwrap_or("global");
        log.push(LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::In,
            message_type: "FaultRaised".into(),
            summary: format!("{}: {:?} ({})", oven_str, ev.fault_code, ev.message),
        });
    }

    for ev in ev_accepted.read() {
        log.push(LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::In,
            message_type: "CommandAccepted".into(),
            summary: format!(
                "{}: {}",
                ev.oven_id.as_deref().unwrap_or("global"),
                ev.message
            ),
        });
    }

    for ev in ev_rejected.read() {
        log.push(LogEntry {
            timestamp: format_timestamp(),
            direction: LogDirection::In,
            message_type: "CommandRejected".into(),
            summary: format!(
                "{}: {:?} — {}",
                ev.oven_id.as_deref().unwrap_or("global"),
                ev.reason,
                ev.message
            ),
        });
    }

    // Capture pending outbound commands (before bridge drains the queue)
    for envelope in &outbound.0 {
        let entry = match &envelope.payload {
            Message::SetTargetTemperature(p) => Some(LogEntry {
                timestamp: format_timestamp(),
                direction: LogDirection::Out,
                message_type: "SetTargetTemperature".into(),
                summary: format!("{}: {:.1} °C", p.oven_id, p.target_celsius),
            }),
            Message::SetOvenEnabled(p) => Some(LogEntry {
                timestamp: format_timestamp(),
                direction: LogDirection::Out,
                message_type: "SetOvenEnabled".into(),
                summary: format!("{}: {}", p.oven_id, p.enabled),
            }),
            Message::RequestStatus(p) => Some(LogEntry {
                timestamp: format_timestamp(),
                direction: LogDirection::Out,
                message_type: "RequestStatus".into(),
                summary: match &p.oven_id {
                    Some(id) => format!("{} (single)", id),
                    None => "all ovens".into(),
                },
            }),
            Message::EmergencyStop(p) => Some(LogEntry {
                timestamp: format_timestamp(),
                direction: LogDirection::Out,
                message_type: "EmergencyStop".into(),
                summary: p.reason.clone(),
            }),
            // Inbound event variants should not appear in outbound queue
            _ => None,
        };
        if let Some(entry) = entry {
            log.push(entry);
        }
    }
}

/// Format current time as HH:MM:SS without chrono dependency.
fn format_timestamp() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
