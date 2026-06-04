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

// ── UI resources ─────────────────────────────────────────────────────────────

/// Direction of a log entry: inbound (RPi → PC) or outbound (PC → RPi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDirection {
    In,
    Out,
}

impl LogDirection {
    pub fn label(&self) -> &'static str {
        match self {
            LogDirection::In => "IN",
            LogDirection::Out => "OUT",
        }
    }
}

/// A single entry in the event log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: LogDirection,
    pub message_type: String,
    pub summary: String,
}

/// Scrollable event log — FIFO with a 200-entry limit.
#[derive(Debug, Resource)]
pub struct EventLog {
    pub entries: Vec<LogEntry>,
    pub max_entries: usize,
}

impl Default for EventLog {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 200,
        }
    }
}

impl EventLog {
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
}

/// Connection state heuristic — reflects transport channel availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// UI intent captured by egui controls each frame.
/// Processed by `ui_command_dispatch` in the next Update tick.
#[derive(Debug, Resource, Default)]
pub struct UiIntent {
    pub set_enabled: Option<(String, bool)>,
    pub set_temperature: Option<(String, f64)>,
    /// `Some(None)` = request all, `Some(Some(id))` = request single oven.
    pub request_status: Option<Option<String>>,
    pub emergency_stop: bool,
}

/// Tracks whether the Emergency Stop confirmation dialog is open.
#[derive(Debug, Resource, Default)]
pub struct EmergencyStopConfirm {
    pub open: bool,
}

/// Per-oven temperature validation message (shown when out of range).
#[derive(Debug, Resource, Default)]
pub struct TemperatureValidation {
    /// oven_id → optional error message to display.
    pub errors: std::collections::HashMap<String, String>,
}

/// Per-oven local edit state for temperature adjustment.
/// The operator edits a local value, then explicitly clicks "Apply" to send.
#[derive(Debug, Clone, Resource, Default)]
pub struct OvenEditStates {
    /// oven_id → local edit value (the temperature the user is adjusting).
    pub values: std::collections::HashMap<String, OvenEditEntry>,
}

/// Local editing state for a single oven's target temperature.
#[derive(Debug, Clone)]
pub struct OvenEditEntry {
    /// Local target temperature the user is adjusting before applying.
    pub local_target: f64,
    /// Maximum allowed temperature (synced from MaxTemperature component).
    pub max_temp: f64,
}

impl OvenEditStates {
    /// Get or create an edit entry for the given oven, syncing max_temp if needed.
    pub fn get_or_create(&mut self, oven_id: &str, max_temp: f64) -> &mut OvenEditEntry {
        self.values
            .entry(oven_id.to_string())
            .or_insert_with(|| OvenEditEntry {
                local_target: 0.0,
                max_temp,
            })
    }

    /// Sync the local target to the actual target from the ECS (only on first appearance
    /// or if the entry doesn't exist yet).
    pub fn sync_from_ecs(&mut self, oven_id: &str, actual_target: f64, max_temp: f64) {
        let entry = self.get_or_create(oven_id, max_temp);
        entry.max_temp = max_temp;
        // Only overwrite local_target if this is a fresh entry (max_temp == 0.0 means new)
        // or if the user hasn't started editing (we use a sentinel: if local_target == 0.0
        // and actual_target != 0.0, sync).
        if entry.local_target == 0.0 && actual_target != 0.0 {
            entry.local_target = actual_target;
        }
    }
}
