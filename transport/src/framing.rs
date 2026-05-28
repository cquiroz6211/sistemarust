//! JSON line framing for TCP transport.
//!
//! Each `EventEnvelope` is serialized as a single JSON line terminated by `\n`.
//! This module provides serialize and parse helpers used by the reader/writer tasks.

use protocol::EventEnvelope;

/// Serializes an envelope to a JSON string (no trailing newline).
///
/// The caller is responsible for appending `\n` before writing to the socket.
pub fn serialize_envelope(envelope: &EventEnvelope) -> String {
    serde_json::to_string(envelope).expect("EventEnvelope serialization must not fail")
}

/// Parses a single JSON line into an `EventEnvelope`.
///
/// Returns `None` if the line is empty, whitespace-only, or malformed.
/// Malformed lines are logged to stderr and skipped — the caller continues normally.
pub fn parse_json_line(line: &str) -> Option<EventEnvelope> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    match serde_json::from_str(trimmed) {
        Ok(envelope) => Some(envelope),
        Err(e) => {
            eprintln!("[FRAMING] Skipping malformed line: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{Message, SetTargetTemperaturePayload};

    fn sample_envelope() -> EventEnvelope {
        EventEnvelope::new(
            "pc-app",
            "rpi-controller",
            Message::SetTargetTemperature(SetTargetTemperaturePayload {
                oven_id: "oven1".into(),
                target_celsius: 250.0,
            }),
        )
    }

    #[test]
    fn serialize_produces_valid_json() {
        let env = sample_envelope();
        let json = serialize_envelope(&env);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("event_id").is_some());
        assert!(parsed.get("payload").is_some());
    }

    #[test]
    fn roundtrip_valid_envelope() {
        let env = sample_envelope();
        let json = serialize_envelope(&env);
        let result = parse_json_line(&json);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), env);
    }

    #[test]
    fn parse_empty_line_returns_none() {
        assert_eq!(parse_json_line(""), None);
    }

    #[test]
    fn parse_whitespace_only_returns_none() {
        assert_eq!(parse_json_line("   \t  "), None);
    }

    #[test]
    fn parse_malformed_json_returns_none() {
        assert_eq!(parse_json_line("not json at all"), None);
    }

    #[test]
    fn parse_partial_json_returns_none() {
        assert_eq!(parse_json_line(r#"{"event_id":"bad"#), None);
    }

    #[test]
    fn parse_line_with_trailing_newline() {
        let env = sample_envelope();
        let json = serialize_envelope(&env) + "\n";
        let result = parse_json_line(&json);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), env);
    }

    #[test]
    fn roundtrip_multiple_message_types() {
        use protocol::{
            CommandAcceptedPayload, EmergencyStopPayload, FaultCode, FaultRaisedPayload,
            OvenDetectedPayload, OvenState, OvenStatusUpdatedPayload, Severity,
        };

        let envelopes = vec![
            EventEnvelope::new("pc-app", "rpi-controller", Message::EmergencyStop(
                EmergencyStopPayload { reason: "fire".into() },
            )),
            EventEnvelope::new("rpi-controller", "pc-app", Message::OvenDetected(
                OvenDetectedPayload {
                    oven_id: "oven1".into(),
                    sensor_ref: "s1".into(),
                    output_ref: "r1".into(),
                    max_celsius: 300.0,
                },
            )),
            EventEnvelope::new("rpi-controller", "pc-app", Message::CommandAccepted(
                CommandAcceptedPayload {
                    accepted_type: "EmergencyStop".into(),
                    oven_id: None,
                    message: "OK".into(),
                },
            )),
            EventEnvelope::new("rpi-controller", "pc-app", Message::OvenStatusUpdated(
                OvenStatusUpdatedPayload {
                    oven_id: "oven1".into(),
                    current_celsius: 200.0,
                    target_celsius: 250.0,
                    enabled: true,
                    heating: true,
                    output_level: Some(75.0),
                    state: OvenState::Heating,
                },
            )),
            EventEnvelope::new("rpi-controller", "pc-app", Message::FaultRaised(
                FaultRaisedPayload {
                    oven_id: Some("oven1".into()),
                    fault_code: FaultCode::SafetyLimitExceeded,
                    severity: Severity::High,
                    message: "Over temp".into(),
                },
            )),
        ];

        for env in &envelopes {
            let json = serialize_envelope(env);
            let parsed = parse_json_line(&json);
            assert!(parsed.is_some(), "Failed to roundtrip {:?}", env.payload);
            assert_eq!(parsed.unwrap(), *env);
        }
    }
}
