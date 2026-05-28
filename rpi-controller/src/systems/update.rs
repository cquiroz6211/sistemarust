//! `Update` schedule systems — command ingestion, routing, and response emission.
//!
//! ## Event Flow (per spec)
//! 1. `ingest_commands` drains `InboundProtocolQueue` → emits `CommandReceivedEvent`
//! 2. `route_command` reads `CommandReceivedEvent`, validates, mutates ECS components,
//!    emits `CommandAcceptedEvent` / `CommandRejectedEvent`
//! 3. `emit_protocol_responses` drains the above → `OutboundProtocolQueue`

use bevy::prelude::{Entity, EventReader, EventWriter, Query, Res, ResMut};

use protocol::{
    CommandAcceptedPayload, CommandRejectedPayload, EventEnvelope, FaultCode, Message,
    SetOvenEnabledPayload, SetTargetTemperaturePayload, EmergencyStopPayload,
};

use crate::components::{
    Enabled, MaxTemperature, OvenId, OvenStatus, TargetTemperature,
};
use crate::events::{CommandAcceptedEvent, CommandReceivedEvent, CommandRejectedEvent};
use crate::resources::{EmergencyStopActive, InboundProtocolQueue, OutboundProtocolQueue, OvenIndex};

// ── System 1: drain inbound queue → internal event ─────────────────────────────

/// Drains the `InboundProtocolQueue` resource and emits one `CommandReceivedEvent`
/// per envelope for downstream processing.
pub fn ingest_commands(
    mut inbound: ResMut<InboundProtocolQueue>,
    mut ev_command: EventWriter<CommandReceivedEvent>,
) {
    for envelope in inbound.0.drain(..) {
        ev_command.send(CommandReceivedEvent(envelope));
    }
}

// ── System 2: read internal event → mutate ECS + emit response event ───────────

/// Main command router: reads each `CommandReceivedEvent`, validates it against
/// the domain rules, mutates ECS component state, and emits `CommandAcceptedEvent`
/// or `CommandRejectedEvent` for the response phase.
pub fn route_command(
    mut ev_command: EventReader<CommandReceivedEvent>,
    mut emergency_active: ResMut<EmergencyStopActive>,
    mut ev_accepted: EventWriter<CommandAcceptedEvent>,
    mut ev_rejected: EventWriter<CommandRejectedEvent>,
    mut ev_status: EventWriter<crate::events::StatusPublishEvent>,
    oven_index: Res<OvenIndex>,
    mut query: Query<(Entity, &OvenId, &MaxTemperature, &mut TargetTemperature, &mut Enabled, &mut OvenStatus)>,
) {
    for cmd_ev in ev_command.read() {
        let envelope = &cmd_ev.0;

        match &envelope.payload {
            Message::SetTargetTemperature(payload) => {
                handle_set_target_temperature(
                    envelope,
                    payload,
                    &mut emergency_active,
                    &oven_index,
                    &mut query,
                    &mut ev_accepted,
                    &mut ev_rejected,
                );
            }
            Message::SetOvenEnabled(payload) => {
                handle_set_oven_enabled(
                    envelope,
                    payload,
                    &mut emergency_active,
                    &oven_index,
                    &mut query,
                    &mut ev_accepted,
                    &mut ev_rejected,
                );
            }
            Message::RequestStatus(payload) => {
                // RequestStatus emits OvenStatusUpdated directly via OutboundProtocolQueue
                // to avoid EventWriter move issues
                let oven_ids: Vec<String> = match payload.scope {
                    protocol::RequestScope::Single => {
                        if let Some(ref oid) = payload.oven_id {
                            if oven_index.0.contains_key(oid) {
                                vec![oid.clone()]
                            } else {
                                ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(oid.clone()), FaultCode::OvenNotFound)));
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    }
                    protocol::RequestScope::All => {
                        oven_index.0.keys().cloned().collect()
                    }
                };
                // The status will be emitted by emit_status_events after this
                // We just need to send a StatusPublishEvent
                for oid in oven_ids {
                    ev_status.send(crate::events::StatusPublishEvent(Some(oid)));
                }
            }
            Message::EmergencyStop(payload) => {
                handle_emergency_stop(
                    envelope,
                    payload,
                    &mut emergency_active,
                    &mut query,
                    &mut ev_accepted,
                );
            }
            // Protocol events (PC→RPi direction): OvenDetected, CommandAccepted,
            // CommandRejected, OvenStatusUpdated, FaultRaised — ignored by controller in v1.
            _ => {}
        }
    }
}

