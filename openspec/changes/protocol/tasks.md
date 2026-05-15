# Tasks: protocol — Crate Shared Types

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~600–700 |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Foundation) → PR 2 (Envelope + Tests) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Foundation: version + types + payloads + message (lib re-export) | PR 1 → main | Base module files; TDD unit tests per module |
| 2 | Envelope + roundtrip integration tests | PR 2 → main | Depends on PR 1; EventEnvelope + full integration suite |

---

## Phase 1: Foundation — Module Setup

- [ ] 1.1 `protocol/Cargo.toml` — add `uuid = "1"` and `chrono = { version = "0.4", features = ["serde"] }` dependencies
- [ ] 1.2 `protocol/src/version.rs` — create with `pub const PROTOCOL_VERSION: &str = "1";` + TDD test
- [ ] 1.3 `protocol/src/types.rs` — create with enums: `OvenState` (Disabled/Idle/Heating/Faulted/EmergencyStopped), `FaultCode` (6 variants per spec), `RequestScope` (Single/All), `Severity` (Low/Medium/High) + TDD round-trip tests per enum
- [ ] 1.4 `protocol/src/payloads.rs` — create with all 9 payload structs (4 command + 5 event) with `#[serde(rename_all = "snake_case")]` + TDD tests per struct

## Phase 2: Core — Message Enum + Re-exports

- [ ] 2.1 `protocol/src/message.rs` — create `Message` enum with `#[serde(tag = "type", content = "payload")]` and 9 variants + TDD round-trip tests per variant
- [ ] 2.2 `protocol/src/lib.rs` — add module declarations + re-export `EventEnvelope`, `Message`, `PROTOCOL_VERSION`, all payloads, all enums from `protocol::`

## Phase 3: Envelope — EventEnvelope

- [ ] 3.1 `protocol/src/envelope.rs` — create `EventEnvelope` struct with all fields + `new()` and `reply_to()` helpers + TDD unit tests for construction and reply_to
- [ ] 3.2 `protocol/tests/roundtrip.rs` — create integration tests: full envelope→JSON→envelope for all 9 Message variants + unknown variant rejection test

## Dependencies Graph

```
Task 1.1 (Cargo.toml)         → unblocks → all
Task 1.2 (version.rs)         → unblocks → Task 2.2
Task 1.3 (types.rs)           → unblocks → Task 1.4
Task 1.4 (payloads.rs)        → unblocks → Task 2.1
Task 2.1 (message.rs)         → unblocks → Task 2.2, Task 3.2
Task 2.2 (lib.rs re-exports)  → unblocks → Task 3.1
Task 3.1 (envelope.rs)        → unblocks → Task 3.2
Task 3.2 (roundtrip tests)    → final gate
```

## Estimation Detail

| Task | Focus | Est. Lines |
|------|-------|------------|
| 1.1 | Cargo.toml deps | ~5 |
| 1.2 | version.rs | ~10 |
| 1.3 | types.rs (4 enums) | ~50 |
| 1.4 | payloads.rs (9 structs) | ~180 |
| 2.1 | message.rs (enum + tests) | ~120 |
| 2.2 | lib.rs re-exports | ~30 |
| 3.1 | envelope.rs (struct + helpers + tests) | ~100 |
| 3.2 | roundtrip.rs (integration) | ~150 |
| **Total** | | **~645** |

## Review Workload Note

~645 lines exceeds the 400-line budget. Split into 2 PRs:
- **PR 1**: Tasks 1.1–2.1 (foundation + message enum + re-exports) — ~365 lines
- **PR 2**: Tasks 3.1–3.2 (envelope + integration tests) — ~250 lines

User decision needed: approve stacked-to-main chain strategy before sdd-apply starts.

## Next Step

Ready for sdd-apply. **Decision needed from user** on chain strategy before implementation begins.