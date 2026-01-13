# Provider authoring guide

Std.Data keeps the contracts small and async-first so providers can slot in without modifying the core.

## Required types
- `DbProviderFactory`: create connections, commands, and parameters for your backend.
- `DbConnection`: hold connection state, parse/apply connection strings, and implement `OpenAsync`, `CloseAsync`, and `BeginTransactionAsync`.
- `DbCommand`: manage text/type/timeout, expose a provider-specific `DbParameterCollection`, and implement async execute paths (`ExecuteNonQueryAsync`, `ExecuteScalarAsync`, `ExecuteReaderAsync`). `PrepareAsync` is optional.
- `DbDataReader`: stream rows with `ReadAsync`, optional `NextResultAsync`, typed getters, and indexers by ordinal/name.
- `DbTransaction`: wrap commit/rollback semantics with async methods and sync wrappers (`Commit`/`Rollback`) that block on the async calls.
- `DbParameter` + `DbParameterCollection`: store names, types, directions, nullability, sizes, and values; index parameters by ordinal and name.

## Design notes
- Async-first: implement async operations with real async I/O; leave sync helpers alone (they call `Runtime.BlockOn` in the base classes).
- Cancellation: honor `CancellationToken` in every async entry point; surface `TaskCanceledException` when requested.
- State: keep `ConnectionState` accurate across open/close/failure paths and ensure transactions clean up underlying handles.
- Disposal: implement `dispose(ref this)` to release native cursors, sockets, or pooled handles; closing a connection/command/reader should be safe to call multiple times.
- Connection strings: use `DbConnectionStringBuilder` to parse/serialize provider-specific keys; accept plain `key=value` pairs.
- Parameter naming: `Std.Data.Mapping` binds names without prefixes. Map names to the provider-specific marker (`@`/`:`/`$`) inside your command implementation.
- Mapping helpers depend on `GetName`, `GetOrdinal`, `IsDBNull`, and typed getters (`GetInt32`/`GetString`/`GetDecimal`/etc.). Ensure these methods are fast and deterministic so auto-mapping stays allocation-light.

## Registration
- Register factories with `DbProviderRegistry.Register("provider-invariant-name", factory)`; the registry replaces existing entries with the same invariant name.
- Consumers can resolve factories with `DbProviderRegistry.Resolve` or construct connections directly with `DbProviderRegistry.CreateConnection`.

## Testing recommendations
- Build fake/in-memory providers to validate command/transaction ordering, parameter binding, cancellation, and streaming behaviors.
- Ensure sync wrappers (`Open`, `ExecuteReader`, `Commit`) and async methods share the same code path to avoid divergence.