fn handle_set_target_temperature(
    envelope: &EventEnvelope,
    payload: &SetTargetTemperaturePayload,
    emergency_active: &mut EmergencyStopActive,
    oven_index: &OvenIndex,
    query: &mut Query<(Entity, &OvenId, &MaxTemperature, &mut TargetTemperature, &mut Enabled, &mut OvenStatus)>,
    ev_accepted: &mut EventWriter<CommandAcceptedEvent>,
    ev_rejected: &mut EventWriter<CommandRejectedEvent>,
) {
    // Rule 1: Emergency stop active
    if emergency_active.0 {
        ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::EmergencyStopActive)));
        return;
    }

    // Rule 2: Find the oven entity
    let entity = match oven_index.0.get(&payload.oven_id) {
        Some(&e) => e,
        None => {
            ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::OvenNotFound)));
            return;
        }
    };

    // Get oven components
    let (_, _, max_temp, mut target_temp, _, _) = match query.get_mut(entity) {
        Ok(components) => components,
        Err(_) => {
            ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::OvenNotFound)));
            return;
        }
    };

    // Rule 3: Negative temperature
    if payload.target_celsius < 0.0 {
        ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::InvalidTemperature)));
        return;
    }

    // Rule 4: Exceeds max
    if payload.target_celsius > max_temp.0 {
        ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::SafetyLimitExceeded)));
        return;
    }

    // Rule 5: Valid — update target
    target_temp.0 = payload.target_celsius;
    ev_accepted.send(CommandAcceptedEvent(make_acceptance(envelope, Some(payload.oven_id.clone()), format!("Target set to {}°C", payload.target_celsius))));
}

fn handle_set_oven_enabled(
    envelope: &EventEnvelope,
    payload: &SetOvenEnabledPayload,
    emergency_active: &mut EmergencyStopActive,
    oven_index: &OvenIndex,
    query: &mut Query<(Entity, &OvenId, &MaxTemperature, &mut TargetTemperature, &mut Enabled, &mut OvenStatus)>,
    ev_accepted: &mut EventWriter<CommandAcceptedEvent>,
    ev_rejected: &mut EventWriter<CommandRejectedEvent>,
) {
    // Rule 1: Emergency stop
    if emergency_active.0 {
        ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::EmergencyStopActive)));
        return;
    }

    // Rule 2: Find entity
    let entity = match oven_index.0.get(&payload.oven_id) {
        Some(&e) => e,
        None => {
            ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::OvenNotFound)));
            return;
        }
    };

    // Check if oven can be enabled (not Faulted/EmergencyStopped)
    // Use single query access - get all components at once
    let (_, _, _, _, oven_status) = match query.get(entity) {
        Ok(components) => {
            let (_, _, _, _, _, status) = components;
            ((), (), (), (), status)
        }
        Err(_) => {
            ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::OvenNotFound)));
            return;
        }
    };

    if matches!(oven_status.0, protocol::OvenState::Faulted | protocol::OvenState::EmergencyStopped) {
        ev_rejected.send(CommandRejectedEvent(make_rejection(envelope, Some(payload.oven_id.clone()), FaultCode::OutputUnavailable)));
        return;
    }

    // Apply enable - single mutable access
    let (_, _, _, _, mut enabled, _) = query.get_mut(entity).unwrap();
    enabled.0 = payload.enabled;
    ev_accepted.send(CommandAcceptedEvent(make_acceptance(envelope, Some(payload.oven_id.clone()), if payload.enabled { "Oven enabled".into() } else { "Oven disabled".into() })));
}

fn handle_emergency_stop(
    envelope: &EventEnvelope,
    _payload: &EmergencyStopPayload,
    emergency_active: &mut EmergencyStopActive,
    query: &mut Query<(Entity, &OvenId, &MaxTemperature, &mut TargetTemperature, &mut Enabled, &mut OvenStatus)>,
    ev_accepted: &mut EventWriter<CommandAcceptedEvent>,
) {
    // EmergencyStop is always accepted
    emergency_active.0 = true;

    // Set all ovens to EmergencyStopped
    for (_, _, _, _, mut enabled, mut status) in query.iter_mut() {
        enabled.0 = false;
        status.0 = protocol::OvenState::EmergencyStopped;
    }

    ev_accepted.send(CommandAcceptedEvent(make_acceptance(envelope, None, "Emergency stop activated".into())));
}

// ── System 3: drain response events → outbound queue ──────────────────────────

/// Drains `CommandAcceptedEvent` and `CommandRejectedEvent` and appends them
/// to the `OutboundProtocolQueue` as protocol envelopes.
pub fn emit_protocol_responses(
    mut ev_accepted: EventReader<CommandAcceptedEvent>,
    mut ev_rejected: EventReader<CommandRejectedEvent>,
    mut outbound: ResMut<OutboundProtocolQueue>,
) {
    for ev in ev_accepted.read() {
        outbound.0.push(ev.0.clone());
    }
    for ev in ev_rejected.read() {
        outbound.0.push(ev.0.clone());
    }
}

