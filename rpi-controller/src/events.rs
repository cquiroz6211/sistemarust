//! Internal ECS events emitted by the controller.

use bevy::prelude::Event;
use protocol::{EventEnvelope, FaultCode, Severity};

/// Emitted when a command is received from the inbound protocol queue.
#[derive(Debug, Clone, Event)]
pub struct CommandReceivedEvent(pub EventEnvelope);

/// Emitted when a command is accepted and processed.
#[derive(Debug, Clone, Event)]
pub struct CommandAcceptedEvent(pub EventEnvelope);

/// Emitted when a command is rejected.
#[derive(Debug, Clone, Event)]
pub struct CommandRejectedEvent(pub EventEnvelope);

/// Emitted when a new oven entity is spawned during startup.
#[derive(Debug, Clone, Event)]
pub struct OvenDetectedEvent {
    pub oven_id: String,
    pub sensor_ref: String,
    pub output_ref: String,
    pub max_celsius: f64,
}

/// Emitted when a thermal fault is detected during `FixedUpdate`.
#[derive(Debug, Clone, Event)]
pub struct FaultDetectedEvent {
    pub oven_id: String,
    pub fault_code: FaultCode,
    pub severity: Severity,
    pub message: String,
}

/// Request to publish status for one oven (Some oven_id) or all (None).
#[derive(Debug, Clone, Event)]
pub struct StatusPublishEvent(pub Option<String>);