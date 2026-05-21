# Design: rpi-controller

## Technical Approach

Async Tokio headless process. Oven store behind `Arc<RwLock<HashMap>>`. `tokio::select!` multiplexes serial commands, a 2 s temperature ticker, and an optional CLI REPL. Command validation is pure functions. Heating control uses 5 °C hysteresis per ADR 002.

## Architecture Decisions

| Decision | Choice | Alternatives rejected | Rationale |
|----------|--------|----------------------|-----------|
| Module layout | 7 modules + `app.rs` orchestrator | Flat `lib.rs` + `main.rs` | Clear separation, unit-testable |
| OvenStore concurrency | `tokio::sync::RwLock` | `std::sync::Mutex`, `dashmap` | Async-friendly; dashmap is overkill for 1–4 ovens |
| Serial abstraction | `trait Transport` with async methods | Direct `serialport` in main | Mockable via channels for tests |
| Temp simulation timing | Single `tokio::time::interval(2s)` in `select!` | Separate task per oven | Simpler, deterministic, easy to cancel |
| Hysteresis impl | Store `heating: bool` in `Oven`, apply ADR 002 rules | Recompute from scratch | Matches spec, trivial, needs previous state anyway |

## Data Flow

```
[Serial USB] ──→ serial::read() ──→ app::select! ──→ handlers::handle()
                                               │
                                               ↓
                                     [OvenStore: RwLock<HashMap>]
                                               │
[Serial USB] ←── serial::write() ←── reply envelope
                                               ↑
[2s ticker] ──→ simulation::update() + control::decide()
                → emit OvenStatusUpdated
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `rpi-controller/src/main.rs` | Modify | CLI args (`--simulate N`), init runtime, spawn `App` |
| `rpi-controller/src/app.rs` | Create | `App` struct, `tokio::select!` loop, orchestration |
| `rpi-controller/src/state.rs` | Create | `Oven` (internal), `OvenStore` = `Arc<RwLock<HashMap<String, Oven>>>` |
| `rpi-controller/src/validation.rs` | Create | Pure `fn validate_*` → `Result<(), FaultCode>` |
| `rpi-controller/src/control.rs` | Create | `fn heating_decision(current, target, previous) -> bool` |
| `rpi-controller/src/simulation.rs` | Create | `fn update_temperature(oven, dt)` with drift & noise |
| `rpi-controller/src/serial.rs` | Create | `trait Transport` + `RealTransport` + `MockTransport` |
| `rpi-controller/src/handlers.rs` | Create | Dispatch `Message` → mutate store + emit replies |
| `rpi-controller/Cargo.toml` | Modify | Add `rand`; keep `serialport` optional behind feature |

## Interfaces / Contracts

```rust
// state.rs
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

pub type OvenStore = Arc<RwLock<HashMap<String, Oven>>>;

// serial.rs
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn read(&mut self) -> anyhow::Result<Option<EventEnvelope>>;
    async fn write(&mut self, envelope: &EventEnvelope) -> anyhow::Result<()>;
}

// validation.rs
pub fn validate_set_target(oven: &Oven, target: f64) -> Result<(), FaultCode>;
pub fn validate_set_enabled(oven: &Oven) -> Result<(), FaultCode>;
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Validation rules | `#[cfg(test)]` in `validation.rs` |
| Unit | Hysteresis boundaries | `#[cfg(test)]` in `control.rs` |
| Unit | Temperature drift & noise | `#[cfg(test)]` in `simulation.rs` |
| Unit | Handler dispatch + side effects | `#[cfg(test)]` in `handlers.rs` with in-memory store |
| Integration | Full event loop with mock transport | `tests/integration.rs` using `MockTransport` |

## Migration / Rollout

No migration required. New crate code; no existing state.

## Open Questions — RESOLVED

- [x] `Transport` uses `async_trait` or RPITIT? → **DECISION: `async_trait`** por ergonomía y facilidad de mock injection en tests.
- [x] Exact drift rates for simulation? → **DECISION: constantes configurables** con `+4 °C/tick` heating, `-1 °C/tick` cooling y `±2 °C` de ruido para una simulación v1 creíble.
