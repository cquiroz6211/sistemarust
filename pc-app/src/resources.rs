//! Bevy resources for the PC-side application.
//!
//! Two in-memory protocol queues mediate I/O between PC and RPi, mirroring the
//! `rpi-controller` pattern. `OvenIndex` provides O(1) lookup from protocol
//! oven_id strings to Bevy entities.

use std::collections::HashMap;

use bevy::prelude::{Entity, Resource};
use protocol::EventEnvelope;

use crate::components::FaultInfo;

/// Queue of inbound protocol envelopes (RPi → PC).
/// Drained each tick by `ingest_inbound_protocol`.
#[derive(Debug, Clone, Resource, Default)]
pub struct InboundProtocolQueue(pub Vec<EventEnvelope>);

/// Queue of outbound protocol envelopes (PC → RPi).
/// Command authoring systems push here; transport layer drains it.
#[derive(Debug, Clone, Resource, Default)]
pub struct OutboundProtocolQueue(pub Vec<EventEnvelope>);

/// Fast lookup index: `oven_id` → Bevy `Entity`.
/// Maintained by `apply_oven_detected`; consumed by all state-mutation systems.
#[derive(Debug, Clone, Resource, Default)]
pub struct OvenIndex(pub HashMap<String, Entity>);

/// Global system-level fault (when `FaultRaised` arrives with `oven_id: None`).
/// Oven entities are never modified by global faults.
#[derive(Debug, Clone, Resource, Default)]
pub struct GlobalFault(pub Option<FaultInfo>);
