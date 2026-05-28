//! Internal ECS events dispatched by the ingestion system.
//!
//! `ingest_inbound_protocol` drains `InboundProtocolQueue` and emits these
//! typed events. State-mutation systems in `FixedUpdate` read them.

use bevy::prelude::Event;
use protocol::{FaultCode, OvenState, Severity};

/// Emitted when the RPi reports a newly detected oven.
#[derive(Debug, Clone, Event)]
pub struct OvenDiscovered {
    pub oven_id: String,
    pub sensor_ref: String,
    pub output_ref: String,
    pub max_celsius: f64,
}

/// Emitted when the RPi reports an updated operational status for an oven.
#[derive(Debug, Clone, Event)]
pub struct OvenStatusReceived {
    pub oven_id: String,
    pub current_celsius: f64,
    pub target_celsius: f64,
    pub enabled: bool,
    pub heating: bool,
    pub output_level: Option<f64>,
    pub state: OvenState,
}

/// Emitted when the RPi reports a fault (oven-level or global).
#[derive(Debug, Clone, Event)]
pub struct FaultReceived {
    pub oven_id: Option<String>,
    pub fault_code: FaultCode,
    pub severity: Severity,
    pub message: String,
}

/// Emitted when the RPi accepts a command.
#[derive(Debug, Clone, Event)]
pub struct CommandAcceptedReceived {
    pub accepted_type: String,
    pub oven_id: Option<String>,
    pub message: String,
}

/// Emitted when the RPi rejects a command.
#[derive(Debug, Clone, Event)]
pub struct CommandRejectedReceived {
    pub rejected_type: String,
    pub oven_id: Option<String>,
    pub reason: FaultCode,
    pub message: String,
}
