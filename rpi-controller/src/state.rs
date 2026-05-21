//! Core domain types: Oven entity and OvenStore.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use protocol::{OvenState, FaultCode, FaultRaisedPayload, Severity};

/// Simulation constants for temperature dynamics.
pub const HEATING_DRIFT: f64 = 4.0;      // °C per tick when heating
pub const COOLING_DRIFT: f64 = 1.0;     // °C per tick when cooling
pub const ROOM_TEMP: f64 = 20.0;        // Minimum temperature (°C)
pub const NOISE_AMPLITUDE: f64 = 2.0;   // ±°C random noise per tick
pub const HYSTERESIS: f64 = 5.0;       // °C hysteresis band
pub const TICK_INTERVAL_SECS: u64 = 2;  // seconds between simulation ticks

/// Represents a simulated industrial oven.
#[derive(Debug, Clone)]
pub struct Oven {
    pub oven_id: String,
    pub sensor_ref: String,
    pub output_ref: String,
    pub max_celsius: f64,
    pub current_celsius: f64,
    pub target_celsius: f64,
    pub enabled: bool,
    pub heating: bool,
    pub state: OvenState,
}

impl Default for Oven {
    fn default() -> Self {
        Self {
            oven_id: String::new(),
            sensor_ref: String::new(),
            output_ref: String::new(),
            max_celsius: 300.0,
            current_celsius: ROOM_TEMP,
            target_celsius: ROOM_TEMP,
            enabled: false,
            heating: false,
            state: OvenState::Disabled,
        }
    }
}

impl Oven {
    /// Creates a new simulated oven with given parameters.
    pub fn new(oven_id: &str, sensor_ref: &str, output_ref: &str, max_celsius: f64) -> Self {
        Self {
            oven_id: oven_id.to_string(),
            sensor_ref: sensor_ref.to_string(),
            output_ref: output_ref.to_string(),
            max_celsius,
            current_celsius: ROOM_TEMP,
            target_celsius: ROOM_TEMP,
            enabled: false,
            heating: false,
            state: OvenState::Disabled,
        }
    }
}

/// Thread-safe oven store holding all managed ovens.
pub type OvenStore = Arc<RwLock<HashMap<String, Oven>>>;

/// Creates a new empty OvenStore.
pub fn new_oven_store() -> OvenStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Inserts an oven into the store. Returns the oven.
pub async fn insert_oven(store: &OvenStore, oven: Oven) -> Oven {
    let mut guard = store.write().await;
    guard.insert(oven.oven_id.clone(), oven.clone());
    oven
}

/// Removes an oven from the store by ID.
pub async fn remove_oven(store: &OvenStore, oven_id: &str) -> Option<Oven> {
    let mut guard = store.write().await;
    guard.remove(oven_id)
}

/// Gets a clone of an oven by ID.
pub async fn get_oven(store: &OvenStore, oven_id: &str) -> Option<Oven> {
    let guard = store.read().await;
    guard.get(oven_id).cloned()
}

/// Gets all oven IDs currently in the store.
pub async fn get_all_oven_ids(store: &OvenStore) -> Vec<String> {
    let guard = store.read().await;
    guard.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn oven_store_insert_and_get() {
        let store = new_oven_store();
        let oven = Oven::new("oven1", "temp0", "relay0", 300.0);

        let inserted = insert_oven(&store, oven.clone()).await;
        assert_eq!(inserted.oven_id, "oven1");

        let retrieved = get_oven(&store, "oven1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().oven_id, "oven1");
    }

    #[tokio::test]
    async fn oven_store_remove() {
        let store = new_oven_store();
        let oven = Oven::new("oven1", "temp0", "relay0", 300.0);
        insert_oven(&store, oven).await;

        let removed = remove_oven(&store, "oven1").await;
        assert!(removed.is_some());

        let gone = get_oven(&store, "oven1").await;
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn oven_store_get_all_ids() {
        let store = new_oven_store();
        insert_oven(&store, Oven::new("oven1", "t0", "r0", 300.0)).await;
        insert_oven(&store, Oven::new("oven2", "t1", "r1", 300.0)).await;

        let ids = get_all_oven_ids(&store).await;
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"oven1".to_string()));
        assert!(ids.contains(&"oven2".to_string()));
    }

    #[tokio::test]
    async fn oven_store_get_nonexistent() {
        let store = new_oven_store();
        let result = get_oven(&store, "nonexistent").await;
        assert!(result.is_none());
    }

    #[test]
    fn oven_default_disabled_at_room_temp() {
        let oven = Oven::default();
        assert!(!oven.enabled);
        assert_eq!(oven.current_celsius, ROOM_TEMP);
        assert_eq!(oven.target_celsius, ROOM_TEMP);
        assert_eq!(oven.state, OvenState::Disabled);
        assert!(!oven.heating);
    }

    #[test]
    fn oven_new_sets_fields() {
        let oven = Oven::new("o1", "temp0", "relay0", 300.0);
        assert_eq!(oven.oven_id, "o1");
        assert_eq!(oven.sensor_ref, "temp0");
        assert_eq!(oven.output_ref, "relay0");
        assert_eq!(oven.max_celsius, 300.0);
        assert_eq!(oven.current_celsius, ROOM_TEMP);
    }
}