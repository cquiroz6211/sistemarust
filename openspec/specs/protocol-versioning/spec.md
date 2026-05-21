# protocol-versioning Specification

## Purpose
Define version identification strategy for the shared event protocol so both sides agree on schema compatibility.

## Requirements

### Requirement: Version Field
`EventEnvelope.version` SHALL be `"1"` for all messages conforming to the initial protocol schema.

#### Scenario: Version present in every envelope
- GIVEN any valid `EventEnvelope` carrying any `Message` variant
- WHEN serialized to JSON
- THEN the `version` field MUST equal `"1"`

### Requirement: Version Constant
The `protocol` crate SHALL expose `pub const PROTOCOL_VERSION: &str = "1";`.

#### Scenario: Constant matches envelope version
- GIVEN `PROTOCOL_VERSION`
- THEN it MUST equal `"1"` and MUST match `EventEnvelope.version` for all v1 messages

### Requirement: Future Compatibility Marker
The protocol crate MAY expose a function or constant indicating the minimum supported version for deserialization. This is optional for v1.

#### Scenario: Version mismatch detection (future)
- GIVEN a received envelope with `version: "2"` (hypothetical)
- WHEN the receiver checks against `PROTOCOL_VERSION`
- THEN it SHOULD reject or handle via a compatibility layer (behavior to be defined in future change)

## Test Contract

- A TDD test SHALL assert that `PROTOCOL_VERSION == "1"`.
- A TDD test SHALL assert that serializing any `EventEnvelope` produces `"version":"1"`.
