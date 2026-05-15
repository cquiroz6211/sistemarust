# protocol-message-serialization Specification

## Purpose
Define shared Rust types for PC ↔ Raspberry Pi event protocol. All messages serialize via `serde_json` with external tagging `#[serde(tag = "type", content = "payload")]`.

## Requirements

### Requirement: Message Enum Completeness
The `Message` enum SHALL contain exactly 9 variants: 4 commands (`SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, `EmergencyStop`) and 5 events (`OvenDetected`, `CommandAccepted`, `CommandRejected`, `OvenStatusUpdated`, `FaultRaised`). Each variant SHALL carry a dedicated strongly-typed payload struct.

#### Scenario: OvenDetected round-trip
- GIVEN an `OvenDetected` payload with `oven_id="oven1"`, `sensor_ref="temp0"`, `output_ref="relay0"`, `max_celsius=300.0`
- WHEN serialized to JSON then deserialized back to `Message`
- THEN the result MUST equal the original and JSON `type` MUST be `"OvenDetected"`

#### Scenario: Unknown variant rejection
- GIVEN JSON `{"type":"RemoveOven","payload":{}}`
- WHEN deserializing to `Message`
- THEN serde MUST fail with an unknown variant error

### Requirement: EventEnvelope Structure
`EventEnvelope` SHALL contain: `event_id: Uuid`, `source: String`, `target: String`, `timestamp: DateTime<Utc>`, `correlation_id: Option<Uuid>`, `version: String`, `payload: Message`.

#### Scenario: Envelope serialization completeness
- GIVEN an `EventEnvelope` with all fields populated and `correlation_id: Some(uuid)`
- WHEN serialized to JSON
- THEN every field MUST appear in output and `correlation_id` MUST be a valid UUID string

### Requirement: Command Payloads
`SetTargetTemperature` SHALL have `oven_id`, `target_celsius`.
`SetOvenEnabled` SHALL have `oven_id`, `enabled`.
`RequestStatus` SHALL have `oven_id: Option<String>` and `scope` enum (`Single` / `All`).
`EmergencyStop` SHALL have `reason`.

#### Scenario: SetTargetTemperature with boundary temperature
- GIVEN `SetTargetTemperature` with `target_celsius=0.0`
- WHEN serialized and deserialized
- THEN it MUST round-trip successfully (protocol accepts any f64 value; range validation is controller concern)

#### Scenario: Missing required payload field
- GIVEN JSON `{"type":"SetTargetTemperature","payload":{"oven_id":"o1"}}`
- WHEN deserializing to `Message`
- THEN serde MUST fail due to missing required fields

### Requirement: Event Payloads
`OvenDetected` SHALL have `oven_id`, `sensor_ref`, `output_ref`, `max_celsius`. The Raspberry Pi emits this when it discovers a new physical oven configuration.
`CommandAccepted` SHALL have `accepted_type`, `oven_id: Option<String>`, `message`.
`CommandRejected` SHALL have `rejected_type`, `oven_id: Option<String>`, `reason`, `message`.
`OvenStatusUpdated` SHALL have `oven_id`, `current_celsius`, `target_celsius`, `enabled`, `heating`, `output_level`, `state` enum (`Disabled` / `Idle` / `Heating` / `Faulted` / `EmergencyStopped`).
`FaultRaised` SHALL have `oven_id: Option<String>`, `fault_code` enum, `severity` enum, `message`.

#### Scenario: CommandRejected correlation
- GIVEN a `CommandRejected` event where `correlation_id` matches the original command UUID
- WHEN wrapped in `EventEnvelope`
- THEN envelope `correlation_id` MUST be `Some(original_uuid)`

#### Scenario: OvenStatusUpdated state encoding
- GIVEN `OvenStatusUpdated` with `state: Heating`
- WHEN serialized
- THEN JSON `payload.state` MUST be `"Heating"`

#### Scenario: FaultRaised without specific oven
- GIVEN `FaultRaised` with `oven_id: None` and `fault_code: EmergencyStopActive`
- WHEN serialized
- THEN JSON `payload.oven_id` MUST be `null` and `payload.fault_code` MUST be `"EmergencyStopActive"`

### Requirement: Fault Code Enum
`FaultCode` enum SHALL contain: `OvenNotFound`, `SensorUnavailable`, `InvalidTemperature`, `OutputUnavailable`, `SafetyLimitExceeded`, `EmergencyStopActive`.

#### Scenario: Fault code round-trip
- GIVEN each `FaultCode` variant
- WHEN serialized to string and deserialized back
- THEN the result MUST equal the original variant

## Test Contract

For each requirement above, TDD SHALL apply:
1. **RED**: write a test asserting the requirement before implementation exists.
2. **GREEN**: implement the type until the test passes.
3. **REFACTOR**: clean duplication without changing behavior.

Coverage matrix:
- Every `Message` variant MUST have a serde round-trip test.
- Every payload struct MUST have a deserialization test with both valid and malformed JSON.
- `EventEnvelope` MUST have tests for full serialization, missing fields, and `correlation_id` handling.
- `FaultCode` and `OvenState` enums MUST have exhaustive round-trip tests.
