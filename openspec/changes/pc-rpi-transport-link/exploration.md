## Exploration: pc-rpi-transport-link

### Current State
The system consists of two independent Bevy ECS applications (`pc-app` and `rpi-controller`) that share a `protocol` crate. Both applications use an internal, in-memory pattern to manage I/O via `InboundProtocolQueue` and `OutboundProtocolQueue` (Resources). Currently, these queues are isolated; there is no mechanism to move messages from the `Outbound` queue of one process to the `Inbound` queue of the other.

### Affected Areas
- `pc-app/src/resources.rs` — The `Inbound/OutboundProtocolQueue` must be drained/filled by the new transport layer.
- `rpi-controller/src/resources.rs` — The `Inbound/OutboundProtocolQueue` must be drained/filled by the new transport layer.
- `protocol/src/` — May need serialization/deserialization helpers (though `serde` is already used).
- New crate/module: A transport implementation is required.

### Approaches

| Approach | Pros | Cons | Complexity |
|----------|------|------|------------|
| **1. In-memory Demo (Single Process)** | Extremely fast to implement; zero network/IO overhead; easiest for debugging logic. | Doesn't test actual process boundary; not a "real" transport; hard to simulate network latency/failures. | Low |
| **2. TCP Localhost (Dual Process)** | Best simulation of real distributed system; tests serialization/deserialization; allows running RPi on actual hardware later; highly observable (e.g., using Wireshark/Tcpdump). | Requires managing two processes; requires handling connection/reconnection logic; slightly more complex setup. | Medium |
| **3. Stdin/Stdout (JSON Lines)** | Very simple to implement; works well with shell pipes; great for CLI-based testing. | Not standard for bidirectional "network-like" protocols; harder to scale to multiple clients; harder to integrate with Bevy's async needs without blocking. | Low |
| **4. Serial/Loopback** | Closest to real Raspberry Pi hardware (GPIO/UART). | Harder to simulate on PC without extra hardware or virtual serial ports; more complex driver/library management. | High |

### Recommendation
**Approach 2: TCP Localhost (Dual Process)**.

**Why:** For an academic project, observing the "bridge" between two distinct processes is crucial. It reinforces the concept of distributed systems (independent lifecycles, serialization, network boundaries) while remaining highly compatible with the eventual move to real hardware (where TCP/IP is also common). 

To keep it clean:
1. Create a new crate `transport` or a dedicated module within a `platform` crate.
2. Implement a `TransportPlugin` for both `pc-app` and `rpi-controller`.
3. The plugin will run a background task (using `tokio` or a similar lightweight executor, or even just a non-blocking socket check in a Bevy system) that:
   - Drains `OutboundProtocolQueue` $\rightarrow$ Sends via Socket.
   - Receives via Socket $\rightarrow$ Populates `InboundProtocolQueue`.

### Implementation Strategy for v1
- **Architecture**: The transport should act as an "Adapter" (Hexagonal Architecture). The ECS domain (systems/resources) should NOT know that TCP exists. They only know about the `ProtocolQueue` resources.
- **Crate Structure**: A new crate `transport` is recommended. It will depend on `protocol`. `pc-app` and `rpi-controller` will depend on `transport`.
- **Observability**: The demo should run two terminal windows. One shows the "PC" logs (commands sent, status received), and the other shows the "RPi" logs (commands processed, temperature changes).

### Risks
- **Blocking the Main Loop**: If the transport implementation uses blocking I/O, it will freeze the Bevy ECS simulation. Must use non-blocking or async-to-sync bridging.
- **Complexity Creep**: Don't implement retry logic, encryption, or sophisticated error recovery in v1. Keep it to "Connect, Send, Receive".

### Ready for Proposal
**Yes** — The next step is to define the technical spec for the `transport` crate and the `TransportPlugin` interface.
