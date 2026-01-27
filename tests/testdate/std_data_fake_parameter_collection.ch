namespace Exec.StdData;

import Foundation.Collections;
import Std.Collections;
import Std.Data;
import Std.Numeric;

public class FakeParameterCollection : DbParameterCollection
{
    private VecPtr _items;

    public init()
    {
        _items = Vec.New<DbParameter>();
    }

    public override int Count
    {
        get { return NumericUnchecked.ToInt32(Vec.Len(in _items)); }
    }

    public override void Add(DbParameter parameter)
    {
        if (parameter == null)
        {
            throw new Std.ArgumentNullException("parameter");
        }
        let status = Vec.Push<DbParameter>(ref _items, parameter);
        if (status != VecError.Success)
        {
            throw new Std.InvalidOperationException("Failed to add parameter");
        }
    }

    public override bool Remove(DbParameter parameter)
    {
        if (parameter == null)
        {
            return false;
        }
        let span = Vec.AsSpan<DbParameter>(ref _items);
        let length = span.Length;
        var idx = 0usize;
        while (idx < length)
        {
            if (span[idx] == parameter)
            {
                var next = idx + 1usize;
                while (next < length)
                {
                    span[idx] = span[next];
                    idx += 1usize;
                    next += 1usize;
                }
                let _ = VecIntrinsics.chic_rt_vec_truncate(ref _items, length - 1usize);
                return true;
            }
            idx += 1usize;
        }
        return false;
    }

    public override void Clear()
    {
        let _ = VecIntrinsics.chic_rt_vec_clear(ref _items);
    }

    public override DbParameter this[int index]
    {
        get
        {
            let span = Vec.AsReadOnlySpan<DbParameter>(in _items);
            if (index < 0 || (usize)index >= span.Length)
            {
                throw new Std.ArgumentOutOfRangeException("index");
            }
            return span[(usize)index];
        }
    }

    public override DbParameter this[string name]
    {
        get
        {
            let span = Vec.AsReadOnlySpan<DbParameter>(in _items);
            var idx = 0usize;
            while (idx < span.Length)
            {
                let parameter = span[idx];
                if (parameter.ParameterName == name)
                {
                    return parameter;
                }
                idx += 1usize;
            }
            throw new Std.ArgumentException("Unknown parameter: " + name);
        }
    }

    public void dispose(ref this)
    {
        VecIntrinsics.chic_rt_vec_drop(ref _items);
    }
}
