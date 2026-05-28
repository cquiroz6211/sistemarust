//! `Startup` schedule systems — spawns simulated ovens and emits `OvenDetected`.

use bevy::prelude::{Commands, Entity, EventWriter, Query, ResMut, With};

use crate::components::{CurrentTemperature, Enabled, Heating, MaxTemperature, OvenId, OvenStatus, OutputRef, SensorRef, TargetTemperature};
use crate::events::{OvenDetectedEvent, StatusPublishEvent};
use crate::resources::{OvenIndex, OutboundProtocolQueue, SimulationConfig, SimulateOvenCount};
use protocol::{EventEnvelope, Message, OvenDetectedPayload};

/// Spawns N simulated oven entities at startup and populates OvenIndex.
/// Uses `Option<ResMut<SimulateOvenCount>>` so it works even when the resource
/// is not inserted (simulate_count = 0 → no resource → no-op).
pub fn spawn_ovens(
    mut commands: Commands,
    simulate_count: Option<ResMut<SimulateOvenCount>>,
    mut oven_index: ResMut<OvenIndex>,
) {
    let count = match simulate_count {
        Some(sc) => sc.0,
        None => return,
    };

    let config = SimulationConfig::default();
    let room_temp = config.room_temp;

    for i in 0..count {
        let oven_id = format!("oven-{}", i);
        let sensor_ref = format!("temp-{}", i);
        let output_ref = format!("relay-{}", i);

        let entity = commands.spawn((
            OvenId(oven_id.clone()),
            SensorRef(sensor_ref.clone()),
            OutputRef(output_ref.clone()),
            CurrentTemperature(room_temp),
            TargetTemperature(room_temp),
            MaxTemperature(300.0),
            Enabled(false),
            Heating(false),
            OvenStatus(protocol::OvenState::Disabled),
        )).id();

        oven_index.0.insert(oven_id.clone(), entity);
    }
}

/// Emits `OvenDetected` protocol envelopes for each oven entity.
pub fn emit_oven_detected(
    query: Query<(Entity, &OvenId, &SensorRef, &OutputRef), With<OvenId>>,
    mut ev_oven_detected: EventWriter<OvenDetectedEvent>,
    mut ev_status: EventWriter<StatusPublishEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    let count = query.iter().count();

    for (_entity, oven_id, sensor_ref, output_ref) in &query {
        let max_celsius = 300.0;
        let oven_id_str = oven_id.0.clone();

        ev_oven_detected.send(OvenDetectedEvent {
            oven_id: oven_id_str.clone(),
            sensor_ref: sensor_ref.0.clone(),
            output_ref: output_ref.0.clone(),
            max_celsius,
        });

        outbound.0.push(EventEnvelope::new(
            "rpi-controller",
            "pc-app",
            Message::OvenDetected(OvenDetectedPayload {
                oven_id: oven_id_str,
                sensor_ref: sensor_ref.0.clone(),
                output_ref: output_ref.0.clone(),
                max_celsius,
            }),
        ));
    }

    if count > 0 {
        ev_status.send(StatusPublishEvent(None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bevy_app::build_app;
    use crate::resources::OvenIndex;

    #[test]
    fn startup_spawnea_hornos_y_emite_oven_detected() {
        let mut app = build_app(2);
        app.update();

        let index = app.world().resource::<OvenIndex>();
        assert_eq!(index.0.len(), 2);

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        let detected: Vec<_> = outbound
            .0
            .iter()
            .filter(|env| matches!(env.payload, Message::OvenDetected(_)))
            .collect();
        assert_eq!(detected.len(), 2);
    }
}
