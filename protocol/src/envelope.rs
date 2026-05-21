use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::Message;
use crate::version::PROTOCOL_VERSION;

/// Envoltura de metadatos para cada mensaje del protocolo.
///
/// Contiene información de enrutamiento (`source`, `target`),
/// identificación (`event_id`, `correlation_id`), versión del
/// protocolo y el payload (`Message`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Identificador único del evento.
    pub event_id: Uuid,
    /// Origen del mensaje (ej: `"pc-app"`, `"rpi-controller"`).
    pub source: String,
    /// Destino del mensaje.
    pub target: String,
    /// Momento de creación del evento (UTC).
    pub timestamp: DateTime<Utc>,
    /// ID del evento original al que este responde (si aplica).
    pub correlation_id: Option<Uuid>,
    /// Versión del protocolo. Siempre `PROTOCOL_VERSION` en v1.
    pub version: String,
    /// Mensaje del protocolo (comando o evento).
    pub payload: Message,
}

impl EventEnvelope {
    /// Crea un nuevo envelope con timestamp actual, UUID v4 y versión del protocolo.
    pub fn new(source: &str, target: &str, payload: Message) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            source: source.to_string(),
            target: target.to_string(),
            timestamp: Utc::now(),
            correlation_id: None,
            version: PROTOCOL_VERSION.to_string(),
            payload,
        }
    }

    /// Crea un envelope de respuesta que invierte source/target y conserva
    /// el `event_id` original como `correlation_id`.
    pub fn reply_to(&self, payload: Message) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            source: self.target.clone(),
            target: self.source.clone(),
            timestamp: Utc::now(),
            correlation_id: Some(self.event_id),
            version: PROTOCOL_VERSION.to_string(),
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payloads::SetTargetTemperaturePayload;
    use crate::types::FaultCode;

    /// Helper para crear un envelope de prueba con payload SetTargetTemperature.
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

    // ── Construcción ──

    #[test]
    fn envelope_new_asigna_campos_correctos() {
        let env = sample_envelope();
        assert_eq!(env.source, "pc-app");
        assert_eq!(env.target, "rpi-controller");
        assert_eq!(env.correlation_id, None);
        assert_eq!(env.version, "1");
    }

    #[test]
    fn envelope_new_genera_uuid_no_nil() {
        let env = sample_envelope();
        assert!(!env.event_id.is_nil(), "event_id no debe ser nil");
    }

    #[test]
    fn envelope_new_timestamp_es_reciente() {
        let before = Utc::now();
        let env = sample_envelope();
        let after = Utc::now();
        assert!(env.timestamp >= before && env.timestamp <= after);
    }

    #[test]
    fn envelope_version_es_uno() {
        let env = sample_envelope();
        assert_eq!(env.version, "1");
    }

    // ── reply_to ──

    #[test]
    fn reply_to_invierte_source_target() {
        let original = sample_envelope();
        let reply = original.reply_to(Message::CommandAccepted(
            crate::payloads::CommandAcceptedPayload {
                accepted_type: "SetTargetTemperature".into(),
                oven_id: Some("oven1".into()),
                message: "OK".into(),
            },
        ));
        assert_eq!(reply.source, "rpi-controller");
        assert_eq!(reply.target, "pc-app");
    }

    #[test]
    fn reply_to_correlaciona_con_original() {
        let original = sample_envelope();
        let reply = original.reply_to(Message::CommandAccepted(
            crate::payloads::CommandAcceptedPayload {
                accepted_type: "SetTargetTemperature".into(),
                oven_id: Some("oven1".into()),
                message: "OK".into(),
            },
        ));
        assert_eq!(reply.correlation_id, Some(original.event_id));
    }

    #[test]
    fn reply_to_genera_nuevo_event_id() {
        let original = sample_envelope();
        let reply = original.reply_to(Message::CommandRejected(
            crate::payloads::CommandRejectedPayload {
                rejected_type: "SetTargetTemperature".into(),
                oven_id: Some("oven1".into()),
                reason: FaultCode::OvenNotFound,
                message: "No encontrado".into(),
            },
        ));
        assert_ne!(reply.event_id, original.event_id);
    }

    // ── Serde round-trip ──

    #[test]
    fn envelope_roundtrip_sin_correlation_id() {
        let env = sample_envelope();
        let json = serde_json::to_string(&env).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, env);
    }

    #[test]
    fn envelope_roundtrip_con_correlation_id() {
        let original = sample_envelope();
        let reply = original.reply_to(Message::CommandAccepted(
            crate::payloads::CommandAcceptedPayload {
                accepted_type: "SetTargetTemperature".into(),
                oven_id: Some("oven1".into()),
                message: "OK".into(),
            },
        ));
        let json = serde_json::to_string(&reply).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, reply);
    }

    #[test]
    fn envelope_json_contiene_todos_los_campos() {
        let env = sample_envelope();
        let json = serde_json::to_string(&env).unwrap();
        // Verificar presencia de todos los campos esperados
        assert!(json.contains("\"event_id\""));
        assert!(json.contains("\"source\""));
        assert!(json.contains("\"target\""));
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"payload\""));
        assert!(json.contains("\"correlation_id\":null"));
    }

    #[test]
    fn envelope_correlation_id_es_uuid_string_en_json() {
        let original = sample_envelope();
        let reply = original.reply_to(Message::CommandAccepted(
            crate::payloads::CommandAcceptedPayload {
                accepted_type: "SetTargetTemperature".into(),
                oven_id: Some("oven1".into()),
                message: "OK".into(),
            },
        ));
        let json = serde_json::to_string(&reply).unwrap();
        // correlation_id debe ser un string UUID válido
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let corr_id = parsed["correlation_id"].as_str().unwrap();
        assert!(Uuid::parse_str(corr_id).is_ok(), "correlation_id debe ser UUID válido");
    }

    #[test]
    fn envelope_version_presente_en_json() {
        let env = sample_envelope();
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("\"version\":\"1\""), "version debe ser \"1\" en JSON");
    }
}
