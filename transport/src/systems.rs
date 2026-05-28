//! Bevy Resource types for channel handles.
//!
//! These resources bridge the tokio async transport tasks with the Bevy ECS world.
//! Bevy systems read/write these channel handles — the async tasks never touch the World.

use bevy::prelude::Resource;
use tokio::sync::mpsc;

use protocol::EventEnvelope;

/// Bevy Resource holding the sender side of the outbound channel.
///
/// Bevy bridge systems drain `OutboundProtocolQueue` and call `try_send` on this.
/// The tokio writer task receives from the paired `Receiver`.
#[derive(Resource)]
pub struct OutboundSender(pub mpsc::Sender<Vec<EventEnvelope>>);

/// Bevy Resource holding the receiver side of the inbound channel.
///
/// The `Mutex` is required because `mpsc::Receiver` is `Send` but not `Sync`.
/// Bevy bridge systems lock the mutex and call `try_recv`.
/// The tokio reader task sends to the paired `Sender`.
#[derive(Resource)]
pub struct InboundReceiver(pub std::sync::Mutex<mpsc::Receiver<Vec<EventEnvelope>>>);

/// Wraps the tokio runtime to keep it alive as a Bevy Resource.
#[derive(Resource)]
pub struct TokioRuntime(pub tokio::runtime::Runtime);
