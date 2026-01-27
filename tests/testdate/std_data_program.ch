namespace Exec.StdData;

import Std;
import Std.Async;
import Std.Core;
import Std.Data;
import Std.Data.Mapping;
import Std.Collections;
import Std.Numeric;

public static class Program
{
    public static int Main()
    {
        if (!TestConnectionLifecycle())
        {
            return 1;
        }
        if (!TestExecuteNonQueryAndScalar())
        {
            return 2;
        }
        if (!TestExecuteReaderStreaming())
        {
            return 3;
        }
        if (!TestCancellationPropagation())
        {
            return 4;
        }
        if (!TestTransactions())
        {
            return 5;
        }
        if (!TestParameters())
        {
            return 6;
        }
        if (!TestConnectionStringBuilder())
        {
            return 7;
        }
        if (!TestAutoMapping())
        {
            return 8;
        }
        if (!TestMappingOptions())
        {
            return 9;
        }
        if (!TestManualMapping())
        {
            return 10;
        }
        if (!TestColumnMapping())
        {
            return 11;
        }
        if (!TestParameterBindingExtensions())
        {
            return 12;
        }
        if (!TestStreamingEnumeration())
        {
            return 13;
        }
        if (!TestMappingCaching())
        {
            return 14;
        }
        if (!TestMappingCancellation())
        {
            return 15;
        }
        return 0;
    }

