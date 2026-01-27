namespace Std.Data;
import Std;
import Std.Async;
import Std.Core;
import Std.Runtime;
import Std.Testing;
/// <summary>Represents an open connection to a data source.</summary>
public abstract class DbConnection
{
    private bool _disposed;
    private string _connectionString;
    protected init() {
        _connectionString = StringRuntime.Create();
    }
    /// <summary>Gets or sets the connection string.</summary>
    public string ConnectionString {
        get {
            ThrowIfDisposed();
            return _connectionString;
        }
        set {
            ThrowIfDisposed();
            _connectionString = value;
        }
    }
    /// <summary>Gets the current state of the connection.</summary>
    public abstract ConnectionState State {
        get;
    }
    /// <summary>Gets the database name.</summary>
    public abstract string Database {
        get;
    }
    /// <summary>Gets the data source description.</summary>
    public abstract string DataSource {
        get;
    }
    /// <summary>Gets the server version string.</summary>
    public abstract string ServerVersion {
        get;
    }
    /// <summary>Creates a command associated with the connection.</summary>
    public virtual DbCommand CreateCommand() {
        ThrowIfDisposed();
        throw new DbException("DbConnection.CreateCommand not implemented");
        return CoreIntrinsics.DefaultValue <DbCommand >();
    }
    /// <summary>Opens the connection asynchronously.</summary>
    public virtual Task OpenAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Open canceled");
        }
        throw new DbException("DbConnection.OpenAsync not implemented");
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Closes the connection asynchronously.</summary>
    public virtual Task CloseAsync() {
        ThrowIfDisposed();
        throw new DbException("DbConnection.CloseAsync not implemented");
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Begins a transaction asynchronously.</summary>
    public virtual Task <DbTransaction >BeginTransactionAsync(IsolationLevel level, CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("BeginTransaction canceled");
        }
        throw new DbException("DbConnection.BeginTransactionAsync not implemented");
        return TaskRuntime.FromResult <DbTransaction >(CoreIntrinsics.DefaultValue <DbTransaction >());
    }
    /// <summary>Opens the connection, blocking until complete.</summary>
    public void Open() {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = OpenAsync(ct);
        Runtime.BlockOn(task);
    }
    /// <summary>Closes the connection, blocking until complete.</summary>
    public void Close() {
        ThrowIfDisposed();
        let task = CloseAsync();
        Runtime.BlockOn(task);
    }
    /// <summary>Begins a transaction, blocking until complete.</summary>
    public DbTransaction BeginTransaction(IsolationLevel level) {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = BeginTransactionAsync(level, ct);
        Runtime.BlockOn(task);
        return TaskRuntime.GetResult <DbTransaction >(task);
    }
    /// <summary>Disposes the connection.</summary>
    public void dispose(ref this) {
        Dispose();
    }
    /// <summary>Releases connection resources.</summary>
    protected virtual void Dispose() {
        if (_disposed)
        {
            return;
        }
        let task = CloseAsync();
        Runtime.BlockOn(task);
        _disposed = true;
    }
    /// <summary>Throws if the connection has been disposed.</summary>
    protected void ThrowIfDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("DbConnection disposed");
        }
    }
}
private sealed class DbConnectionTestAdapter : DbConnection
{
    public override ConnectionState State => ConnectionState.Closed;
    public override string Database => "dummy";
    public override string DataSource => "dummy";
    public override string ServerVersion => "1.0";
}
testcase Given_db_connection_default_connection_string_empty_When_executed_Then_db_connection_default_connection_string_empty()
{
    var connection = new DbConnectionTestAdapter();
    Assert.That(connection.ConnectionString.Length).IsEqualTo(0);
}
testcase Given_db_connection_create_command_throws_When_executed_Then_db_connection_create_command_throws()
{
    var connection = new DbConnectionTestAdapter();
    Assert.Throws <DbException >(() => {
        let _ = connection.CreateCommand();
    }
    );
}
testcase Given_db_connection_open_async_throws_When_executed_Then_db_connection_open_async_throws()
{
    var connection = new DbConnectionTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = connection.OpenAsync(ct);
    }
    );
}
testcase Given_db_connection_begin_transaction_async_throws_When_executed_Then_db_connection_begin_transaction_async_throws()
{
    var connection = new DbConnectionTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = connection.BeginTransactionAsync(IsolationLevel.ReadCommitted, ct);
    }
    );
}
