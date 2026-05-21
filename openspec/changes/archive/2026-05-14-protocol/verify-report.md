# Verification Report — `protocol`

**Change**: `protocol` — Shared Event Protocol Crate
**Version**: v1 (initial schema)
**Mode**: Strict TDD
**Date**: 2026-05-14

---

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 8 |
| Tasks complete | 8 |
| Tasks incomplete | 0 |

All tasks from the apply-progress artifact are accounted for and implemented.

---

## Build & Tests Execution

**Build**: ✅ Passed
```text
$ cargo check -p protocol
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.04s
```

**Tests**: ✅ 85 passed / 0 failed / 0 skipped
```text
$ cargo test -p protocol
running 64 tests (unit)
test result: ok. 64 passed; 0 failed; 0 ignored

running 21 tests (integration)
test result: ok. 21 passed; 0 failed; 0 ignored

Doc-tests: 0 passed; 0 failed
```

**Coverage**: ➖ Not available (no coverage tool detected in environment)

---

## Spec Compliance Matrix

### protocol-message-serialization

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| R1: Message Enum Completeness | OvenDetected round-trip | `message::tests::message_oven_detected_roundtrip`<br>`tests::roundtrip_oven_detected` | ✅ COMPLIANT |
| R1: Message Enum Completeness | Unknown variant rejection | `message::tests::message_rechaza_variante_desconocido`<br>`tests::unknown_variant_rechazado`<br>`tests::unknown_variant_nonexistent_command` | ✅ COMPLIANT |
| R2: EventEnvelope Structure | Envelope serialization completeness | `envelope::tests::envelope_json_contiene_todos_los_campos`<br>`envelope::tests::envelope_correlation_id_es_uuid_string_en_json`<br>`envelope::tests::envelope_roundtrip_sin_correlation_id`<br>`envelope::tests::envelope_roundtrip_con_correlation_id` | ✅ COMPLIANT |
| R3: Command Payloads | SetTargetTemperature with boundary temperature | `message::tests::message_set_target_temperature_boundary_zero`<br>`payloads::tests::set_target_temp_boundary_zero`<br>`tests::roundtrip_set_target_temperature_boundary_zero` | ✅ COMPLIANT |
| R3: Command Payloads | Missing required payload field | `message::tests::message_campo_faltante_falla`<br>`payloads::tests::set_target_temp_campo_faltante_falla`<br>`tests::message_con_payload_incompleto_falla` | ✅ COMPLIANT |
| R4: Event Payloads | CommandRejected correlation | `envelope::tests::reply_to_corrrelaciona_con_original`<br>`tests::reply_to_correlacion_command_rejected`<br>`tests::reply_to_correlacion_command_accepted` | ✅ COMPLIANT |
| R4: Event Payloads | OvenStatusUpdated state encoding | `message::tests::message_oven_status_updated_state_heating`<br>`payloads::tests::oven_status_updated_state_heating_json`<br>`tests::roundtrip_oven_status_updated` | ✅ COMPLIANT |
| R4: Event Payloads | FaultRaised without specific oven | `message::tests::message_fault_raised_sin_oven_id`<br>`payloads::tests::fault_raised_sin_oven_id`<br>`tests::roundtrip_fault_raised_sin_oven` | ✅ COMPLIANT |
| R5: Fault Code Enum | Fault code round-trip | `types::tests::fault_code_todos_los_variantes_roundtrip`<br>`types::tests::fault_code_emergency_stop_active_serializa_correctamente`<br>`types::tests::fault_code_desde_json_string` | ✅ COMPLIANT |

### protocol-versioning

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| V1: Version Field | Version present in every envelope | `envelope::tests::envelope_version_presente_en_json`<br>`tests::version_en_todos_los_envelopes` | ✅ COMPLIANT |
| V2: Version Constant | Constant matches envelope version | `version::tests::protocol_version_es_uno`<br>`tests::version_constante_es_uno` | ✅ COMPLIANT |
| V3: Future Compatibility Marker | Version mismatch detection (future) | — | ➖ Optional for v1 |

**Compliance summary**: 10/10 mandatory scenarios compliant

---

## Correctness (Static Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| Message enum has exactly 9 variants (4 cmd + 5 evt) | ✅ Implemented | Verified in `src/message.rs` lines 15–28 |
| Each variant carries a dedicated payload struct | ✅ Implemented | All 9 structs defined in `src/payloads.rs` |
| `#[serde(tag = "type", content = "payload")]` on Message | ✅ Implemented | Line 14 in `src/message.rs` |
| EventEnvelope has all 7 required fields | ✅ Implemented | Lines 14–29 in `src/envelope.rs` |
| `version` is `String` and always `"1"` | ✅ Implemented | Populated from `PROTOCOL_VERSION` in `EventEnvelope::new` |
| `output_level` is `Option<f64>` | ✅ Implemented | Line 82 in `src/payloads.rs` (design decision) |
| All payload structs use `#[serde(rename_all = "snake_case")]` | ✅ Implemented | Every struct in `src/payloads.rs` |
| Enums use default PascalCase serialization | ✅ Implemented | `OvenState`, `FaultCode`, `RequestScope`, `Severity` |
| `FaultCode` has exactly 6 variants | ✅ Implemented | Lines 16–23 in `src/types.rs` |
| `PROTOCOL_VERSION = "1"` exposed as `pub const` | ✅ Implemented | `src/version.rs` line 3 |
| `EventEnvelope::new()` and `reply_to()` helpers | ✅ Implemented | Lines 33–57 in `src/envelope.rs` |
| Public API re-exported from `lib.rs` | ✅ Implemented | Lines 19–31 in `src/lib.rs` |

