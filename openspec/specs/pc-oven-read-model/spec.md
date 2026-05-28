# pc-oven-read-model Specification

## Purpose

Maintain a local ECS representation of all ovens detected by the Raspberry Pi. The read model is updated by inbound protocol events and persists the latest known state of each oven's identity, sensors, output, temperature, operational status, and fault state.

## Requirements

### Requirement: Oven entity lifecycle

The system MUST spawn a new ECS entity for each unique `OvenDetected` event and MUST update the existing entity on subsequent `OvenDetected` events for the same `oven_id`.

#### Scenario: First detection spawns entity

- GIVEN no entity exists for `oven_id = "oven1"`
- WHEN an `OvenDetected` event arrives for `oven1`
- THEN a new entity with `oven_id = "oven1"` is spawned
- AND the entity carries components `OvenId`, `SensorRef`, `OutputRef`, `MaxTemperature`

#### Scenario: Redetection updates existing entity

- GIVEN an entity exists for `oven_id = "oven1"` with `MaxTemperature = 300.0`
- WHEN a second `OvenDetected` event arrives for `oven1` with `MaxTemperature = 350.0`
- THEN the entity's `MaxTemperature` is updated to `350.0`
- AND no duplicate entity is spawned

### Requirement: Oven status field updates

The system MUST update `CurrentTemperature`, `TargetTemperature`, `Enabled`, `Heating`, and `OvenStatus` components on the matching oven entity when an `OvenStatusUpdated` event arrives.

#### Scenario: Status update applied to known oven

- GIVEN an entity exists for `oven_id = "oven1"` with `CurrentTemperature = 0.0`
- WHEN an `OvenStatusUpdated` event arrives for `oven1` with `current_celsius = 180.5`
- THEN the entity's `CurrentTemperature` SHALL be `180.5`
- AND `TargetTemperature`, `Enabled`, `Heating`, `OvenStatus` reflect the payload

#### Scenario: Status update for unknown oven is ignored

- GIVEN no entity exists for `oven_id = "unknown-oven"`
- WHEN an `OvenStatusUpdated` event arrives for `unknown-oven`
- THEN the event is silently discarded without spawning a new entity

### Requirement: Fault state persistence

The system MUST record `FaultState` from a `FaultRaised` event and MUST NOT clear it on subsequent `OvenStatusUpdated` events from the same oven.

#### Scenario: Fault recorded and survives status update

- GIVEN an entity exists for `oven_id = "oven1"` with no active fault
- WHEN a `FaultRaised` event arrives for `oven1`
- THEN the entity's `FaultState` component reflects `fault_code`, `severity`, and `message`
- AND after a subsequent `OvenStatusUpdated` for `oven1`, the `FaultState` remains unchanged

#### Scenario: Global fault without oven_id

- GIVEN a `FaultRaised` event with `oven_id = None`
- WHEN the event is ingested
- THEN the system stores the fault as a global system-level fault
- AND no oven entity's `FaultState` is modified

### Requirement: Command result recording

The system MUST record the last `CommandAccepted` or `CommandRejected` result per oven.

#### Scenario: Accepted command recorded

- GIVEN an entity exists for `oven_id = "oven1"`
- WHEN a `CommandAccepted` event arrives for `oven1`
- THEN the entity's `LastCommandResult` SHALL store `accepted_type`, and `message`

#### Scenario: Rejected command recorded

- GIVEN an entity exists for `oven_id = "oven1"`
- WHEN a `CommandRejected` event arrives for `oven1`
- THEN the entity's `LastCommandResult` SHALL store `rejected_type`, `reason`, and `message`
