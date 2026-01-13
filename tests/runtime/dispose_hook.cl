namespace Exec;

import Std;
import Std.Collections;
import Std.Numeric;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;

public static class DisposeLog
{
    private static int[] Values;
    private static int Index;

    public static void Reset(int count)
    {
        Values = new int[count];
        Index = 0;
    }

    public static void Push(int value)
    {
        if (Values.Length == 0)
        {
            return;
        }
        if (Index < (int)Values.Length)
        {
            Values[Index] = value;
            Index += 1;
        }
    }

    public static int Count() => Index;

    public static int At(int index)
    {
        if (index < 0 || index >= Index)
        {
            return -1;
        }
        return Values[index];
    }
}

public struct Droppy
{
    private int _id;

    public init(int id)
    {
        _id = id;
    }

    public void dispose(ref this)
    {
        DisposeLog.Push(_id);
    }
}

private static void EarlyReturn()
{
    var a = new Droppy(1);
    return;
}

private static void ThrowsAfterAlloc()
{
    var a = new Droppy(1);
    var b = new Droppy(2);
    throw new Std.InvalidOperationException("boom");
}

testcase Dispose_CallsOnScopeExit_InReverseOrder()
{
    DisposeLog.Reset(2);
    {
        var a = new Droppy(1);
        var b = new Droppy(2);
    }
    return DisposeLog.Count() == 2
        && DisposeLog.At(0) == 2
        && DisposeLog.At(1) == 1;
}

testcase Dispose_CallsOnEarlyReturn()
{
    DisposeLog.Reset(1);
    EarlyReturn();
    return DisposeLog.Count() == 1 && DisposeLog.At(0) == 1;
}

testcase Dispose_CallsOnThrow()
{
    DisposeLog.Reset(2);
    try
    {
        ThrowsAfterAlloc();
        return false;
    }
    catch (Std.InvalidOperationException)
    {
    }
    return DisposeLog.Count() == 2
        && DisposeLog.At(0) == 2
        && DisposeLog.At(1) == 1;
}

testcase Dispose_CallsForArrayElements()
{
    DisposeLog.Reset(2);
    {
        var values = new Droppy[] { new Droppy(1), new Droppy(2) };
    }
    return DisposeLog.Count() == 2
        && DisposeLog.At(0) == 2
        && DisposeLog.At(1) == 1;
}

testcase Dispose_CallsForVecElements()
{
    DisposeLog.Reset(3);
    var vec = VecIntrinsics.Create<Droppy>();
    let ok0 = VecUtil.Push(ref vec, new Droppy(1));
    let ok1 = VecUtil.Push(ref vec, new Droppy(2));
    let ok2 = VecUtil.Push(ref vec, new Droppy(3));
    if (ok0 != VecError.Success || ok1 != VecError.Success || ok2 != VecError.Success)
    {
        FVecIntrinsics.chic_rt_vec_drop(ref vec);
        return false;
    }
    FVecIntrinsics.chic_rt_vec_drop(ref vec);
    return DisposeLog.Count() == 3
        && DisposeLog.At(0) == 3
        && DisposeLog.At(1) == 2
        && DisposeLog.At(2) == 1;
}

testcase Using_InvokesDisposeDeterministically()
{
    DisposeLog.Reset(1);
    using (var value = new Droppy(7))
    {
    }
    return DisposeLog.Count() == 1 && DisposeLog.At(0) == 7;
}