// ── Helper builders ────────────────────────────────────────────────────────────

fn command_type_name(msg: &Message) -> &'static str {
    match msg {
        Message::SetTargetTemperature(_) => "SetTargetTemperature",
        Message::SetOvenEnabled(_) => "SetOvenEnabled",
        Message::RequestStatus(_) => "RequestStatus",
        Message::EmergencyStop(_) => "EmergencyStop",
        _ => "Unknown",
    }
}

fn make_acceptance(envelope: &EventEnvelope, oven_id: Option<String>, message: String) -> EventEnvelope {
    envelope.reply_to(Message::CommandAccepted(CommandAcceptedPayload {
        accepted_type: command_type_name(&envelope.payload).to_string(),
        oven_id,
        message,
    }))
}

fn make_rejection(envelope: &EventEnvelope, oven_id: Option<String>, reason: FaultCode) -> EventEnvelope {
    envelope.reply_to(Message::CommandRejected(CommandRejectedPayload {
        rejected_type: command_type_name(&envelope.payload).to_string(),
        oven_id,
        reason,
        message: format!("{:?}", reason),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::FixedUpdate;
    use bevy::prelude::*;
    use crate::bevy_app::build_app;
    use crate::resources::{InboundProtocolQueue, OutboundProtocolQueue};
    use protocol::{RequestStatusPayload, RequestScope};

    fn clear_outbound(app: &mut App) {
        app.world_mut().resource_mut::<OutboundProtocolQueue>().0.clear();
    }

    fn app_with_one_oven() -> App {
        let mut app = build_app(1);
        app.update();
        clear_outbound(&mut app);
        app
    }

    fn push_command(app: &mut App, msg: Message) {
        app.world_mut()
            .resource_mut::<InboundProtocolQueue>()
            .0
            .push(EventEnvelope::new("pc-app", "rpi-controller", msg));
    }

    #[test]
    fn set_target_temperature_valido_es_aceptado() {
        let mut app = app_with_one_oven();
        push_command(
            &mut app,
            Message::SetTargetTemperature(SetTargetTemperaturePayload {
                oven_id: "oven-0".into(),
                target_celsius: 120.0,
            }),
        );

        app.update();

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound
            .0
            .iter()
            .any(|env| matches!(env.payload, Message::CommandAccepted(_))));
    }

    #[test]
    fn set_target_temperature_negativo_es_rechazado() {
        let mut app = app_with_one_oven();
        push_command(
            &mut app,
            Message::SetTargetTemperature(SetTargetTemperaturePayload {
                oven_id: "oven-0".into(),
                target_celsius: -1.0,
            }),
        );

        app.update();

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound.0.iter().any(|env| {
            matches!(
                &env.payload,
                Message::CommandRejected(CommandRejectedPayload {
                    reason: FaultCode::InvalidTemperature,
                    ..
                })
            )
        }));
    }

    #[test]
    fn comando_a_horno_inexistente_es_rechazado() {
        let mut app = app_with_one_oven();
        push_command(
            &mut app,
            Message::SetOvenEnabled(SetOvenEnabledPayload {
                oven_id: "oven-99".into(),
                enabled: true,
            }),
        );

        app.update();

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound.0.iter().any(|env| {
            matches!(
                &env.payload,
                Message::CommandRejected(CommandRejectedPayload {
                    reason: FaultCode::OvenNotFound,
                    ..
                })
            )
        }));
    }

    #[test]
    fn emergency_stop_rechaza_comandos_posteriores() {
        let mut app = app_with_one_oven();
        push_command(
            &mut app,
            Message::EmergencyStop(EmergencyStopPayload {
                reason: "test".into(),
            }),
        );
        app.update();
        clear_outbound(&mut app);

        push_command(
            &mut app,
            Message::SetTargetTemperature(SetTargetTemperaturePayload {
                oven_id: "oven-0".into(),
                target_celsius: 130.0,
            }),
        );
        app.update();

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound.0.iter().any(|env| {
            matches!(
                &env.payload,
                Message::CommandRejected(CommandRejectedPayload {
                    reason: FaultCode::EmergencyStopActive,
                    ..
                })
            )
        }));
    }

    #[test]
    fn request_status_emite_oven_status_updated() {
        let mut app = app_with_one_oven();
        push_command(
            &mut app,
            Message::RequestStatus(RequestStatusPayload {
                oven_id: Some("oven-0".into()),
                scope: RequestScope::Single,
            }),
        );

        app.update();
        app.world_mut().run_schedule(FixedUpdate);

        let outbound = app.world().resource::<OutboundProtocolQueue>();
        assert!(outbound
            .0
            .iter()
            .any(|env| matches!(env.payload, Message::OvenStatusUpdated(_))));
    }
}
