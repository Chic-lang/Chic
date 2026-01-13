# Std.Data mapping layer

Std.Data ships a built-in, Dapper-like mapper that sits directly under `Std.Data`. It is async-first, streaming-friendly, and uses compile-time reflection metadata to generate cached mapping plans—no per-row reflection or hidden allocations.

## Auto-mapping

```chic
import Std.Async;
import Std.Data;
import Std.Data.Mapping;

public async Task ConsumeAsync(DbConnection connection, CancellationToken ct)
{
    await connection.OpenAsync(ct);

    // Columns `Id`/`name` map to fields on UserRow (case-insensitive by default)
    var rows = await connection.QueryAsync<UserRow>("select Id, name from users", ct: ct);
    while (await rows.MoveNextAsync(ct))
    {
        let user = rows.Current;
        Console.WriteLine(user.Name);
    }
    await rows.DisposeAsync();
}
```

- Case-insensitive matching is the default. Set `MappingOptions.CaseSensitive = true` to enforce case.
- Optional underscore folding strips `_` before matching (`MappingOptions.UnderscoreToCamel = true`), so `user_id` matches `UserId`.
- Missing columns leave the target field/property at its default value. Nulls flowing into non-nullable targets raise `DataMappingException`.
- Plans are cached per `(T, schema signature)` behind a reader/writer lock so repeated executions avoid rebuilding.

## Manual mapping

When you need full control, pass a mapper:

```chic
RowFactory<OrderLine> map = (Row row) => new OrderLine(
    row.GetInt32("id"),
    row.GetString("sku"),
    row.GetDecimal("price")
);
var lines = await connection.QueryAsync<OrderLine, object?>("select ...", null, map, ct: ct);
```

`Row` exposes name/ordinal lookups plus typed helpers (`GetInt32`, `GetString`, `TryGet<T>`, `GetNullable<T>`) that cache ordinals internally.

## Column/scalar helpers

- `QueryColumnAsync<T>` maps the first column of each row.
- `ExecuteScalarAsync<T>` converts the first column of the first row to `T`.
- `QuerySingleAsync`/`QuerySingleOrDefaultAsync`/`QueryFirstOrDefaultAsync` handle single-row projections with deterministic errors when counts are unexpected.

## Parameter binding

`args` accepts:

- Plain objects/structs with public fields/properties,
- `HashMap<string, object?>`, or
- Explicit `DbParameter[]`.

Names are bound without prefixes; providers translate to `@`/`:`/`$` as needed. Writer plans are cached per argument type.

## Streaming and cancellation

`AsyncEnumerable<T>` wraps the underlying `DbDataReader` and advances one row per `MoveNextAsync`. Cancellation tokens flow directly into `DbDataReader.ReadAsync`; cancellations close the reader/command deterministically before surfacing `TaskCanceledException`.
