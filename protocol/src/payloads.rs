use serde::{Deserialize, Serialize};

use crate::types::{FaultCode, OvenState, RequestScope, Severity};

// ──────────────────────────────────────────────
// Comandos (PC → Raspberry Pi)
// ──────────────────────────────────────────────

/// Comando: establecer temperatura objetivo de un horno.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetTargetTemperaturePayload {
    pub oven_id: String,
    pub target_celsius: f64,
}

/// Comando: habilitar o deshabilitar un horno.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetOvenEnabledPayload {
    pub oven_id: String,
    pub enabled: bool,
}

/// Comando: solicitar estado actual de hornos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestStatusPayload {
    pub oven_id: Option<String>,
    pub scope: RequestScope,
}

/// Comando: parada de emergencia del sistema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EmergencyStopPayload {
    pub reason: String,
}

// ──────────────────────────────────────────────
// Eventos (Raspberry Pi → PC)
// ──────────────────────────────────────────────

/// Evento: la Raspberry detectó un nuevo horno conectado físicamente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OvenDetectedPayload {
    pub oven_id: String,
    pub sensor_ref: String,
    pub output_ref: String,
    pub max_celsius: f64,
}

/// Evento: confirmación de que un comando fue aceptado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CommandAcceptedPayload {
    pub accepted_type: String,
    pub oven_id: Option<String>,
    pub message: String,
}

/// Evento: rechazo de un comando con motivo tipado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CommandRejectedPayload {
    pub rejected_type: String,
    pub oven_id: Option<String>,
    pub reason: FaultCode,
    pub message: String,
}

/// Evento: actualización del estado operativo de un horno.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OvenStatusUpdatedPayload {
    pub oven_id: String,
    pub current_celsius: f64,
    pub target_celsius: f64,
    pub enabled: bool,
    pub heating: bool,
    pub output_level: Option<f64>,
    pub state: OvenState,
}

