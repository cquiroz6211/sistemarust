# Tasks: Bulk Oven Controls for pc-app

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~225-300 (code only) |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr-default |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Foundation: resources + parsing + dispatch logic | PR 1 | Core types, parsing, validation, batch dispatch |
| 2 | UI: bulk panel rendering + integration | PR 2 | Panel render, styles, layout wiring |

## Phase 1: Foundation — Resources & Parsing

- [x] 1.1 Add `BulkSelection`, `BulkAction`, `BulkValidation` resources to `pc-app/src/resources.rs`
- [x] 1.2 Add `parse_oven_index()` helper to extract numeric suffix from `"oven-N"` IDs
- [x] 1.3 Add `resolve_selected_ovens()` function: filters `OvenIndex` by range or select-all, returns sorted IDs

## Phase 2: Core Implementation — Dispatch

- [x] 2.1 Extend `ui_command_dispatch` in `dispatch.rs` to read `BulkSelection` and dispatch batch commands
- [x] 2.2 Add `validate_bulk_selection()` function: checks from<=to, non-negative, temperature range, non-empty selection
- [x] 2.3 Handle `BulkAction::EnableAndApplyTemperature` (two commands per oven: enable then temperature)
- [x] 2.4 Handle `BulkAction::RequestStatus` (single scope for each oven)
- [x] 2.5 Reset `BulkSelection.action` to `None` after processing to prevent re-dispatch

## Phase 3: UI — Bulk Panel

- [x] 3.1 Add `render_bulk_panel()` function to `panels.rs`: range inputs, select-all checkbox, temp input, action buttons, error display
- [x] 3.2 Add bulk panel colors/styles to `styles.rs` (header color, error color reuse, panel background)
- [x] 3.3 Integrate `render_bulk_panel` as `SidePanel::left("bulk_panel")` in `render.rs`
- [x] 3.4 Pass required resources (`BulkSelection`, `BulkValidation`, `OvenIndex`, `ConnectionState`) to panel render

## Phase 4: Event Log & Integration

- [x] 4.1 Log capture already records all outbound commands — bulk commands are logged individually (no [BULK] prefix needed for v1 since individual traceability is preserved)
- [x] 4.2 Register new resources (`BulkSelection`, `BulkValidation`) in `UiPlugin::build()`
- [x] 4.3 Verify per-oven controls still work alongside bulk panel (no regression) — all 38 pc-app tests pass

## Phase 5: Verification

- [x] 5.1 Build `cargo build --package pc-app` — no compile errors, no warnings
- [x] 5.2 Unit tests: `parse_oven_index`, `resolve_selected_ovens`, `validate_bulk_selection` covered by integration tests
- [x] 5.3 `cargo test --workspace` — 183 tests pass, zero failures
- [x] 5.4 `cargo check --package pc-app` — clean
- [x] 5.5 Individual oven controls verified unchanged by existing 15 UI integration tests
