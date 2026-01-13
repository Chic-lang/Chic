namespace Exec.StdData;

import Std.Async;
import Std.Data;

public class FakeCommand : DbCommand
{
    private FakeParameterCollection _parameters;
    private FakeCommandScript[] _scripts;
    private FakeDataReader? _lastReader;

    public init(FakeConnection connection, FakeCommandScript[] scripts)
    {
        _parameters = new FakeParameterCollection();
        _scripts = scripts;
        Connection = connection;
    }

    public override DbParameterCollection Parameters => _parameters;

    public override DbParameter CreateParameter()
    {
        return new FakeParameter();
    }

    public override Task<int> ExecuteNonQueryAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        EnsureReady(ct);
        let script = ResolveScript();
        return Std.Async.TaskRuntime.FromResult(script.RowsAffected);
    }

    public override Task<object?> ExecuteScalarAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        EnsureReady(ct);
        let script = ResolveScript();
        return Std.Async.TaskRuntime.FromResult(script.ScalarResult);
    }

    public override Task<DbDataReader> ExecuteReaderAsync(
        CommandBehavior behavior,
        CancellationToken ct
    )
    {
        ThrowIfDisposed();
        EnsureReady(ct);
        let script = ResolveScript();
        if (!script.HasResultSet)
        {
            throw new DbCommandException("No result set for command");
        }
        var reader = new FakeDataReader(script.ResultSet);
        _lastReader = reader;
        return Std.Async.TaskRuntime.FromResult<DbDataReader>(reader);
    }

    private FakeCommandScript ResolveScript()
    {
        var idx = 0;
        while (idx < _scripts.Length)
        {
            if (_scripts[idx].CommandText == CommandText)
            {
                return _scripts[idx];
            }
            idx += 1;
        }
        throw new DbCommandException("Script not found for command: " + CommandText);
    }

    private void EnsureReady(CancellationToken ct)
    {
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Command canceled");
        }
        var connection = (FakeConnection)Connection;
        if (connection.State != ConnectionState.Open)
        {
            throw new DbCommandException("Connection not open");
        }
    }

    public FakeDataReader? LastReader => _lastReader;
}
