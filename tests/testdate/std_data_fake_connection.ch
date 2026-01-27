namespace Exec.StdData;

import Std.Async;
import Std.Data;

public class FakeConnection : DbConnection
{
    private ConnectionState _state;
    private string _database;
    private string _dataSource;
    private string _serverVersion;
    private FakeTransaction? _activeTransaction;
    private FakeCommandScript[] _scripts;
    private FakeCommand? _lastCommand;

    public init(FakeCommandScript[] scripts, string dataSource, string database, string serverVersion)
    {
        _scripts = scripts;
        _dataSource = dataSource;
        _database = database;
        _serverVersion = serverVersion;
        _state = ConnectionState.Closed;
    }

    public override ConnectionState State => _state;

    public override string Database => _database;

    public override string DataSource => _dataSource;

    public override string ServerVersion => _serverVersion;

    public override DbCommand CreateCommand()
    {
        var command = new FakeCommand(this, _scripts);
        _lastCommand = command;
        return command;
    }

    public override Task OpenAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Open canceled");
        }
        _state = ConnectionState.Connecting;
        var builder = new DbConnectionStringBuilder(ConnectionString);
        if (builder.TryGetValue("DataSource", out var dataSource))
        {
            _dataSource = dataSource;
        }
        if (builder.TryGetValue("Database", out var database))
        {
            _database = database;
        }
        _state = ConnectionState.Open;
        return Std.Async.TaskRuntime.CompletedTask();
    }

    public override Task CloseAsync()
    {
        _state = ConnectionState.Closed;
        _activeTransaction = null;
        return Std.Async.TaskRuntime.CompletedTask();
    }

    public override Task<DbTransaction> BeginTransactionAsync(
        IsolationLevel level,
        CancellationToken ct
    )
    {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Begin transaction canceled");
        }
        if (_state != ConnectionState.Open)
        {
            throw new DbConnectionException("Connection must be open for transactions");
        }
        var transaction = new FakeTransaction(this, level);
        _activeTransaction = transaction;
        return Std.Async.TaskRuntime.FromResult<DbTransaction>(transaction);
    }

    public void CompleteTransaction()
    {
        _activeTransaction = null;
    }

    public FakeCommand? LastCommand => _lastCommand;
}
