namespace Exec.StdData;

import Std.Async;
import Std.Data;

public class FakeTransaction : DbTransaction
{
    private FakeConnection _connection;
    private IsolationLevel _level;
    private bool _committed;
    private bool _rolledBack;

    public init(FakeConnection connection, IsolationLevel level)
    {
        _connection = connection;
        _level = level;
        _committed = false;
        _rolledBack = false;
    }

    public bool Committed => _committed;
    public bool RolledBack => _rolledBack;

    public override DbConnection Connection => _connection;

    public override IsolationLevel IsolationLevel => _level;

    public override Task CommitAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Commit canceled");
        }
        _committed = true;
        _connection.CompleteTransaction();
        return Std.Async.TaskRuntime.CompletedTask();
    }

    public override Task RollbackAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Rollback canceled");
        }
        _rolledBack = true;
        _connection.CompleteTransaction();
        return Std.Async.TaskRuntime.CompletedTask();
    }
}
