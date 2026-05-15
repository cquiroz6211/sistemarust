# Design: Protocolo Compartido — Crate `protocol`

## Technical Approach

Implementar el crate `protocol` como biblioteca de tipos Rust puros, sin dependencias de runtime ni hardware. Un único enum `Message` con `#[serde(tag = "type", content = "payload")]` y un `EventEnvelope` genérico de metadatos. Cada variante lleva un struct payload dedicado. Los tipos auxiliares (`OvenState`, `FaultCode`, etc.) son enums planos. La API se expone íntegramente desde `protocol::` para que `pc-app` y `rpi-controller` consuman un solo `use`.

## Architecture Decisions

| Decision | Options | Tradeoffs | Choice |
|---|---|---|---|
| **Module layout** | a) Single `message.rs` with everything<br>b) Split by concern (`envelope`, `message`, `payloads`, `types`, `version`) | (a) Fewer files, but payloads obscure the enum.<br>(b) Clear boundaries, easier parallel TDD. | **(b)** Split by concern. |
| **Error handling** | a) Reuse `serde_json::Error`<br>b) Add custom `ProtocolError` with `thiserror` | (a) Zero deps, sufficient for v1.<br>(b) Richer context, but extra crate. | **(a)** `serde_json::Error` is enough for a types-only crate. |
| **Enum casing in JSON** | a) `snake_case`<br>b) `PascalCase` (Rust default) | (a) Matches spec examples for `fault_code`.<br>(b) Matches spec examples for `state` (`"Heating"`). | **PascalCase for enums**, **snake_case for struct fields**. Use `#[serde(rename_all = "snake_case")]` on structs; default PascalCase on enums. |
| `output_level` type | a) `f64`<br>b) `Option<f64>` | (a) Simpler struct.<br>(b) Honours "si aplica" semantics. | **(b)** `Option<f64>` to represent absence. |
| `EventEnvelope` generics | a) `EventEnvelope<T>`<br>b) `EventEnvelope` with concrete `Message` | (a) Reusable for future non-Message envelopes.<br>(b) Simpler API, no turbofish in tests. | **(b)** Concrete `Message` for v1. Can generalise later. |

## Data Flow

    pc-app / rpi-controller
           │
           ▼
    ┌──────────────┐
    │  construct   │  EventEnvelope::new(src, tgt, msg)
    │   Message    │
    └──────┬───────┘
           │ serde_json::to_string
           ▼
         JSON wire
           │ serde_json::from_str
           ▼
    ┌──────────────┐
    │ deserialize  │  EventEnvelope
    │   Message    │
    └──────────────┘
           │
           ▼
     match message { ... }

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `protocol/Cargo.toml` | Modify | Add `uuid`, `chrono` dependencies. |
| `protocol/src/lib.rs` | Modify | Re-export public API: `EventEnvelope`, `Message`, `PROTOCOL_VERSION`, all payloads, all enums. |
| `protocol/src/envelope.rs` | Create | `EventEnvelope` with helpers `new()` and `reply_to()`. |
| `protocol/src/message.rs` | Create | `Message` enum with 9 variants and `#[serde(tag = "type", content = "payload")]`. |
| `protocol/src/payloads.rs` | Create | Payload structs: command + event payloads. |
| `protocol/src/types.rs` | Create | `OvenState`, `FaultCode`, `RequestScope`, `Severity`. |
| `protocol/src/version.rs` | Create | `pub const PROTOCOL_VERSION: &str = "1";` |
| `protocol/tests/roundtrip.rs` | Create | Integration tests: serde round-trip per variant + envelope. |

## Interfaces / Contracts

```rust
// version.rs
pub const PROTOCOL_VERSION: &str = "1";

// envelope.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub source: String,
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
    pub version: String,
    pub payload: Message,
}

impl EventEnvelope {
    pub fn new(source: &str, target: &str, payload: Message) -> Self;
    pub fn reply_to(&self, payload: Message) -> Self;
}

// message.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    SetTargetTemperature(SetTargetTemperaturePayload),
    SetOvenEnabled(SetOvenEnabledPayload),
    RequestStatus(RequestStatusPayload),
    EmergencyStop(EmergencyStopPayload),
    OvenDetected(OvenDetectedPayload),
    CommandAccepted(CommandAcceptedPayload),
    CommandRejected(CommandRejectedPayload),
    OvenStatusUpdated(OvenStatusUpdatedPayload),
    FaultRaised(FaultRaisedPayload),
}
```

Serde attributes:
- `Message`: `#[serde(tag = "type", content = "payload")]`
- All payload structs: `#[serde(rename_all = "snake_case")]`
- Enums (`OvenState`, `FaultCode`, etc.): default PascalCase to match spec (`"Heating"`, `"OvenNotFound"`)

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Each `Message` variant serde round-trip | `#[cfg(test)]` modules in `src/message.rs`, `src/payloads.rs`, `src/types.rs` |
| Unit | `EventEnvelope` construction + `reply_to` | Module tests in `src/envelope.rs` |
| Unit | Missing field rejection | Malformed JSON strings in tests |
| Integration | Full envelope → JSON → envelope for every variant | `tests/roundtrip.rs` |
| Integration | Unknown variant rejection | `tests/roundtrip.rs` |

All tests follow TDD red-green-refactor. `cargo test -p protocol` is the gate.

## Migration / Rollout

No migration required. This is a new crate. `pc-app` and `rpi-controller` will add `protocol = { path = "../protocol" }` in future changes.

## Open Questions — RESOLVED

- [x] Should `CommandRejected.reason` be `FaultCode` or `String`? → **DECISION: `FaultCode`**. Type safety and standardisation across PC and Raspberry. Only oven-related reasons travel in the protocol.
- [x] Should `Severity` include `Critical` or stop at `High`? → **DECISION: `Low`, `Medium`, `High` only**. v1 intentionally simple. Adding `Critical` requires clear criteria definition in a future version.
