# pc-command-authoring Specification

## Purpose

Provide systems that construct and enqueue protocol commands into `OutboundProtocolQueue`. Each command system reads intent state, builds the correct `EventEnvelope` with the proper `Message` variant, and pushes it to the queue for the transport layer to deliver.

## Requirements

### Requirement: SetTargetTemperature command

The system MUST enqueue a `SetTargetTemperature` command containing `oven_id` and `target_celsius` when triggered.

#### Scenario: Temperature command enqueued

- GIVEN the system has an oven identified by `oven_id`
- WHEN a `SetTargetTemperature` command is issued with `target_celsius = 250.0`
- THEN an `EventEnvelope` with `Message::SetTargetTemperature` is pushed to `OutboundProtocolQueue`
- AND the envelope SHALL contain `oven_id = "oven1"` and `target_celsius = 250.0`
- AND the envelope's `source` SHALL be `"pc-app"` and `target` SHALL be `"rpi-controller"`

### Requirement: SetOvenEnabled command

The system MUST enqueue a `SetOvenEnabled` command with `oven_id` and `enabled` flag when triggered.

#### Scenario: Enable oven command enqueued

- GIVEN the system has an oven identified by `oven_id`
- WHEN a `SetOvenEnabled` command is issued with `enabled = true`
- THEN an `EventEnvelope` with `Message::SetOvenEnabled` is pushed to `OutboundProtocolQueue`
- AND the envelope SHALL carry `oven_id = "oven1"` and `enabled = true`

#### Scenario: Disable oven command enqueued

- GIVEN the system has an oven identified by `oven_id`
- WHEN a `SetOvenEnabled` command is issued with `enabled = false`
- THEN an `EventEnvelope` with `Message::SetOvenEnabled` is pushed to `OutboundProtocolQueue`
- AND the envelope SHALL carry `enabled = false`

### Requirement: RequestStatus command

The system MUST enqueue a `RequestStatus` command supporting both single-oven and all-ovens scope.

#### Scenario: Single-oven status request

- GIVEN the system has an oven identified by `oven_id`
- WHEN a `RequestStatus` command is issued with `scope = Single`
- THEN an `EventEnvelope` with `Message::RequestStatus` is pushed to `OutboundProtocolQueue`
- AND the envelope SHALL carry `oven_id = Some("oven1")` and `scope = Single`

#### Scenario: All-ovens status request

- WHEN a `RequestStatus` command is issued with `scope = All`
- THEN an `EventEnvelope` with `Message::RequestStatus` is pushed to `OutboundProtocolQueue`
- AND the envelope SHALL carry `oven_id = None` and `scope = All`

### Requirement: EmergencyStop command

The system MUST enqueue an `EmergencyStop` command with a descriptive reason when triggered.

#### Scenario: Emergency stop enqueued

- GIVEN the system is running
- WHEN an `EmergencyStop` command is issued with reason `"Overheating detected"`
- THEN an `EventEnvelope` with `Message::EmergencyStop` is pushed to `OutboundProtocolQueue`
- AND the envelope SHALL carry `reason = "Overheating detected"`
