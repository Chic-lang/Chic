# Distributed Actors

Deterministic actor messaging surface for capability-secure IPC. The native
adapter provides a loopback transport that still exercises serialization,
capability validation, and deterministic retry/backoff; the WASM side stubs
collectives with diagnostics but shares the same actor semantics.

- Capability tokens (`Std.Distributed.CapabilityToken`) carry an `id` and
  deterministic `nonce`. The native adapter validates both before accepting a
  send (`runtime_adapter/native/distributed.rs`).
- Messages are serialized as JSON (`WireEnvelope`) containing the sender,
  attempt, backoff, and payload bytes. The serialized form is stored on the
  `ActorMessage` delivered to receivers for replay/debugging.
- Retry/backoff is linear and deterministic: `backoff_ns * attempt`. All
  attempts are recorded in `SendRecord` telemetry even though loopback delivery
  succeeds on the first attempt.
- Networking is an explicit effect: actor helpers in `Std.Distributed.actor.ch`
  declare `effects(network)`, and the compiler emits `[NET100]` if a caller uses
  networking without declaring the effect.
- Collectives remain loopback/stubbed; metrics/diagnostics are logged instead of
  performing network I/O.

Tests: `tests/distributed_runtime.rs` exercises capability validation,
deterministic serialization, backoff logging, and WASM stub diagnostics.
