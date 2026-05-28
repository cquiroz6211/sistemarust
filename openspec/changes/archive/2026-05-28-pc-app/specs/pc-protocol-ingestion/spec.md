# pc-protocol-ingestion Specification

## Purpose

Drain `InboundProtocolQueue`, deserialize `EventEnvelope`s, dispatch internal ECS events, and route each protocol event to its corresponding state-update system. Malformed or unsupported envelopes are safely rejected according to protocol semantics.

## Requirements

### Requirement: Inbound queue draining

The system MUST drain all `EventEnvelope`s from `InboundProtocolQueue` every tick and dispatch each one as an internal ECS event.

#### Scenario: Queue drained on tick

- GIVEN `InboundProtocolQueue` contains 3 envelopes
- WHEN the ingestion system runs
- THEN all 3 envelopes are consumed from the queue
- AND the queue SHALL be empty after the tick

#### Scenario: Empty queue is a no-op

- GIVEN `InboundProtocolQueue` is empty
- WHEN the ingestion system runs
- THEN no internal events are emitted
- AND no state changes occur

### Requirement: Event dispatch by variant

The ingestion system MUST map each `Message` variant to a corresponding internal event: `OvenDetected` → `OvenDiscovered`, `OvenStatusUpdated` → `OvenStatusReceived`, `FaultRaised` → `FaultReceived`, `CommandAccepted` → `CommandAcceptedReceived`, `CommandRejected` → `CommandRejectedReceived`.

#### Scenario: All variants dispatch correctly

- GIVEN `InboundProtocolQueue` contains one envelope of each event variant
- WHEN the ingestion system runs
- THEN internal events `OvenDiscovered`, `OvenStatusReceived`, `FaultReceived`, `CommandAcceptedReceived`, and `CommandRejectedReceived` are emitted
- AND each internal event SHALL carry the payload data from its source envelope

### Requirement: Malformed envelope rejection

The system MUST ignore envelopes that fail deserialization and MUST NOT crash the application.

#### Scenario: Malformed JSON is rejected

- GIVEN `InboundProtocolQueue` contains a non-JSON byte sequence
- WHEN the ingestion system runs
- THEN the malformed entry is discarded
- AND the system continues processing subsequent valid envelopes

#### Scenario: Unknown Message variant is rejected

- GIVEN `InboundProtocolQueue` contains an envelope with `type: "RemoveOven"`
- WHEN the ingestion system runs
- THEN the unrecognized variant is silently dropped
- AND no state changes or internal events are emitted for that entry

### Requirement: Command direction guard

The system MUST NOT dispatch commands (`SetTargetTemperature`, `SetOvenEnabled`, `RequestStatus`, `EmergencyStop`) received in `InboundProtocolQueue` — only event variants SHALL be accepted.

#### Scenario: Inbound command is ignored

- GIVEN `InboundProtocolQueue` contains an envelope with `Message::SetTargetTemperature`
- WHEN the ingestion system runs
- THEN the command variant is silently dropped
- AND no internal event or state change occurs from that envelope