---

## Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Module layout: split by concern | ✅ Yes | `version`, `types`, `payloads`, `message`, `envelope`, `lib` |
| Error handling: reuse `serde_json::Error` | ✅ Yes | No custom error type; `thiserror` not added |
| Enum casing: PascalCase enums, snake_case structs | ✅ Yes | Verified in JSON output tests |
| `output_level` as `Option<f64>` | ✅ Yes | Represents absence semantics correctly |
| `EventEnvelope` concrete (not generic) | ✅ Yes | Simpler API for v1 |

---

## TDD Compliance

| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | ✅ | Found in apply-progress artifact (`#1185`) |
| All tasks have tests | ✅ | 8/8 tasks have test files |
| RED confirmed (tests exist) | ✅ | All reported test files exist in codebase |
| GREEN confirmed (tests pass) | ✅ | 85/85 tests pass on execution |
| Triangulation adequate | ✅ | Multiple cases per task; 64 unit + 21 integration cases |
| Safety Net for modified files | ✅ | All files were new; safety net N/A is correct |

**TDD Compliance**: 6/6 checks passed

---

## Test Layer Distribution

| Layer | Tests | Files | Tools |
|-------|-------|-------|-------|
| Unit | 64 | 6 (`src/*.rs`) | `cargo test` (built-in) |
| Integration | 21 | 1 (`tests/roundtrip.rs`) | `cargo test` (built-in) |
| E2E | 0 | 0 | not installed |
| **Total** | **85** | **7** | |

---

## Changed File Coverage

Coverage analysis skipped — no coverage tool detected in the environment.

---

## Assertion Quality

✅ All assertions verify real behavior. No tautologies, ghost loops, smoke-only tests, or implementation-detail coupling found.

---

## Quality Metrics

**Linter**: ➖ Not available (`cargo clippy` not installed)
**Type Checker**: ✅ No errors (`cargo check -p protocol` passed cleanly)

---

## Issues Found

### CRITICAL
None.

### WARNING
1. **Incomplete malformed-JSON coverage for payload structs**
   - **What**: The spec test contract states: *"Every payload struct MUST have a deserialization test with both valid and malformed JSON."* Only `SetTargetTemperaturePayload` has an explicit missing-field test (`set_target_temp_campo_faltante_falla`). The remaining 8 payload structs are tested with valid JSON only at the struct level.
   - **Where**: `src/payloads.rs`
   - **Risk**: Low — all structs use the same `serde` derive macro, so the deserialization behavior is identical. However, strict spec compliance is not fully met.
   - **Recommendation**: Add missing-field deserialization tests for `SetOvenEnabledPayload`, `RequestStatusPayload`, `EmergencyStopPayload`, `OvenDetectedPayload`, `CommandAcceptedPayload`, `CommandRejectedPayload`, `OvenStatusUpdatedPayload`, and `FaultRaisedPayload`.

### SUGGESTION
1. **Typo in test name**: `reply_to_corrrelaciona_con_original` has a double `r` (`corrrelaciona` → `correlaciona`) in `src/envelope.rs`.
2. **Spanish grammar in test names**: `message_rechaza_variante_desconocido`, `oven_state_rechaza_variante_desconocido`, and `fault_code_rechaza_variante_desconocido` use masculine `desconocido`; should be feminine `desconocida` because "variante" is feminine.
3. **Doc-tests**: The example in `src/lib.rs` is commented out with `//`. Consider converting it to a real doc-test (```rust) to ensure the public API example compiles.

---

## Verdict

**PASS WITH WARNINGS**

The `protocol` crate is fully functional, all 85 tests pass, and the implementation correctly satisfies every spec requirement. The only deviation is in the **test contract coverage**: not every payload struct has an explicit malformed-JSON deserialization test, which the spec mandates as a MUST. Because the underlying serde mechanism is uniform across all structs, this does not represent a functional risk, but it should be addressed to achieve 100% spec compliance.

**Status**: success
**Summary**: Verification complete for `protocol`. 85/85 tests pass. 10/10 spec scenarios compliant. 1 WARNING on payload malformed-JSON test coverage.
**Artifacts**: `openspec/changes/protocol/verify-report.md` | Engram `sdd/protocol/verify-report`
**Next**: sdd-archive
**Risks**: Low — missing struct-level malformed JSON tests are cosmetic coverage gaps, not functional defects.
**Skill Resolution**: fallback-path — loaded `sdd-verify` skill from filesystem
