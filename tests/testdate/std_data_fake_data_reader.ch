namespace Exec.StdData;

import Std.Async;
import Std.Data;

public class FakeDataReader : DbDataReader
{
    private string[] _columns;
    private object?[][] _rows;
    private int _currentRow;

    public init(FakeResultSet resultSet)
    {
        _columns = resultSet.Columns;
        _rows = resultSet.Rows;
        _currentRow = -1;
    }

    public override int FieldCount => _columns.Length;

    public int CurrentRowIndex => _currentRow;

    public override Task<bool> ReadAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Read canceled");
        }
        _currentRow += 1;
        if (_currentRow >= _rows.Length)
        {
            return Std.Async.TaskRuntime.FromResult(false);
        }
        return Std.Async.TaskRuntime.FromResult(true);
    }

    public override Task<bool> NextResultAsync(CancellationToken ct)
    {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested())
        {
            throw new Std.TaskCanceledException("Next result canceled");
        }
        return Std.Async.TaskRuntime.FromResult(false);
    }

    public override bool IsDBNull(int ordinal)
    {
        return this[ordinal] == null;
    }

    public override string GetName(int ordinal)
    {
        ValidateOrdinal(ordinal);
        return _columns[ordinal];
    }

    public override int GetOrdinal(string name)
    {
        var idx = 0;
        while (idx < _columns.Length)
        {
            if (_columns[idx] == name)
            {
                return idx;
            }
            idx += 1;
        }
        throw new Std.ArgumentException("Column not found: " + name);
    }

    public override int GetInt32(int ordinal)
    {
        ValidateOrdinal(ordinal);
        EnsureCurrentRow();
        return (int)_rows[_currentRow][ordinal];
    }

    public override long GetInt64(int ordinal)
    {
        ValidateOrdinal(ordinal);
        EnsureCurrentRow();
        return (long)_rows[_currentRow][ordinal];
    }

    public override string GetString(int ordinal)
    {
        ValidateOrdinal(ordinal);
        EnsureCurrentRow();
        return (string)_rows[_currentRow][ordinal];
    }

    public override bool GetBoolean(int ordinal)
    {
        ValidateOrdinal(ordinal);
        EnsureCurrentRow();
        return (bool)_rows[_currentRow][ordinal];
    }

    public override double GetDouble(int ordinal)
    {
        ValidateOrdinal(ordinal);
        EnsureCurrentRow();
        return (double)_rows[_currentRow][ordinal];
    }

    public override decimal GetDecimal(int ordinal)
    {
        ValidateOrdinal(ordinal);
        EnsureCurrentRow();
        return (decimal)_rows[_currentRow][ordinal];
    }

    public override object? this[int ordinal]
    {
        get
        {
            ValidateOrdinal(ordinal);
            EnsureCurrentRow();
            return _rows[_currentRow][ordinal];
        }
    }

    public override object? this[string name]
    {
        get
        {
            let ordinal = GetOrdinal(name);
            return this[ordinal];
        }
    }

    protected override void Dispose()
    {
        base.Dispose();
    }

    private void ValidateOrdinal(int ordinal)
    {
        ThrowIfDisposed();
        if (ordinal < 0 || ordinal >= _columns.Length)
        {
            throw new Std.ArgumentOutOfRangeException("ordinal");
        }
    }

    private void EnsureCurrentRow()
    {
        if (_currentRow < 0 || _currentRow >= _rows.Length)
        {
            throw new Std.InvalidOperationException("No current row");
        }
    }
}