/// Evento: se detectó una falla en el sistema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FaultRaisedPayload {
    pub oven_id: Option<String>,
    pub fault_code: FaultCode,
    pub severity: Severity,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper genérico de round-trip serde para cualquier struct.
    fn assert_roundtrip<T>(value: &T)
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serialización falló");
        let back: T = serde_json::from_str(&json).expect("deserialización falló");
        assert_eq!(&back, value, "round-trip no coincide");
    }

    // ─── SetTargetTemperature ───

    #[test]
    fn set_target_temp_roundtrip() {
        let p = SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 250.0,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn set_target_temp_boundary_zero() {
        // Spec: protocol accepts any f64 value; range validation is controller concern
        let p = SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 0.0,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn set_target_temp_json_fields_snake_case() {
        let p = SetTargetTemperaturePayload {
            oven_id: "oven1".into(),
            target_celsius: 250.0,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"oven_id\""), "debe usar snake_case");
        assert!(json.contains("\"target_celsius\""), "debe usar snake_case");
    }

    #[test]
    fn set_target_temp_campo_faltante_falla() {
        let json = r#"{"oven_id":"o1"}"#;
        let result = serde_json::from_str::<SetTargetTemperaturePayload>(json);
        assert!(result.is_err(), "falta target_celsius, debe fallar");
    }

    // ─── SetOvenEnabled ───

    #[test]
    fn set_oven_enabled_roundtrip() {
        let p = SetOvenEnabledPayload {
            oven_id: "oven1".into(),
            enabled: true,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn set_oven_enabled_disabled() {
        let p = SetOvenEnabledPayload {
            oven_id: "oven2".into(),
            enabled: false,
        };
        assert_roundtrip(&p);
    }

    // ─── RequestStatus ───

    #[test]
    fn request_status_single_roundtrip() {
        let p = RequestStatusPayload {
            oven_id: Some("oven1".into()),
            scope: RequestScope::Single,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn request_status_all_roundtrip() {
        let p = RequestStatusPayload {
            oven_id: None,
            scope: RequestScope::All,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn request_status_oven_id_null_en_json() {
        let p = RequestStatusPayload {
            oven_id: None,
            scope: RequestScope::All,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"oven_id\":null"), "None debe ser null en JSON");
    }

    // ─── EmergencyStop ───

    #[test]
    fn emergency_stop_roundtrip() {
        let p = EmergencyStopPayload {
            reason: "Overheating detected".into(),
        };
        assert_roundtrip(&p);
    }

    // ─── OvenDetected ───

    #[test]
    fn oven_detected_roundtrip() {
        let p = OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn oven_detected_json_type_es_correcto() {
        let p = OvenDetectedPayload {
            oven_id: "oven1".into(),
            sensor_ref: "temp0".into(),
            output_ref: "relay0".into(),
            max_celsius: 300.0,
        };
        let json = serde_json::to_string(&p).unwrap();
        // Verificar que todos los campos aparecen con snake_case
        assert!(json.contains("\"oven_id\""));
        assert!(json.contains("\"sensor_ref\""));
        assert!(json.contains("\"output_ref\""));
        assert!(json.contains("\"max_celsius\""));
    }

    // ─── CommandAccepted ───

    #[test]
    fn command_accepted_roundtrip() {
        let p = CommandAcceptedPayload {
            accepted_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            message: "OK".into(),
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn command_accepted_sin_oven_id() {
        let p = CommandAcceptedPayload {
            accepted_type: "EmergencyStop".into(),
            oven_id: None,
            message: "Parada ejecutada".into(),
        };
        assert_roundtrip(&p);
    }

    // ─── CommandRejected ───

    #[test]
    fn command_rejected_roundtrip() {
        let p = CommandRejectedPayload {
            rejected_type: "SetTargetTemperature".into(),
            oven_id: Some("oven1".into()),
            reason: FaultCode::OvenNotFound,
            message: "Horno no encontrado".into(),
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn command_rejected_reason_es_fault_code() {
        let p = CommandRejectedPayload {
            rejected_type: "SetOvenEnabled".into(),
            oven_id: None,
            reason: FaultCode::SensorUnavailable,
            message: "Sensor no disponible".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"reason\":\"SensorUnavailable\""), "reason debe serializar como FaultCode enum");
    }

    // ─── OvenStatusUpdated ───

    #[test]
    fn oven_status_updated_roundtrip() {
        let p = OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 180.5,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(75.0),
            state: OvenState::Heating,
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn oven_status_updated_output_level_none() {
        let p = OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 25.0,
            target_celsius: 250.0,
            enabled: false,
            heating: false,
            output_level: None,
            state: OvenState::Disabled,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"output_level\":null"), "output_level None debe ser null");
    }

    #[test]
    fn oven_status_updated_state_heating_json() {
        let p = OvenStatusUpdatedPayload {
            oven_id: "oven1".into(),
            current_celsius: 200.0,
            target_celsius: 250.0,
            enabled: true,
            heating: true,
            output_level: Some(50.0),
            state: OvenState::Heating,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"state\":\"Heating\""), "state debe ser PascalCase");
    }

    // ─── FaultRaised ───

    #[test]
    fn fault_raised_roundtrip() {
        let p = FaultRaisedPayload {
            oven_id: Some("oven1".into()),
            fault_code: FaultCode::SafetyLimitExceeded,
            severity: Severity::High,
            message: "Temperatura excedió límite de seguridad".into(),
        };
        assert_roundtrip(&p);
    }

    #[test]
    fn fault_raised_sin_oven_id() {
        let p = FaultRaisedPayload {
            oven_id: None,
            fault_code: FaultCode::EmergencyStopActive,
            severity: Severity::High,
            message: "Parada de emergencia activa".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"oven_id\":null"));
        assert!(json.contains("\"fault_code\":\"EmergencyStopActive\""));
    }

    #[test]
    fn fault_raised_severity_baja() {
        let p = FaultRaisedPayload {
            oven_id: Some("oven1".into()),
            fault_code: FaultCode::InvalidTemperature,
            severity: Severity::Low,
            message: "Temperatura fuera de rango".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"severity\":\"Low\""));
    }

    // ─── Tests de JSON malformado (campos faltantes) ───

    #[test]
    fn set_oven_enabled_campo_faltante_falla() {
        let json = r#"{"oven_id":"o1"}"#;
        let result = serde_json::from_str::<SetOvenEnabledPayload>(json);
        assert!(result.is_err(), "falta enabled, debe fallar");
    }

    #[test]
    fn request_status_campo_faltante_falla() {
        let json = r#"{"oven_id":"o1"}"#;
        let result = serde_json::from_str::<RequestStatusPayload>(json);
        assert!(result.is_err(), "falta scope, debe fallar");
    }

    #[test]
    fn emergency_stop_campo_faltante_falla() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<EmergencyStopPayload>(json);
        assert!(result.is_err(), "falta reason, debe fallar");
    }

    #[test]
    fn oven_detected_campo_faltante_falla() {
        let json = r#"{"oven_id":"o1","sensor_ref":"t0"}"#;
        let result = serde_json::from_str::<OvenDetectedPayload>(json);
        assert!(result.is_err(), "faltan output_ref y max_celsius, debe fallar");
    }

    #[test]
    fn command_accepted_campo_faltante_falla() {
        let json = r#"{"accepted_type":"SetTargetTemperature"}"#;
        let result = serde_json::from_str::<CommandAcceptedPayload>(json);
        assert!(result.is_err(), "faltan oven_id y message, debe fallar");
    }

    #[test]
    fn command_rejected_campo_faltante_falla() {
        let json = r#"{"rejected_type":"SetTargetTemperature","reason":"OvenNotFound"}"#;
        let result = serde_json::from_str::<CommandRejectedPayload>(json);
        assert!(result.is_err(), "faltan oven_id y message, debe fallar");
    }

    #[test]
    fn oven_status_updated_campo_faltante_falla() {
        let json = r#"{"oven_id":"o1","current_celsius":100.0}"#;
        let result = serde_json::from_str::<OvenStatusUpdatedPayload>(json);
        assert!(result.is_err(), "faltan varios campos obligatorios, debe fallar");
    }

    #[test]
    fn fault_raised_campo_faltante_falla() {
        let json = r#"{"fault_code":"SensorUnavailable"}"#;
        let result = serde_json::from_str::<FaultRaisedPayload>(json);
        assert!(result.is_err(), "faltan oven_id, severity y message, debe fallar");
    }
}
