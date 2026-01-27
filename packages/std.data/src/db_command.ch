namespace Std.Data;
import Std;
import Std.Async;
import Std.Core;
import Std.Runtime;
import Std.Testing;
/// <summary>Represents a database command with async-first execution semantics.</summary>
public abstract class DbCommand
{
    private bool _disposed;
    private string _commandText;
    private CommandType _commandType;
    private int _commandTimeoutSeconds;
    private DbConnection _connection;
    private DbTransaction ?_transaction;
    protected init() {
        _commandText = StringRuntime.Create();
        _commandType = CommandType.Text;
        _commandTimeoutSeconds = 30;
    }
    /// <summary>Gets or sets the command text to execute.</summary>
    public string CommandText {
        get {
            ThrowIfDisposed();
            return _commandText;
        }
        set {
            ThrowIfDisposed();
            _commandText = value;
        }
    }
    /// <summary>Gets or sets how the command text should be interpreted.</summary>
    public CommandType CommandType {
        get {
            ThrowIfDisposed();
            return _commandType;
        }
        set {
            ThrowIfDisposed();
            _commandType = value;
        }
    }
    /// <summary>Gets or sets the command timeout in seconds.</summary>
    public int CommandTimeoutSeconds {
        get {
            ThrowIfDisposed();
            return _commandTimeoutSeconds;
        }
        set {
            ThrowIfDisposed();
            _commandTimeoutSeconds = value;
        }
    }
    /// <summary>Gets or sets the connection associated with this command.</summary>
    public DbConnection Connection {
        get {
            ThrowIfDisposed();
            return _connection;
        }
        set {
            ThrowIfDisposed();
            _connection = value;
        }
    }
    /// <summary>Gets or sets the transaction associated with this command.</summary>
    public DbTransaction ?Transaction {
        get {
            ThrowIfDisposed();
            return _transaction;
        }
        set {
            ThrowIfDisposed();
            _transaction = value;
        }
    }
    /// <summary>Gets the parameter collection attached to this command.</summary>
    public abstract DbParameterCollection Parameters {
        get;
    }
    /// <summary>Creates a new parameter instance compatible with the provider.</summary>
    public virtual DbParameter CreateParameter() {
        ThrowIfDisposed();
        throw new DbException("DbCommand.CreateParameter not implemented");
        return CoreIntrinsics.DefaultValue <DbParameter >();
    }
    /// <summary>Executes the command asynchronously, returning the number of affected rows.</summary>
    public virtual Task <int >ExecuteNonQueryAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("ExecuteNonQuery canceled");
        }
        throw new DbException("DbCommand.ExecuteNonQueryAsync not implemented");
        return TaskRuntime.FromResult <int >(0);
    }
    /// <summary>Executes the command asynchronously, returning the first column of the first row.</summary>
    public virtual Task <object ?>ExecuteScalarAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("ExecuteScalar canceled");
        }
        throw new DbException("DbCommand.ExecuteScalarAsync not implemented");
        return TaskRuntime.FromResult <object ?>(CoreIntrinsics.DefaultValue <object ?>());
    }
    /// <summary>Executes the command asynchronously, returning a streaming reader.</summary>
    public virtual Task <DbDataReader >ExecuteReaderAsync(CommandBehavior behavior, CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("ExecuteReader canceled");
        }
        throw new DbException("DbCommand.ExecuteReaderAsync not implemented");
        return TaskRuntime.FromResult <DbDataReader >(CoreIntrinsics.DefaultValue <DbDataReader >());
    }
    /// <summary>Prepares the command asynchronously.</summary>
    public virtual Task PrepareAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Prepare canceled");
        }
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Executes the command, blocking until complete.</summary>
    public int ExecuteNonQuery() {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = ExecuteNonQueryAsync(ct);
        Runtime.BlockOn(task);
        return TaskRuntime.GetResult <int >(task);
    }
    /// <summary>Executes the command, returning the first column of the first row.</summary>
    public object ?ExecuteScalar() {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = ExecuteScalarAsync(ct);
        Runtime.BlockOn(task);
        return TaskRuntime.GetResult <object ?>(task);
    }
    /// <summary>Executes the command and returns a streaming reader.</summary>
    public DbDataReader ExecuteReader() {
        return ExecuteReader(CommandBehavior.Default);
    }
    /// <summary>Executes the command with the specified behavior, blocking until complete.</summary>
    public DbDataReader ExecuteReader(CommandBehavior behavior) {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = ExecuteReaderAsync(behavior, ct);
        Runtime.BlockOn(task);
        return TaskRuntime.GetResult <DbDataReader >(task);
    }
    /// <summary>Disposes the command.</summary>
    public void dispose(ref this) {
        Dispose();
    }
    /// <summary>Releases command resources.</summary>
    protected virtual void Dispose() {
        _disposed = true;
    }
    /// <summary>Throws if the command has been disposed.</summary>
    protected void ThrowIfDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("DbCommand disposed");
        }
    }
}
private sealed class DbCommandTestAdapter : DbCommand
{
    public override DbParameterCollection Parameters => CoreIntrinsics.DefaultValue <DbParameterCollection >();
}
testcase Given_db_command_default_command_type_text_When_executed_Then_db_command_default_command_type_text()
{
    var command = new DbCommandTestAdapter();
    Assert.That(command.CommandType).IsEqualTo(CommandType.Text);
}
testcase Given_db_command_default_timeout_seconds_When_executed_Then_db_command_default_timeout_seconds()
{
    var command = new DbCommandTestAdapter();
    Assert.That(command.CommandTimeoutSeconds).IsEqualTo(30);
}
testcase Given_db_command_default_command_text_empty_When_executed_Then_db_command_default_command_text_empty()
{
    var command = new DbCommandTestAdapter();
    Assert.That(command.CommandText.Length).IsEqualTo(0);
}
testcase Given_db_command_create_parameter_throws_When_executed_Then_db_command_create_parameter_throws()
{
    var command = new DbCommandTestAdapter();
    Assert.Throws <DbException >(() => {
        let _ = command.CreateParameter();
    }
    );
}
testcase Given_db_command_execute_non_query_async_throws_When_executed_Then_db_command_execute_non_query_async_throws()
{
    var command = new DbCommandTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = command.ExecuteNonQueryAsync(ct);
    }
    );
}
testcase Given_db_command_execute_scalar_async_throws_When_executed_Then_db_command_execute_scalar_async_throws()
{
    var command = new DbCommandTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = command.ExecuteScalarAsync(ct);
    }
    );
}
testcase Given_db_command_execute_reader_async_throws_When_executed_Then_db_command_execute_reader_async_throws()
{
    var command = new DbCommandTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = command.ExecuteReaderAsync(CommandBehavior.Default, ct);
    }
    );
}
testcase Given_db_command_disposed_throws_on_access_When_executed_Then_db_command_disposed_throws_on_access()
{
    var command = new DbCommandTestAdapter();
    command.dispose();
    Assert.Throws <ObjectDisposedException >(() => {
        let _ = command.CommandType;
    }
    );
}
