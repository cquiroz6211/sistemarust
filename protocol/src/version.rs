/// Versión del protocolo de eventos entre PC y Raspberry Pi.
/// Todos los mensajes v1 usan `"1"` en `EventEnvelope.version`.
pub const PROTOCOL_VERSION: &str = "1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_es_uno() {
        assert_eq!(PROTOCOL_VERSION, "1");
    }

    #[test]
    fn protocol_version_no_esta_vacio() {
        assert!(!PROTOCOL_VERSION.is_empty());
    }
}
