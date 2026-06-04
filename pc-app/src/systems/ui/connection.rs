//! `Update` schedule system — connection state monitor.
//!
//! Heuristic: if `OutboundSender` exists, we assume Connected.
//! The transport crate does not expose explicit connection state.

use bevy::prelude::{Res, ResMut};

use crate::resources::ConnectionState;
use transport::OutboundSender;

pub fn connection_monitor(
    mut state: ResMut<ConnectionState>,
    sender: Option<Res<OutboundSender>>,
) {
    *state = if sender.is_some() {
        ConnectionState::Connected
    } else {
        ConnectionState::Disconnected
    };
}
