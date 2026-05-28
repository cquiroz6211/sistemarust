//! Observable logging helpers for the transport layer.
//!
//! Every envelope sent or received is logged to stderr with `[TX]`/`[RX]`
//! prefixes so students can see the message flow in real time.

use protocol::{EventEnvelope, Message};

/// Returns a human-readable name for a `Message` variant.
pub fn message_type_name(msg: &Message) -> &'static str {
    match msg {
        Message::SetTargetTemperature(_) => "SetTargetTemperature",
        Message::SetOvenEnabled(_) => "SetOvenEnabled",
        Message::RequestStatus(_) => "RequestStatus",
        Message::EmergencyStop(_) => "EmergencyStop",
        Message::OvenDetected(_) => "OvenDetected",
        Message::CommandAccepted(_) => "CommandAccepted",
        Message::CommandRejected(_) => "CommandRejected",
        Message::OvenStatusUpdated(_) => "OvenStatusUpdated",
        Message::FaultRaised(_) => "FaultRaised",
    }
}

/// Logs a transmitted envelope with `[TX]` prefix.
pub fn log_tx(envelope: &EventEnvelope) {
    eprintln!(
        "[TX] {} \u{2192} {} | {} | id={}",
        envelope.source,
        envelope.target,
        message_type_name(&envelope.payload),
        envelope.event_id,
    );
}

/// Logs a received envelope with `[RX]` prefix.
pub fn log_rx(envelope: &EventEnvelope) {
    eprintln!(
        "[RX] {} \u{2192} {} | {} | id={}",
        envelope.source,
        envelope.target,
        message_type_name(&envelope.payload),
        envelope.event_id,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_type_name_returns_correct_strings() {
        use protocol::{
            CommandAcceptedPayload, EmergencyStopPayload, FaultCode, FaultRaisedPayload,
            OvenDetectedPayload, OvenState, OvenStatusUpdatedPayload, RequestScope,
            RequestStatusPayload, Severity, SetOvenEnabledPayload, SetTargetTemperaturePayload,
        };

        assert_eq!(
            message_type_name(&Message::SetTargetTemperature(SetTargetTemperaturePayload {
                oven_id: "o".into(),
                target_celsius: 1.0,
            })),
            "SetTargetTemperature"
        );
        assert_eq!(
            message_type_name(&Message::SetOvenEnabled(SetOvenEnabledPayload {
                oven_id: "o".into(),
                enabled: true,
            })),
            "SetOvenEnabled"
        );
        assert_eq!(
            message_type_name(&Message::RequestStatus(RequestStatusPayload {
                oven_id: None,
                scope: RequestScope::All,
            })),
            "RequestStatus"
        );
        assert_eq!(
            message_type_name(&Message::EmergencyStop(EmergencyStopPayload {
                reason: "r".into(),
            })),
            "EmergencyStop"
        );
        assert_eq!(
            message_type_name(&Message::OvenDetected(OvenDetectedPayload {
                oven_id: "o".into(),
                sensor_ref: "s".into(),
                output_ref: "r".into(),
                max_celsius: 300.0,
            })),
            "OvenDetected"
        );
        assert_eq!(
            message_type_name(&Message::CommandAccepted(CommandAcceptedPayload {
                accepted_type: "t".into(),
                oven_id: None,
                message: "m".into(),
            })),
            "CommandAccepted"
        );
        assert_eq!(
            message_type_name(&Message::CommandRejected(protocol::CommandRejectedPayload {
                rejected_type: "t".into(),
                oven_id: None,
                reason: FaultCode::OvenNotFound,
                message: "m".into(),
            })),
            "CommandRejected"
        );
        assert_eq!(
            message_type_name(&Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                oven_id: "o".into(),
                current_celsius: 1.0,
                target_celsius: 2.0,
                enabled: true,
                heating: true,
                output_level: None,
                state: OvenState::Heating,
            })),
            "OvenStatusUpdated"
        );
        assert_eq!(
            message_type_name(&Message::FaultRaised(FaultRaisedPayload {
                oven_id: None,
                fault_code: FaultCode::SafetyLimitExceeded,
                severity: Severity::High,
                message: "m".into(),
            })),
            "FaultRaised"
        );
    }
}