    private static bool TestConnectionLifecycle()
    {
        var scripts = new FakeCommandScript[] { FakeCommandScript.NonQuery("noop", 0) };
        var factory = new FakeProviderFactory(scripts, "local", "default", "1.0");
        DbProviderRegistry.Register("fake-lifecycle", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-lifecycle",
            "DataSource=local;Database=default"
        );
        if (connection.State != ConnectionState.Closed)
        {
            return false;
        }
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        Runtime.BlockOn(connection.OpenAsync(ct));
        if (connection.State != ConnectionState.Open)
        {
            return false;
        }
        Runtime.BlockOn(connection.CloseAsync());
        if (connection.State != ConnectionState.Closed)
        {
            return false;
        }
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestExecuteNonQueryAndScalar()
    {
        var scripts = new FakeCommandScript[]
        {
            FakeCommandScript.NonQuery("update", 3),
            FakeCommandScript.Scalar("scalar", 42),
        };
        var factory = new FakeProviderFactory(scripts, "cmd-source", "cmd-db", "2.0");
        DbProviderRegistry.Register("fake-commands", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-commands",
            "DataSource=cmd;Database=cmd-db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        var command = (FakeCommand)connection.CreateCommand();
        command.CommandText = "update";
        let affected = command.ExecuteNonQuery();
        if (affected != 3)
        {
            return false;
        }
        command.CommandText = "scalar";
        let scalar = command.ExecuteScalar();
        if ((int)scalar != 42)
        {
            return false;
        }
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestExecuteReaderStreaming()
    {
        var rows = new object?[][]
        {
            new object?[] { 1, "alpha" },
            new object?[] { 2, "beta" },
        };
        var resultSet = new FakeResultSet(new string[] { "id", "name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("select", resultSet) };
        var factory = new FakeProviderFactory(scripts, "reader", "db", "1.1");
        DbProviderRegistry.Register("fake-reader", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-reader",
            "DataSource=reader;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        var command = (FakeCommand)connection.CreateCommand();
        command.CommandText = "select";
        let readerTask = command.ExecuteReaderAsync(
            CommandBehavior.Default,
            CoreIntrinsics.DefaultValue<CancellationToken>()
        );
        Runtime.BlockOn(readerTask);
        var reader = (FakeDataReader)TaskRuntime.GetResult<DbDataReader>(readerTask);
        if (reader.FieldCount != 2)
        {
            return false;
        }

        let readFirst = reader.Read();
        if (!readFirst)
        {
            return false;
        }
        if (reader.GetInt32(0) != 1 || reader.GetString(1) != "alpha")
        {
            return false;
        }
        if (reader["name"] as string != "alpha")
        {
            return false;
        }

        let readSecondTask = reader.ReadAsync(CoreIntrinsics.DefaultValue<CancellationToken>());
        Runtime.BlockOn(readSecondTask);
        if (!TaskRuntime.GetResult<bool>(readSecondTask))
        {
            return false;
        }
        if (reader.GetInt32(reader.GetOrdinal("id")) != 2)
        {
            return false;
        }

        let readThirdTask = reader.ReadAsync(CoreIntrinsics.DefaultValue<CancellationToken>());
        Runtime.BlockOn(readThirdTask);
        if (TaskRuntime.GetResult<bool>(readThirdTask))
        {
            return false;
        }

        let nextResultTask = reader.NextResultAsync(CoreIntrinsics.DefaultValue<CancellationToken>());
        Runtime.BlockOn(nextResultTask);
        if (TaskRuntime.GetResult<bool>(nextResultTask))
        {
            return false;
        }

        reader.dispose(ref reader);
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestCancellationPropagation()
    {
        var scripts = new FakeCommandScript[] { FakeCommandScript.NonQuery("cancel", 1) };
        var factory = new FakeProviderFactory(scripts, "cancel", "canceldb", "1.0");
        DbProviderRegistry.Register("fake-cancel", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-cancel",
            "DataSource=cancel;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        var command = (FakeCommand)connection.CreateCommand();
        command.CommandText = "cancel";
        var source = CancellationTokenSource.Create();
        source.Cancel();
        try
        {
            let task = command.ExecuteNonQueryAsync(source.Token());
            Runtime.BlockOn(task);
            return false;
        }
        catch (Std.TaskCanceledException)
        {
            connection.dispose(ref connection);
            return true;
        }
    }

    private static bool TestTransactions()
    {
        var scripts = new FakeCommandScript[] { FakeCommandScript.NonQuery("txn", 0) };
        var factory = new FakeProviderFactory(scripts, "txn", "db", "1.0");
        DbProviderRegistry.Register("fake-txn", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-txn",
            "DataSource=txn;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        let txTask = connection.BeginTransactionAsync(
            IsolationLevel.Serializable,
            CoreIntrinsics.DefaultValue<CancellationToken>()
        );
        Runtime.BlockOn(txTask);
        var tx = (FakeTransaction)TaskRuntime.GetResult<DbTransaction>(txTask);
        if (tx.IsolationLevel != IsolationLevel.Serializable)
        {
            return false;
        }
        tx.Commit();
        if (!tx.Committed)
        {
            return false;
        }

        let rollbackTask = connection.BeginTransactionAsync(
            IsolationLevel.ReadCommitted,
            CoreIntrinsics.DefaultValue<CancellationToken>()
        );
        Runtime.BlockOn(rollbackTask);
        var rollbackTx = (FakeTransaction)TaskRuntime.GetResult<DbTransaction>(rollbackTask);
        rollbackTx.Rollback();
        if (!rollbackTx.RolledBack)
        {
            return false;
        }
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestParameters()
    {
        var scripts = new FakeCommandScript[] { FakeCommandScript.NonQuery("params", 0) };
        var factory = new FakeProviderFactory(scripts, "params", "db", "1.0");
        DbProviderRegistry.Register("fake-params", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-params",
            "DataSource=params;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        var command = (FakeCommand)connection.CreateCommand();
        command.CommandText = "params";
        var parameter = (FakeParameter)command.CreateParameter();
        parameter.ParameterName = "id";
        parameter.DbType = DbType.Int32;
        parameter.Value = 5;
        parameter.IsNullable = false;
        parameter.Size = 4;
        command.Parameters.Add(parameter);
        if (command.Parameters.Count != 1)
        {
            return false;
        }
        if (command.Parameters[0] != parameter)
        {
            return false;
        }
        if (command.Parameters["id"] != parameter)
        {
            return false;
        }
        if (!command.Parameters.Remove(parameter))
        {
            return false;
        }
        if (command.Parameters.Count != 0)
        {
            return false;
        }
        command.Parameters.Add(parameter);
        var other = (FakeParameter)command.CreateParameter();
        other.ParameterName = "name";
        command.Parameters.Add(other);
        command.Parameters.Clear();
        if (command.Parameters.Count != 0)
        {
            return false;
        }
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestConnectionStringBuilder()
    {
        var builder = new DbConnectionStringBuilder("DataSource=alpha;Database=beta;User=user1");
        if (builder.Count != 3)
        {
            return false;
        }
        if (builder["DataSource"] != "alpha" || builder["Database"] != "beta")
        {
            return false;
        }
        if (!builder.ContainsKey("User"))
        {
            return false;
        }
        builder.Set("Timeout", "30");
        if (builder.Count != 4)
        {
            return false;
        }
        let text = builder.ToString();
        if (text.Length == 0)
        {
            return false;
        }
        return true;
    }

    private static bool TestAutoMapping()
    {
        var rows = new object?[][]
        {
            new object?[] { 1, "alpha" },
            new object?[] { 2, "beta" },
        };
        var resultSet = new FakeResultSet(new string[] { "Id", "name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-auto", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-auto-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-auto", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-auto",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        let query = connection.QueryAsync<UserRow>("map-auto");
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<UserRow>>(query);
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        let firstTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(firstTask);
        if (!TaskRuntime.GetResult<bool>(firstTask))
        {
            return false;
        }
        let first = enumerable.Current;
        if (first.Id != 1 || first.Name != "alpha" || first.Age != 0)
        {
            return false;
        }

        let secondTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(secondTask);
        if (!TaskRuntime.GetResult<bool>(secondTask))
        {
            return false;
        }
        let second = enumerable.Current;
        if (second.Id != 2 || second.Name != "beta" || second.Age != 0)
        {
            return false;
        }

        let endTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(endTask);
        if (TaskRuntime.GetResult<bool>(endTask))
        {
            return false;
        }
        Runtime.BlockOn(enumerable.DisposeAsync());
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestMappingOptions()
    {
        var rows = new object?[][]
        {
            new object?[] { 7, "Display" },
        };
        var resultSet = new FakeResultSet(new string[] { "user_id", "display_name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-snake", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-snake-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-snake", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-snake",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        var options = MappingOptions.Default();
        options.UnderscoreToCamel = true;
        let query = connection.QueryAsync<SnakeCaseRow>(
            "map-snake",
            args: null,
            tx: null,
            timeoutSeconds: null,
            type: null,
            options: options
        );
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<SnakeCaseRow>>(query);
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        let moveTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(moveTask);
        if (!TaskRuntime.GetResult<bool>(moveTask))
        {
            return false;
        }
        let row = enumerable.Current;
        Runtime.BlockOn(enumerable.DisposeAsync());
        connection.dispose(ref connection);
        return row.UserId == 7 && row.DisplayName == "Display";
    }

    private static bool TestManualMapping()
    {
        var rows = new object?[][]
        {
            new object?[] { 3, "gamma" },
            new object?[] { 4, "delta" },
        };
        var resultSet = new FakeResultSet(new string[] { "Id", "Name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-manual", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-manual-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-manual", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-manual",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        RowFactory<string> mapper = (Row row) =>
        {
            let id = row.GetInt32("Id");
            let name = row.GetString("Name");
            return id.ToString() + ":" + name;
        };
        let query = connection.QueryAsync<string, object?>("map-manual", null, mapper);
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<string>>(query);
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        let firstTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(firstTask);
        if (!TaskRuntime.GetResult<bool>(firstTask))
        {
            return false;
        }
        if (enumerable.Current != "3:gamma")
        {
            return false;
        }
        let secondTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(secondTask);
        if (!TaskRuntime.GetResult<bool>(secondTask))
        {
            return false;
        }
        if (enumerable.Current != "4:delta")
        {
            return false;
        }
        Runtime.BlockOn(enumerable.DisposeAsync());
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestColumnMapping()
    {
        var rows = new object?[][]
        {
            new object?[] { 9, "unused" },
            new object?[] { 10, "other" },
        };
        var resultSet = new FakeResultSet(new string[] { "Value", "Other" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-column", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-column-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-column", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-column",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        let query = connection.QueryColumnAsync<int>("map-column");
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<int>>(query);
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        let firstTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(firstTask);
        if (!TaskRuntime.GetResult<bool>(firstTask) || enumerable.Current != 9)
        {
            return false;
        }
        let secondTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(secondTask);
        if (!TaskRuntime.GetResult<bool>(secondTask) || enumerable.Current != 10)
        {
            return false;
        }
        Runtime.BlockOn(enumerable.DisposeAsync());
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestParameterBindingExtensions()
    {
        var scripts = new FakeCommandScript[] { FakeCommandScript.NonQuery("map-params", 1) };
        var factory = new FakeProviderFactory(scripts, "map-params-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-params", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-params",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        var args = new ParameterArgs();
        args.Id = 5;
        args.Name = "alpha";
        let execTask = connection.ExecuteAsync<ParameterArgs>("map-params", args);
        Runtime.BlockOn(execTask);
        var command = connection.LastCommand;
        if (command == null)
        {
            return false;
        }
        if (command.Parameters.Count != 2)
        {
            return false;
        }
        if (command.Parameters["Id"].Value as int? != 5)
        {
            return false;
        }
        if (command.Parameters["Name"].Value as string != "alpha")
        {
            return false;
        }

        var map = new HashMap<string, object?>();
        let _ = map.Insert("Age", 30, out var _);
        let execMap = connection.ExecuteAsync<HashMap<string, object?>>("map-params", map);
        Runtime.BlockOn(execMap);
        map.dispose(ref map);
        command = connection.LastCommand;
        if (command == null)
        {
            return false;
        }
        if (command.Parameters.Count != 1)
        {
            return false;
        }
        if (command.Parameters["Age"].Value as int? != 30)
        {
            return false;
        }
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestStreamingEnumeration()
    {
        var rows = new object?[][]
        {
            new object?[] { 11, "first" },
            new object?[] { 12, "second" },
        };
        var resultSet = new FakeResultSet(new string[] { "Id", "Name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-stream", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-stream-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-stream", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-stream",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        let query = connection.QueryAsync<UserRow>("map-stream");
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<UserRow>>(query);
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        if (connection.LastCommand == null)
        {
            return false;
        }
        var reader = connection.LastCommand.LastReader;
        if (reader == null)
        {
            return false;
        }
        if (reader.CurrentRowIndex != -1)
        {
            return false;
        }
        let moveTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(moveTask);
        if (!TaskRuntime.GetResult<bool>(moveTask))
        {
            return false;
        }
        if (reader.CurrentRowIndex != 0)
        {
            return false;
        }
        Runtime.BlockOn(enumerable.DisposeAsync());
        connection.dispose(ref connection);
        return true;
    }

    private static bool TestMappingCaching()
    {
        let before = MappingDiagnostics.CachedPlans<UserRow>();
        var rows = new object?[][] { new object?[] { 21, "cache" } };
        var resultSet = new FakeResultSet(new string[] { "Id", "Name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-cache", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-cache-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-cache", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-cache",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));

        let query = connection.QueryAsync<UserRow>("map-cache");
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<UserRow>>(query);
        let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
        let moveTask = enumerable.MoveNextAsync(ct);
        Runtime.BlockOn(moveTask);
        Runtime.BlockOn(enumerable.DisposeAsync());

        let afterFirst = MappingDiagnostics.CachedPlans<UserRow>();
        let queryTwo = connection.QueryAsync<UserRow>("map-cache");
        Runtime.BlockOn(queryTwo);
        var enumerableTwo = TaskRuntime.GetResult<AsyncEnumerable<UserRow>>(queryTwo);
        let moveTaskTwo = enumerableTwo.MoveNextAsync(ct);
        Runtime.BlockOn(moveTaskTwo);
        Runtime.BlockOn(enumerableTwo.DisposeAsync());
        let afterSecond = MappingDiagnostics.CachedPlans<UserRow>();
        connection.dispose(ref connection);
        return afterFirst > 0 && afterFirst == afterSecond && afterFirst >= before;
    }

    private static bool TestMappingCancellation()
    {
        var rows = new object?[][] { new object?[] { 30, "cancel" } };
        var resultSet = new FakeResultSet(new string[] { "Id", "Name" }, rows);
        var scripts = new FakeCommandScript[] { FakeCommandScript.Reader("map-cancel", resultSet) };
        var factory = new FakeProviderFactory(scripts, "map-cancel-source", "mapdb", "1.0");
        DbProviderRegistry.Register("fake-map-cancel", factory);
        var connection = (FakeConnection)DbProviderRegistry.CreateConnection(
            "fake-map-cancel",
            "DataSource=map;Database=db"
        );
        Runtime.BlockOn(connection.OpenAsync(CoreIntrinsics.DefaultValue<CancellationToken>()));
        let query = connection.QueryAsync<UserRow>("map-cancel");
        Runtime.BlockOn(query);
        var enumerable = TaskRuntime.GetResult<AsyncEnumerable<UserRow>>(query);
        var source = CancellationTokenSource.Create();
        source.Cancel();
        try
        {
            let moveTask = enumerable.MoveNextAsync(source.Token());
            Runtime.BlockOn(moveTask);
            return false;
        }
        catch (Std.TaskCanceledException)
        {
            var reader = connection.LastCommand == null ? null : connection.LastCommand.LastReader;
            if (reader == null)
            {
                return false;
            }
            let current = reader.CurrentRowIndex;
            connection.dispose(ref connection);
            return current == -1;
        }
    }
}
