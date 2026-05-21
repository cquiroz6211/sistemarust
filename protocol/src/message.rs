use serde::{Deserialize, Serialize};

use crate::payloads::{
    CommandAcceptedPayload, CommandRejectedPayload, EmergencyStopPayload, FaultRaisedPayload,
    OvenDetectedPayload, OvenStatusUpdatedPayload, RequestStatusPayload, SetOvenEnabledPayload,
    SetTargetTemperaturePayload,
};

/// Mensaje del protocolo de eventos PC ↔ Raspberry Pi.
///
/// Usa tagging externo: `{"type": "VariantName", "payload": {...}}`.
/// 4 comandos (PC→RPi) + 5 eventos (RPi→PC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    // ── Comandos (PC → Raspberry Pi) ──
    SetTargetTemperature(SetTargetTemperaturePayload),
    SetOvenEnabled(SetOvenEnabledPayload),
    RequestStatus(RequestStatusPayload),
    EmergencyStop(EmergencyStopPayload),

    // ── Eventos (Raspberry Pi → PC) ──
    OvenDetected(OvenDetectedPayload),
    CommandAccepted(CommandAcceptedPayload),
    CommandRejected(CommandRejectedPayload),
    OvenStatusUpdated(OvenStatusUpdatedPayload),
    FaultRaised(FaultRaisedPayload),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FaultCode, OvenState, RequestScope, Severity};

    /// Round-trip completo: Message → JSON → Message
    fn assert_message_roundtrip(msg: &Message) {
        let json = serde_json::to_string(msg).expect("serialización de Message falló");
        let back: Message =
            serde_json::from_str(&json).expect("deserialización de Message falló");
        assert_eq!(&back, msg, "Message round-trip no coincide");
    }

    // ── Comandos ──

    #[test]
    fn message_set_target_temperature_roundtrip() {
        let msg = Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 250.0,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_set_target_temperature_boundary_zero() {
        let msg = Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 0.0,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_set_oven_enabled_roundtrip() {
        let msg = Message::SetOvenEnabled(SetOvenEnabledPayload {
            oven_id: "oven1".into(),
            enabled: true,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_request_status_single_roundtrip() {
        let msg = Message::RequestStatus(RequestStatusPayload {
            oven_id: Some("oven1".into()),
            scope: RequestScope::Single,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_request_status_all_roundtrip() {
        let msg = Message::RequestStatus(RequestStatusPayload {
            oven_id: None,
            scope: RequestScope::All,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_emergency_stop_roundtrip() {
        let msg = Message::EmergencyStop(EmergencyStopPayload {
            reason: "Overheating".into(),
        });
        assert_message_roundtrip(&msg);
    }

    // ── Eventos ──

    #[test]
    fn message_oven_detected_roundtrip() {
        let msg = Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_oven_detected_json_type_field() {
        let msg = Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"OvenDetected\""), "type debe ser OvenDetected");
    }

    #[test]
    fn message_command_accepted_roundtrip() {
        let msg = Message::CommandAccepted(CommandAcceptedPayload {
            accepted_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            message: "OK".into(),
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_command_rejected_roundtrip() {
        let msg = Message::CommandRejected(CommandRejectedPayload {
            rejected_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            reason: FaultCode::OvenNotFound,
            message: "Horno no encontrado".into(),
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_oven_status_updated_roundtrip() {
        let msg = Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 180.5,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_oven_status_updated_state_heating() {
        let msg = Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 200.0,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(50.0),
            state: OvenState::Heating,
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"state\":\"Heating\""));
    }

    #[test]
    fn message_fault_raised_roundtrip() {
        let msg = Message::FaultRaised(FaultRaisedPayload {
            oven_id: Some("oven1".into()),
            fault_code: FaultCode::SafetyLimitExceeded,
            severity: Severity::High,
            message: "Límite excedido".into(),
        });
        assert_message_roundtrip(&msg);
    }

    #[test]
    fn message_fault_raised_sin_oven_id() {
        let msg = Message::FaultRaised(FaultRaisedPayload {
            oven_id: None,
            fault_code: FaultCode::EmergencyStopActive,
            severity: Severity::High,
            message: "Emergencia activa".into(),
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"oven_id\":null"));
        assert!(json.contains("\"fault_code\":\"EmergencyStopActive\""));
    }

    // ── Unknown variant rejection ──

    #[test]
    fn message_rechaza_variante_desconocida() {
        let json = r#"{"type":"RemoveOven","payload":{}}"#;
        let result = serde_json::from_str::<Message>(json);
        assert!(result.is_err(), "Debe rechazar variante desconocido");
    }

    // ── Missing required field ──

    #[test]
    fn message_campo_faltante_falla() {
        let json = r#"{"type":"SetTargetTemperature","payload":{"oven_id":"o1"}}"#;
        let result = serde_json::from_str::<Message>(json);
        assert!(result.is_err(), "falta target_celsius, debe fallar");
    }
}
