namespace Std.Data;
import Std;
import Std.Async;
import Std.Core;
import Std.Testing;
/// <summary>Streams result rows from a database command.</summary>
public abstract class DbDataReader
{
    private bool _disposed;
    /// <summary>Gets the number of columns in the current row.</summary>
    public abstract int FieldCount {
        get;
    }
    /// <summary>Advances to the next row asynchronously.</summary>
    public virtual Task <bool >ReadAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Read canceled");
        }
        throw new DbException("DbDataReader.ReadAsync not implemented");
        return TaskRuntime.FromResult <bool >(false);
    }
    /// <summary>Advances to the next result set asynchronously.</summary>
    public virtual Task <bool >NextResultAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Read canceled");
        }
        return TaskRuntime.FromResult(false);
    }
    /// <summary>Advances to the next row, blocking until complete.</summary>
    public bool Read() {
        ThrowIfDisposed();
        let ct = CoreIntrinsics.DefaultValue <CancellationToken >();
        let task = ReadAsync(ct);
        Runtime.BlockOn(task);
        return TaskRuntime.GetResult <bool >(task);
    }
    /// <summary>Returns whether the value at the given ordinal is null.</summary>
    public virtual bool IsDBNull(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.IsDBNull not implemented");
        return false;
    }
    /// <summary>Gets the column name for the given ordinal.</summary>
    public virtual string GetName(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetName not implemented");
        return CoreIntrinsics.DefaultValue <string >();
    }
    /// <summary>Finds the ordinal for the given column name.</summary>
    public virtual int GetOrdinal(string name) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetOrdinal not implemented");
        return 0;
    }
    /// <summary>Gets a 32-bit integer value from the given ordinal.</summary>
    public virtual int GetInt32(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetInt32 not implemented");
        return 0;
    }
    /// <summary>Gets a 64-bit integer value from the given ordinal.</summary>
    public virtual long GetInt64(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetInt64 not implemented");
        return 0L;
    }
    /// <summary>Gets a string value from the given ordinal.</summary>
    public virtual string GetString(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetString not implemented");
        return CoreIntrinsics.DefaultValue <string >();
    }
    /// <summary>Gets a boolean value from the given ordinal.</summary>
    public virtual bool GetBoolean(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetBoolean not implemented");
        return false;
    }
    /// <summary>Gets a double value from the given ordinal.</summary>
    public virtual double GetDouble(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetDouble not implemented");
        return 0.0d;
    }
    /// <summary>Gets a decimal value from the given ordinal.</summary>
    public virtual decimal GetDecimal(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetDecimal not implemented");
        return 0m;
    }
    /// <summary>Gets the boxed value at the given ordinal.</summary>
    public virtual object ?GetValue(int ordinal) {
        ThrowIfDisposed();
        throw new DbException("DbDataReader.GetValue not implemented");
        return CoreIntrinsics.DefaultValue <object ?>();
    }
    /// <summary>Disposes the reader.</summary>
    public void Close() {
        Dispose();
    }
    /// <summary>Disposes the reader.</summary>
    public void dispose(ref this) {
        Dispose();
    }
    /// <summary>Releases reader resources.</summary>
    protected virtual void Dispose() {
        _disposed = true;
    }
    /// <summary>Throws if the reader has been disposed.</summary>
    protected void ThrowIfDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("DbDataReader disposed");
        }
    }
}

private sealed class DbDataReaderTestAdapter : DbDataReader
{
    public override int FieldCount => 0;
}

testcase Given_db_data_reader_next_result_async_returns_false_When_executed_Then_db_data_reader_next_result_async_returns_false()
{
    var reader = new DbDataReaderTestAdapter();
    let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
    let task = reader.NextResultAsync(ct);
    Assert.That(TaskRuntime.GetResult<bool>(task)).IsFalse();
}

testcase Given_db_data_reader_read_async_throws_When_executed_Then_db_data_reader_read_async_throws()
{
    var reader = new DbDataReaderTestAdapter();
    let ct = CoreIntrinsics.DefaultValue<CancellationToken>();
    Assert.Throws<DbException>(() => {
        let _ = reader.ReadAsync(ct);
    });
}

testcase Given_db_data_reader_is_dbnull_throws_When_executed_Then_db_data_reader_is_dbnull_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.IsDBNull(0);
    });
}

testcase Given_db_data_reader_get_name_throws_When_executed_Then_db_data_reader_get_name_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetName(0);
    });
}

testcase Given_db_data_reader_get_ordinal_throws_When_executed_Then_db_data_reader_get_ordinal_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetOrdinal("col");
    });
}

testcase Given_db_data_reader_get_int32_throws_When_executed_Then_db_data_reader_get_int32_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetInt32(0);
    });
}

testcase Given_db_data_reader_get_int64_throws_When_executed_Then_db_data_reader_get_int64_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetInt64(0);
    });
}

testcase Given_db_data_reader_get_string_throws_When_executed_Then_db_data_reader_get_string_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetString(0);
    });
}

testcase Given_db_data_reader_get_boolean_throws_When_executed_Then_db_data_reader_get_boolean_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetBoolean(0);
    });
}

testcase Given_db_data_reader_get_double_throws_When_executed_Then_db_data_reader_get_double_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetDouble(0);
    });
}

testcase Given_db_data_reader_get_decimal_throws_When_executed_Then_db_data_reader_get_decimal_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetDecimal(0);
    });
}

testcase Given_db_data_reader_get_value_throws_When_executed_Then_db_data_reader_get_value_throws()
{
    var reader = new DbDataReaderTestAdapter();
    Assert.Throws<DbException>(() => {
        let _ = reader.GetValue(0);
    });
}
