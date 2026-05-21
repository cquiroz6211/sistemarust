use serde::{Deserialize, Serialize};

/// Estado operativo de un horno.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OvenState {
    Disabled,
    Idle,
    Heating,
    Faulted,
    EmergencyStopped,
}

/// Código de falla del sistema.
/// Se usa tanto en `CommandRejected.reason` como en `FaultRaised.fault_code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultCode {
    OvenNotFound,
    SensorUnavailable,
    InvalidTemperature,
    OutputUnavailable,
    SafetyLimitExceeded,
    EmergencyStopActive,
}

/// Alcance de una solicitud de estado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestScope {
    Single,
    All,
}

/// Severidad de una falla reportada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper genérico para probar round-trip serde de cualquier tipo serializable.
    fn assert_roundtrip<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serialización falló");
        let deserialized: T = serde_json::from_str(&json).expect("deserialización falló");
        assert_eq!(&deserialized, value, "round-trip no coincide para {:?}", value);
    }

    // --- OvenState ---

    #[test]
    fn oven_state_todos_los_variantes_roundtrip() {
        let states = [
            OvenState::Disabled,
            OvenState::Idle,
            OvenState::Heating,
            OvenState::Faulted,
            OvenState::EmergencyStopped,
        ];
        for state in &states {
            assert_roundtrip(state);
        }
    }

    #[test]
    fn oven_state_heating_serializa_como_heating() {
        let json = serde_json::to_string(&OvenState::Heating).unwrap();
        assert_eq!(json, "\"Heating\"");
    }

    #[test]
    fn oven_state_emergency_stopped_serializa_correctamente() {
        let json = serde_json::to_string(&OvenState::EmergencyStopped).unwrap();
        assert_eq!(json, "\"EmergencyStopped\"");
    }

    // --- FaultCode ---

    #[test]
    fn fault_code_todos_los_variantes_roundtrip() {
        let codes = [
            FaultCode::OvenNotFound,
            FaultCode::SensorUnavailable,
            FaultCode::InvalidTemperature,
            FaultCode::OutputUnavailable,
            FaultCode::SafetyLimitExceeded,
            FaultCode::EmergencyStopActive,
        ];
        for code in &codes {
            assert_roundtrip(code);
        }
    }

    #[test]
    fn fault_code_emergency_stop_active_serializa_correctamente() {
        let json = serde_json::to_string(&FaultCode::EmergencyStopActive).unwrap();
        assert_eq!(json, "\"EmergencyStopActive\"");
    }

    #[test]
    fn fault_code_desde_json_string() {
        let code: FaultCode = serde_json::from_str("\"OvenNotFound\"").unwrap();
        assert_eq!(code, FaultCode::OvenNotFound);
    }

    // --- RequestScope ---

    #[test]
    fn request_scope_ambos_variantes_roundtrip() {
        assert_roundtrip(&RequestScope::Single);
        assert_roundtrip(&RequestScope::All);
    }

    #[test]
    fn request_scope_serializa_pascal_case() {
        assert_eq!(
            serde_json::to_string(&RequestScope::Single).unwrap(),
            "\"Single\""
        );
        assert_eq!(
            serde_json::to_string(&RequestScope::All).unwrap(),
            "\"All\""
        );
    }

    // --- Severity ---

    #[test]
    fn severity_todos_los_variantes_roundtrip() {
        assert_roundtrip(&Severity::Low);
        assert_roundtrip(&Severity::Medium);
        assert_roundtrip(&Severity::High);
    }

    #[test]
    fn severity_serializa_pascal_case() {
        assert_eq!(serde_json::to_string(&Severity::Low).unwrap(), "\"Low\"");
        assert_eq!(serde_json::to_string(&Severity::Medium).unwrap(), "\"Medium\"");
        assert_eq!(serde_json::to_string(&Severity::High).unwrap(), "\"High\"");
    }

    // --- Unknown variant rejection ---

    #[test]
    fn oven_state_rechaza_variante_desconocida() {
        let result = serde_json::from_str::<OvenState>("\"UnknownState\"");
        assert!(result.is_err(), "Debería rechazar variante desconocido");
    }

    #[test]
    fn fault_code_rechaza_variante_desconocida() {
        let result = serde_json::from_str::<FaultCode>("\"NotARealFault\"");
        assert!(result.is_err(), "Debería rechazar variante desconocido");
    }
}
