# Delta Spec: rpi-controller

## Purpose

Define the behavior of the Raspberry Pi physical controller crate: simulated oven management, command validation, threshold heating control with hysteresis, and serial event bridge.

## ADDED Requirements

### Requirement: Oven Simulation

| ID | Requirement |
|---|---|
| OVEN-SIM-1 | The system MUST allow the operator to create a simulated oven with `oven_id`, `sensor_ref`, `output_ref`, and `max_celsius`. |
| OVEN-SIM-2 | The system MUST emit `OvenDetected` for each simulated oven immediately upon creation. |

#### Scenario: Create simulated oven
- GIVEN no ovens exist
- WHEN the operator creates simulated oven "oven1" with max_celsius 300.0
- THEN `OvenDetected` is emitted with oven_id "oven1" and max_celsius 300.0

### Requirement: Command Validation

| ID | Requirement |
|---|---|
| CMD-VAL-1 | The system MUST validate every incoming command before applying it. |
| CMD-VAL-2 | The system MUST reject a command with `CommandRejected` if the `oven_id` does not exist. |
| CMD-VAL-3 | The system MUST reject `SetTargetTemperature` if `target_celsius` is negative or exceeds `max_celsius`. |
| CMD-VAL-4 | The system MUST reject all commands except `EmergencyStop` when emergency stop is active. |

#### Scenario: Valid command accepted
- GIVEN oven "oven1" exists with max_celsius 300.0
- WHEN `SetTargetTemperature` for "oven1" with 150.0 is received
- THEN `CommandAccepted` is emitted

#### Scenario: Non-existent oven rejected
- GIVEN no oven "oven99" exists
- WHEN `SetTargetTemperature` for "oven99" is received
- THEN `CommandRejected` is emitted with `FaultCode::OvenNotFound`

#### Scenario: Invalid temperature rejected
- GIVEN oven "oven1" exists with max_celsius 300.0
- WHEN `SetTargetTemperature` for "oven1" with -10.0 is received
- THEN `CommandRejected` is emitted with `FaultCode::InvalidTemperature`

#### Scenario: Emergency stop blocks other commands
- GIVEN emergency stop is active
- WHEN `SetTargetTemperature` for any oven is received
- THEN `CommandRejected` is emitted with `FaultCode::EmergencyStopActive`

#### Scenario: Emergency stop always accepted
- GIVEN emergency stop is active or inactive
- WHEN `EmergencyStop` is received
- THEN `CommandAccepted` is emitted

### Requirement: Heating Control

| ID | Requirement |
|---|---|
| HEAT-CTRL-1 | The system MUST apply threshold heating control with 5°C hysteresis. |
| HEAT-CTRL-2 | The system MUST set `heating = true` when `enabled && current < target - 5.0`. |
| HEAT-CTRL-3 | The system MUST set `heating = false` when `current >= target`. |
| HEAT-CTRL-4 | The system MUST maintain the previous `heating` state when `target - 5.0 <= current < target`. |

#### Scenario: Heating turns on below hysteresis band
- GIVEN oven enabled, target 150.0, current 140.0
- WHEN temperature update is processed
- THEN `heating` becomes true

#### Scenario: Heating turns off at target
- GIVEN oven enabled, target 150.0, current 150.0
- WHEN temperature update is processed
- THEN `heating` becomes false

#### Scenario: Hysteresis maintains state
- GIVEN oven enabled, target 150.0, current 147.0, previous heating was true
- WHEN temperature update is processed
- THEN `heating` remains true

### Requirement: Temperature Simulation

| ID | Requirement |
|---|---|
| TEMP-SIM-1 | The system MUST simulate temperature changes every 2 seconds. |
| TEMP-SIM-2 | The system MUST increase temperature when `heating == true`. |
| TEMP-SIM-3 | The system MUST decrease temperature toward room temperature (20°C) when `heating == false`. |
| TEMP-SIM-4 | The system MUST NOT let temperature fall below room temperature. |

#### Scenario: Temperature rises when heating
- GIVEN oven at 100.0°C, heating enabled
- WHEN 2 seconds pass
- THEN current_celsius increases

#### Scenario: Temperature stabilizes at room temp
- GIVEN oven at 25.0°C, heating disabled
- WHEN 2 seconds pass
- THEN current_celsius decreases but does not go below 20.0°C

### Requirement: Status Reporting

| ID | Requirement |
|---|---|
| STATUS-1 | The system MUST emit `OvenStatusUpdated` for each oven every 2 seconds. |
| STATUS-2 | The system MUST emit `OvenStatusUpdated` immediately after any state change. |

#### Scenario: Periodic status report
- GIVEN 2 ovens exist and are running
- WHEN the 2-second ticker fires
- THEN `OvenStatusUpdated` is emitted for each oven

### Requirement: Fault Detection

| ID | Requirement |
|---|---|
| FAULT-1 | The system MUST emit `FaultRaised` when temperature exceeds `max_celsius`. |
| FAULT-2 | The system MUST set oven state to `Faulted` when a safety fault occurs. |

#### Scenario: Safety limit exceeded
- GIVEN oven with max_celsius 300.0
- WHEN simulated temperature reaches 305.0°C
- THEN `FaultRaised` is emitted with `FaultCode::SafetyLimitExceeded`

### Requirement: Emergency Stop

| ID | Requirement |
|---|---|
| ESTOP-1 | The system MUST disable all ovens immediately upon `EmergencyStop`. |
| ESTOP-2 | The system MUST set all ovens to `EmergencyStopped` state. |
| ESTOP-3 | The system MUST block all non-emergency commands until reset. |

#### Scenario: Emergency stop disables everything
- GIVEN 2 ovens enabled and heating
- WHEN `EmergencyStop` is received
- THEN both ovens become disabled, state becomes `EmergencyStopped`, and `OvenStatusUpdated` is emitted for each

## Acceptance Criteria

- [ ] All command handlers have TDD unit tests covering happy path and edge cases.
- [ ] Temperature simulation respects room temperature floor.
- [ ] Hysteresis logic is verified with boundary values (target - 5.0, target).
- [ ] Emergency stop rejects subsequent `SetTargetTemperature` commands.
- [ ] Serial port abstraction supports mock injection for integration tests.

## TDD Test Contract

| Test Suite | Scope |
|---|---|
| `validation_tests` | Pure functions: command validation rules |
| `control_tests` | Heating decision logic with hysteresis |
| `simulation_tests` | Temperature drift and boundary conditions |
| `handler_tests` | Command → state mutation → event emission |
| `integration_tests` | Mock serial port, full event loop |
