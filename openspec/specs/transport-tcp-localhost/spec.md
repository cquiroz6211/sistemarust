# Transport TCP Localhost Specification

## Purpose

Define how two Bevy processes (`pc-app`, `rpi-controller`) bridge their isolated protocol queues via TCP localhost, enabling observable message flow without blocking the ECS scheduler.

## Requirements

### Requirement: Bidirectional Queue Bridge

A `TransportPlugin` SHALL connect each process's `OutboundProtocolQueue` to the peer's `InboundProtocolQueue` over TCP localhost without blocking Bevy `Update` or `FixedUpdate` schedules.

#### Scenario: Server bridges outbound to socket

- GIVEN `rpi-controller` runs as server on `127.0.0.1:PORT`
- WHEN `OutboundProtocolQueue` contains an `EventEnvelope`
- THEN it SHALL be serialized to JSON and written to the TCP socket within one frame tick

#### Scenario: Client bridges socket to inbound

- GIVEN `pc-app` connects as client to `127.0.0.1:PORT`
- WHEN a JSON line is received on the TCP socket
- THEN it SHALL be deserialized as `EventEnvelope` and pushed to `InboundProtocolQueue` within one frame tick

#### Scenario: I/O never blocks the ECS scheduler

- GIVEN Bevy is running with `ScheduleRunnerPlugin`
- WHEN transport performs socket read or write
- THEN it MUST NOT block the ECS main thread — I/O SHALL run on `IoTaskPool` or a separate async task

### Requirement: JSON Line Framing

Messages SHALL use newline-delimited JSON — one `EventEnvelope` per line — enabling inspection with `nc` or `telnet`.

#### Scenario: Envelope round-trip over wire

- GIVEN an `EventEnvelope` with any valid `Message` variant
- WHEN serialized and sent over TCP
- THEN the receiver SHALL deserialize it to an identical `EventEnvelope`

#### Scenario: Malformed line handled gracefully

- GIVEN a TCP line that is not valid JSON
- WHEN the receiver attempts deserialization
- THEN it MUST log the error and continue without crashing or corrupting `InboundProtocolQueue`

### Requirement: Observable Message Flow

The transport SHALL log every envelope with `[TX]` or `[RX]` prefix, showing source, message type, and `correlation_id`.

#### Scenario: Demo flow OvenDetected → Command → Status

- GIVEN `rpi-controller` (server) and `pc-app` (client) both running with `--transport tcp`
- WHEN `rpi-controller` emits `OvenDetected`
- THEN `pc-app` SHALL log `[RX] OvenDetected from rpi-controller` within 1 second
- WHEN `pc-app` sends `SetTargetTemperature`
- THEN `rpi-controller` SHALL log `[RX] SetTargetTemperature from pc-app` within 1 second
- WHEN `rpi-controller` responds with `OvenStatusUpdated`
- THEN `pc-app` SHALL log `[RX] OvenStatusUpdated from rpi-controller` within 1 second

### Requirement: Graceful Disconnect Handling

The transport MUST exit cleanly when the peer disconnects. No reconnect or backoff logic is required for v1.

#### Scenario: Server detects client disconnect

- GIVEN `rpi-controller` (server) and `pc-app` (client) connected
- WHEN the client disconnects
- THEN the server SHALL log the disconnection and SHALL NOT crash
- AND the server SHALL continue running (queues accumulate locally)

#### Scenario: Client fails to connect

- GIVEN `pc-app` (client) starts before `rpi-controller` (server)
- WHEN the client fails to connect
- THEN it SHALL log the error and MAY retry once per frame tick
- AND it MUST NOT block the ECS schedule while waiting
