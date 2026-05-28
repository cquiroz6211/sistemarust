//! `FixedUpdate` schedule systems — thermal simulation, hysteresis, fault detection.
//!
//! Runs every 50 ms per spec REQ-HR-001.

use std::time::Duration;

use bevy::prelude::{Entity, EventReader, EventWriter, Query, Res, ResMut};

use protocol::{
    EventEnvelope, FaultCode, Message, OvenState, OvenStatusUpdatedPayload, Severity,
};

use crate::components::{CurrentTemperature, Enabled, Heating, MaxTemperature, OvenId, OvenStatus, TargetTemperature};
use crate::events::{FaultDetectedEvent, StatusPublishEvent};
use crate::resources::{
    ControllerConfig, EmergencyStopActive, LastStatusPublishTime, OutboundProtocolQueue,
    SimulationConfig, TestRng,
};

/// Applies temperature drift based on heating/cooling state.
pub fn thermal_drift(
    mut query: Query<(Entity, &mut CurrentTemperature, &TargetTemperature, &Enabled, &Heating)>,
    config: Res<SimulationConfig>,
    mut test_rng: ResMut<TestRng>,
) {
    for (_entity, mut current, _target, enabled, heating) in query.iter_mut() {
        if !enabled.0 {
            continue;
        }

        let noise = next_noise(&mut test_rng);
        if heating.0 {
            current.0 += config.heating_drift + noise;
        } else {
            current.0 -= config.cooling_drift + noise;
            current.0 = current.0.max(config.room_temp);
        }
    }
}

/// Returns the next noise value from the seeded RNG or zero in production.
fn next_noise(rng: &mut TestRng) -> f64 {
    match &mut rng.0 {
        Some(std_rng) => {
            use rand::Rng;
            std_rng.gen_range(-2.0..=2.0)
        }
        None => 0.0,
    }
}

/// Applies the 5°C hysteresis control rule per ADR 002.
pub fn hysteresis_control(
    mut query: Query<(&mut Heating, &CurrentTemperature, &TargetTemperature, &Enabled)>,
    config: Res<SimulationConfig>,
) {
    for (mut heating, current, target, enabled) in query.iter_mut() {
        if !enabled.0 {
            heating.0 = false;
            continue;
        }

        let lower_bound = target.0 - config.hysteresis;
        heating.0 = if current.0 < lower_bound {
            true
        } else if current.0 >= target.0 {
            false
        } else {
            heating.0 // maintain previous state (dead band)
        };
    }
}

/// Derives the composite `OvenStatus` from component state.
pub fn derive_oven_status(
    mut query: Query<(Entity, &mut OvenStatus, &Enabled, &Heating, &CurrentTemperature, &MaxTemperature)>,
    emergency_active: Res<EmergencyStopActive>,
) {
    for (_entity, mut status, enabled, heating, current, max) in query.iter_mut() {
        if emergency_active.0 {
            status.0 = OvenState::EmergencyStopped;
        } else if !enabled.0 {
            status.0 = OvenState::Disabled;
        } else if heating.0 {
            status.0 = OvenState::Heating;
        } else {
            status.0 = OvenState::Idle;
        }

        // Safety fault overrides the derived state
        if current.0 > max.0 {
            status.0 = OvenState::Faulted;
        }
    }
}

/// Detects when `CurrentTemperature > MaxTemperature` and triggers a fault.
pub fn fault_detection(
    mut query: Query<(Entity, &OvenId, &CurrentTemperature, &MaxTemperature, &mut OvenStatus, &mut Enabled, &mut Heating)>,
    mut ev_fault: EventWriter<FaultDetectedEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    for (_entity, oven_id, current, max, mut status, mut enabled, mut heating) in query.iter_mut() {
        if current.0 > max.0 {
            status.0 = OvenState::Faulted;
            enabled.0 = false;
            heating.0 = false;

            ev_fault.send(FaultDetectedEvent {
                oven_id: oven_id.0.clone(),
                fault_code: FaultCode::SafetyLimitExceeded,
                severity: Severity::High,
                message: format!("{}°C exceeded {}°C limit", current.0, max.0),
            });

            outbound.0.push(EventEnvelope::new(
                "rpi-controller",
                "pc-app",
                Message::FaultRaised(protocol::FaultRaisedPayload {
                    oven_id: Some(oven_id.0.clone()),
                    fault_code: FaultCode::SafetyLimitExceeded,
                    severity: Severity::High,
                    message: format!("Temperature {}°C exceeded safety limit {}°C", current.0, max.0),
                }),
            ));
        }
    }
}

/// Drains `FaultDetectedEvent` and appends `FaultRaised` to the outbound queue.
pub fn emit_fault_events(
    mut ev_fault: EventReader<FaultDetectedEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    for ev in ev_fault.read() {
        outbound.0.push(EventEnvelope::new(
            "rpi-controller",
            "pc-app",
            Message::FaultRaised(protocol::FaultRaisedPayload {
                oven_id: Some(ev.oven_id.clone()),
                fault_code: ev.fault_code,
                severity: ev.severity,
                message: ev.message.clone(),
            }),
        ));
    }
}

