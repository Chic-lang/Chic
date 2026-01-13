# Std.Data overview

Std.Data is the async-first, provider-agnostic database surface for Chic. The core package defines contracts for connections, commands, readers, parameters, and provider factories without bundling a concrete driver.

## Key ideas
- Async is canonical: `OpenAsync`, `ExecuteReaderAsync`, and other operations take `CancellationToken` and return `Task`/`Task<T>`. Synchronous helpers block on the async path.
- Provider-agnostic core: providers live in separate packages and plug in via `DbProviderFactory` + `DbProviderRegistry`.
- Deterministic cleanup: `DbConnection`, `DbCommand`, `DbDataReader`, and `DbTransaction` expose `dispose(ref this)` so `using` blocks release underlying resources.

## Mapping layer
- `Std.Data.Mapping` adds built-in, Dapper-style helpers: `QueryAsync<T>` streams rows via `DbDataReader.ReadAsync` and auto-maps columns to fields/properties with cached reflection metadata (case-insensitive by default).
- `MappingOptions` support case-sensitive matching and optional underscore-to-camel folding.
- Manual mappers (`RowFactory<T>`/`RowFactoryAsync<T>`) and column/scalar helpers cover explicit materialization paths without buffering.
- Parameter binding accepts args objects/structs, `HashMap<string, object?>`, or explicit `DbParameter[]`; names are left unprefixed so providers can translate `@`/`:`/`$`.
- See `docs/std/data/mapping.md` for detailed usage and error semantics.

## Usage (async-first)
```chic
import Std.Async;
import Std.Data;

public async Task<int> CountUsersAsync(CancellationToken ct)
{
    DbProviderRegistry.Register("demo", new DemoProviderFactory());

    var connection = DbProviderRegistry.CreateConnection("demo", "DataSource=local;Database=app");
    await connection.OpenAsync(ct);

    var command = connection.CreateCommand();
    command.CommandText = "select count(*) from users";

    var scalar = await command.ExecuteScalarAsync(ct);
    return (int)scalar;
}
```

## Provider model
- Providers implement `DbProviderFactory` (create connection/command/parameter) and concrete subclasses of `DbConnection`, `DbCommand`, `DbDataReader`, `DbTransaction`, and `DbParameterCollection`.
- Consumers register a factory with `DbProviderRegistry.Register(invariantName, factory)` and resolve it with `DbProviderRegistry.Resolve` or `CreateConnection`.
- Connection strings are parsed with `DbConnectionStringBuilder`; provider-specific keys are stored as plain `key=value` segments.

## Resource disposal
- All core objects expose `dispose(ref this)`; prefer `using` blocks to ensure connections, readers, and transactions close deterministically.
- Sync helpers (`Open`, `ExecuteReader`, `Commit`) block on their async counterparts—there is no alternate code path.
- Cancellation propagates via `CancellationToken` and raises `TaskCanceledException` when requested.
