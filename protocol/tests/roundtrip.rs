// Tests de integración: round-trip completo de EventEnvelope → JSON → EventEnvelope
// para todas las variantes de Message, más tests de rechazo de variantes desconocidos.

use protocol::{
    CommandAcceptedPayload, CommandRejectedPayload, EmergencyStopPayload, EventEnvelope,
    FaultCode, FaultRaisedPayload, Message, OvenDetectedPayload, OvenState,
    OvenStatusUpdatedPayload, RequestScope, RequestStatusPayload, SetOvenEnabledPayload,
    SetTargetTemperaturePayload, Severity, PROTOCOL_VERSION,
};

/// Helper: round-trip completo de un EventEnvelope.
/// Verifica que serialize → deserialize produce un envelope equivalente.
fn assert_envelope_roundtrip(env: &EventEnvelope) {
    let json = serde_json::to_string(env).expect("serialización del envelope falló");
    let back: EventEnvelope =
        serde_json::from_str(&json).expect("deserialización del envelope falló");
    assert_eq!(&back, env, "envelope round-trip no coincide");
}

// ═══════════════════════════════════════════════════════
// Comandos (PC → Raspberry Pi)
// ═══════════════════════════════════════════════════════

#[test]
fn roundtrip_set_target_temperature() {
    let env = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 250.0,
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_set_target_temperature_boundary_zero() {
    let env = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 0.0,
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_set_oven_enabled() {
    let env = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetOvenEnabled(SetOvenEnabledPayload {
            oven_id: "oven2".into(),
            enabled: true,
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_request_status_single() {
    let env = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::RequestStatus(RequestStatusPayload {
            oven_id: Some("oven1".into()),
            scope: RequestScope::Single,
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_request_status_all() {
    let env = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::RequestStatus(RequestStatusPayload {
            oven_id: None,
            scope: RequestScope::All,
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_emergency_stop() {
    let env = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::EmergencyStop(EmergencyStopPayload {
            reason: "Sensor failure".into(),
        }),
    );
    assert_envelope_roundtrip(&env);
}

// ═══════════════════════════════════════════════════════
// Eventos (Raspberry Pi → PC)
// ═══════════════════════════════════════════════════════

#[test]
fn roundtrip_oven_detected() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::OvenDetected(OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        }),
    );
    assert_envelope_roundtrip(&env);

    // Verificar que el type en JSON es "OvenDetected"
    let json = serde_json::to_string(&env).unwrap();
    assert!(
        json.contains("\"type\":\"OvenDetected\""),
        "JSON type debe ser OvenDetected"
    );
}

#[test]
fn roundtrip_command_accepted() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::CommandAccepted(CommandAcceptedPayload {
            accepted_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            message: "Temperatura configurada".into(),
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_command_rejected() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::CommandRejected(CommandRejectedPayload {
            rejected_type: "SetOvenEnabled".into(),
            oven_id: Some("oven1".into()),
            reason: FaultCode::OvenNotFound,
            message: "Horno no encontrado".into(),
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_oven_status_updated() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 180.5,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        }),
    );
    assert_envelope_roundtrip(&env);

    // Verificar que state se serializa como "Heating"
    let json = serde_json::to_string(&env).unwrap();
    assert!(json.contains("\"state\":\"Heating\""));
}

#[test]
fn roundtrip_oven_status_updated_disabled() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 25.0,
            target_celsius: 0.0,
            enabled: false,
            heating: false,
            output_level: None,
            state: OvenState::Disabled,
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_fault_raised() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::FaultRaised(FaultRaisedPayload {
            oven_id: Some("oven1".into()),
            fault_code: FaultCode::SafetyLimitExceeded,
            severity: Severity::High,
            message: "Límite de seguridad excedido".into(),
        }),
    );
    assert_envelope_roundtrip(&env);
}

#[test]
fn roundtrip_fault_raised_sin_oven() {
    let env = EventEnvelope::new(
        "rpi-controller",
        "pc-app",
        Message::FaultRaised(FaultRaisedPayload {
            oven_id: None,
            fault_code: FaultCode::EmergencyStopActive,
            severity: Severity::High,
            message: "Parada de emergencia activa".into(),
        }),
    );
    let json = serde_json::to_string(&env).unwrap();
    assert!(json.contains("\"oven_id\":null"));
    assert!(json.contains("\"fault_code\":\"EmergencyStopActive\""));
    assert_envelope_roundtrip(&env);
}

// ═══════════════════════════════════════════════════════
// reply_to: correlación comando → respuesta
// ═══════════════════════════════════════════════════════

#[test]
fn reply_to_correlacion_command_rejected() {
    let cmd = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetTargetTemperature(SetTargetTemperaturePayload {
            oven_id: "oven99".into(),
            target_celsius: 999.0,
        }),
    );
    let reply = cmd.reply_to(Message::CommandRejected(CommandRejectedPayload {
        rejected_type: "SetTargetTemperature".into(),
        oven_id: Some("oven99".into()),
        reason: FaultCode::OvenNotFound,
        message: "Horno inexistente".into(),
    }));

    assert_eq!(reply.correlation_id, Some(cmd.event_id));
    assert_eq!(reply.source, "rpi-controller");
    assert_eq!(reply.target, "pc-app");
    assert_envelope_roundtrip(&reply);
}

#[test]
fn reply_to_correlacion_command_accepted() {
    let cmd = EventEnvelope::new(
        "pc-app",
        "rpi-controller",
        Message::SetOvenEnabled(SetOvenEnabledPayload {
            oven_id: "oven1".into(),
            enabled: true,
        }),
    );
    let reply = cmd.reply_to(Message::CommandAccepted(CommandAcceptedPayload {
        accepted_type: "SetOvenEnabled".into(),
        oven_id: Some("oven1".into()),
        message: "Horno habilitado".into(),
    }));

    assert_eq!(reply.correlation_id, Some(cmd.event_id));
    assert_envelope_roundtrip(&reply);
}

// ═══════════════════════════════════════════════════════
// Unknown variant rejection
// ═══════════════════════════════════════════════════════

#[test]
fn unknown_variant_rechazado() {
    let json = r#"{"type":"RemoveOven","payload":{}}"#;
    let result = serde_json::from_str::<Message>(json);
    assert!(result.is_err(), "Debe rechazar variante desconocido RemoveOven");
}

#[test]
fn unknown_variant_nonexistent_command() {
    let json = r#"{"type":"Calibrate","payload":{"oven_id":"oven1"}}"#;
    let result = serde_json::from_str::<Message>(json);
    assert!(result.is_err(), "Debe rechazar variante desconocido Calibrate");
}

// ═══════════════════════════════════════════════════════
// Version contract
// ═══════════════════════════════════════════════════════

#[test]
fn version_constante_es_uno() {
    assert_eq!(PROTOCOL_VERSION, "1");
}

#[test]
fn version_en_todos_los_envelopes() {
    let envelopes = [
        EventEnvelope::new("a", "b", Message::EmergencyStop(EmergencyStopPayload {
            reason: "test".into(),
        })),
        EventEnvelope::new("x", "y", Message::OvenDetected(OvenDetectedPayload {
            oven_id: "o1".into(),
            sensor_ref: "s1".into(),
            output_ref: "r1".into(),
            max_celsius: 100.0,
        })),
    ];
    for env in &envelopes {
        let json = serde_json::to_string(env).unwrap();
        assert!(
            json.contains("\"version\":\"1\""),
            "Todo envelope debe tener version \"1\": {}",
            json
        );
    }
}

// ═══════════════════════════════════════════════════════
// Missing fields rejection
// ═══════════════════════════════════════════════════════

#[test]
fn envelope_sin_event_id_falla() {
    let json = r#"{
        "source": "pc",
        "target": "rpi",
        "timestamp": "2025-01-01T00:00:00Z",
        "version": "1",
        "payload": {"type":"EmergencyStop","payload":{"reason":"x"}}
    }"#;
    let result = serde_json::from_str::<EventEnvelope>(json);
    assert!(result.is_err(), "falta event_id, debe fallar");
}

#[test]
fn message_con_payload_incompleto_falla() {
    let json = r#"{"type":"SetTargetTemperature","payload":{"oven_id":"o1"}}"#;
    let result = serde_json::from_str::<Message>(json);
    assert!(result.is_err(), "falta target_celsius, debe fallar");
}
