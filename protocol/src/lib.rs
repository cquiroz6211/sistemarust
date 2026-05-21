// protocolo para comunicación entre PC y raspberry
//
// Este crate define los tipos compartidos del protocolo de eventos
// entre la aplicación PC y el controlador Raspberry Pi.
// Todos los tipos se re-exportan desde la raíz para facilitar el uso:
//
// ```
// use protocol::{Message, EventEnvelope, PROTOCOL_VERSION};
// ```

pub mod envelope;
pub mod message;
pub mod payloads;
pub mod types;
pub mod version;

// ── Re-exports públicos ──

pub use envelope::EventEnvelope;
pub use message::Message;
pub use version::PROTOCOL_VERSION;

// Enums de tipos
pub use types::{FaultCode, OvenState, RequestScope, Severity};

// Payloads de comandos
pub use payloads::{
    CommandAcceptedPayload, CommandRejectedPayload, EmergencyStopPayload,
    FaultRaisedPayload, OvenDetectedPayload, OvenStatusUpdatedPayload,
    RequestStatusPayload, SetOvenEnabledPayload, SetTargetTemperaturePayload,
};
