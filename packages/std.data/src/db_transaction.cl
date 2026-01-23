namespace Std.Data;
import Std;
import Std.Async;
import Std.Core;
import Std.Testing;
/// <summary>Represents an active database transaction.</summary>
public abstract class DbTransaction
{
    private bool _disposed;
    /// <summary>Gets the connection associated with the transaction.</summary>
    public abstract DbConnection Connection {
        get;
    }
    /// <summary>Gets the isolation level used by the transaction.</summary>
    public abstract IsolationLevel IsolationLevel {
        get;
    }
    /// <summary>Commits the transaction asynchronously.</summary>
    public virtual Task CommitAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Commit canceled");
        }
        throw new DbException("DbTransaction.CommitAsync not implemented");
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Rolls back the transaction asynchronously.</summary>
    public virtual Task RollbackAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Rollback canceled");
        }
        throw new DbException("DbTransaction.RollbackAsync not implemented");
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Commits the transaction, blocking until complete.</summary>
    public void Commit() {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = CommitAsync(ct);
        Runtime.BlockOn(task);
    }
    /// <summary>Rolls back the transaction, blocking until complete.</summary>
    public void Rollback() {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = RollbackAsync(ct);
        Runtime.BlockOn(task);
    }
    /// <summary>Disposes the transaction.</summary>
    public void dispose(ref this) {
        Dispose();
    }
    /// <summary>Releases transaction resources.</summary>
    protected virtual void Dispose() {
        _disposed = true;
    }
    /// <summary>Throws if the transaction has been disposed.</summary>
    protected void ThrowIfDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("DbTransaction disposed");
        }
    }
}
private sealed class DbTransactionTestConnection : DbConnection
{
    public override ConnectionState State => ConnectionState.Closed;
    public override string Database => "dummy";
    public override string DataSource => "dummy";
    public override string ServerVersion => "1.0";
}
private sealed class DbTransactionTestAdapter : DbTransaction
{
    private DbConnection _connection;
    public init() {
        _connection = new DbTransactionTestConnection();
    }
    public override DbConnection Connection => _connection;
    public override IsolationLevel IsolationLevel => IsolationLevel.ReadCommitted;
}
testcase Given_db_transaction_commit_async_throws_When_executed_Then_db_transaction_commit_async_throws()
{
    var transaction = new DbTransactionTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = transaction.CommitAsync(ct);
    }
    );
}
testcase Given_db_transaction_rollback_async_throws_When_executed_Then_db_transaction_rollback_async_throws()
{
    var transaction = new DbTransactionTestAdapter();
    let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
    Assert.Throws <DbException >(() => {
        let _ = transaction.RollbackAsync(ct);
    }
    );
}