/// Publishes `OvenStatusUpdated` periodically for all ovens.
pub fn periodic_status(
    time: Res<bevy::prelude::Time<bevy::prelude::Fixed>>,
    mut last_time: ResMut<LastStatusPublishTime>,
    config: Res<ControllerConfig>,
    mut ev_status: EventWriter<StatusPublishEvent>,
) {
    let now = time.elapsed();
    let interval = Duration::from_millis(config.status_publish_interval_ms);

    let should_publish = if last_time.0 == Duration::ZERO {
        true
    } else {
        now > last_time.0 + interval
    };

    if should_publish {
        last_time.0 = now;
        ev_status.send(StatusPublishEvent(None));
    }
}

/// Drains `StatusPublishEvent` and emits `OvenStatusUpdated` for each matching oven.
pub fn emit_status_events(
    mut ev_status: EventReader<StatusPublishEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
    query: Query<(&OvenId, &CurrentTemperature, &TargetTemperature, &Enabled, &Heating, &OvenStatus)>,
) {
    for ev in ev_status.read() {
        let oven_ids: Vec<_> = match &ev.0 {
            Some(id) => vec![id.clone()],
            None => query.iter().map(|(id, _, _, _, _, _)| id.0.clone()).collect(),
        };

        for oven_id in oven_ids {
            if let Some((_, current, target, enabled, heating, status)) =
                query.iter().find(|(id, _, _, _, _, _)| id.0 == oven_id)
            {
                outbound.0.push(EventEnvelope::new(
                    "rpi-controller",
                    "pc-app",
                    Message::OvenStatusUpdated(OvenStatusUpdatedPayload {
                        oven_id,
                        current_celsius: current.0,
                        target_celsius: target.0,
                        enabled: enabled.0,
                        heating: heating.0,
                        output_level: Some(if heating.0 { 100.0 } else { 0.0 }),
                        state: status.0,
                    }),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};
    use crate::components::{OutputRef, SensorRef};
    use crate::resources::{LastStatusPublishTime, OvenIndex};

    fn spawn_test_oven(world: &mut World, current: f64, target: f64, enabled: bool, heating: bool, max: f64) -> Entity {
        let entity = world.spawn((
            OvenId("oven-0".into()),
            SensorRef("temp-0".into()),
            OutputRef("relay-0".into()),
            CurrentTemperature(current),
            TargetTemperature(target),
            MaxTemperature(max),
            Enabled(enabled),
            Heating(heating),
            OvenStatus(OvenState::Idle),
        )).id();
        world.resource_mut::<OvenIndex>().0.insert("oven-0".into(), entity);
        entity
    }

    fn base_app() -> App {
        let mut app = App::new();
        app.insert_resource(SimulationConfig::default());
        app.insert_resource(TestRng(Some(StdRng::seed_from_u64(42))));
        app.insert_resource(OutboundProtocolQueue::default());
        app.insert_resource(EmergencyStopActive(false));
        app.insert_resource(ControllerConfig::default());
        app.insert_resource(LastStatusPublishTime(Duration::ZERO));
        app.insert_resource(OvenIndex::default());
        app.add_event::<FaultDetectedEvent>();
        app.add_event::<StatusPublishEvent>();
        app
    }

    #[test]
    fn hysteresis_mantiene_estado_en_banda() {
        let mut app = base_app();
        spawn_test_oven(app.world_mut(), 97.0, 100.0, true, true, 300.0);
        app.add_systems(Update, hysteresis_control);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<&Heating>();
        let heating = query.single(world);
        assert!(heating.0);
    }

    #[test]
    fn thermal_drift_no_baja_de_temperatura_ambiente() {
        let mut app = base_app();
        spawn_test_oven(app.world_mut(), 20.0, 100.0, true, false, 300.0);
        app.add_systems(Update, thermal_drift);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<&CurrentTemperature>();
        let current = query.single(world);
        assert!(current.0 >= 20.0);
    }

    #[test]
    fn fault_detection_desactiva_heating_y_emite_fault() {
        let mut app = base_app();
        spawn_test_oven(app.world_mut(), 350.0, 100.0, true, true, 300.0);
        app.add_systems(Update, fault_detection);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&Enabled, &Heating, &OvenStatus)>();
        let (enabled, heating, status) = query.single(world);
        assert!(!enabled.0);
        assert!(!heating.0);
        assert!(matches!(status.0, OvenState::Faulted));

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound.0.iter().any(|env| matches!(env.payload, Message::FaultRaised(_))));
    }

    #[test]
    fn emit_status_events_publica_estado_del_horno() {
        let mut app = base_app();
        spawn_test_oven(app.world_mut(), 42.0, 100.0, true, false, 300.0);
        app.world_mut().send_event(StatusPublishEvent(Some("oven-0".into())));
        app.add_systems(Update, emit_status_events);
        app.update();

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound.0.iter().any(|env| matches!(env.payload, Message::OvenStatusUpdated(_))));
    }
}
