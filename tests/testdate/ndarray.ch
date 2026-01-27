import Std;
import Std.NdArray;
import Std.Linalg;
import Std.Numeric;
import Std.Span;
import Std.Memory;
import Std.Runtime.Collections;

namespace Exec;

public class NdArrayTests
{
    private static void SetUsize(ref Span<usize> span, usize index, usize value)
    {
        var slot = MaybeUninit<usize>.Init(value);
        unsafe
        {
            let size = span.Raw.Data.Size;
            let align = span.Raw.Data.Alignment;
            let ptr = SpanIntrinsics.chic_rt_span_ptr_at_mut(ref span.Raw, index);
            let dest = ValuePointer.CreateMut(ptr, size, align);
            GlobalAllocator.Copy(dest, slot.AsValueConstPtr(), size);
            slot.ForgetInit();
        }
    }

    private static void SetDouble(ref Span<double> span, usize index, double value)
    {
        var slot = MaybeUninit<double>.Init(value);
        unsafe
        {
            let size = span.Raw.Data.Size;
            let align = span.Raw.Data.Alignment;
            let ptr = SpanIntrinsics.chic_rt_span_ptr_at_mut(ref span.Raw, index);
            let dest = ValuePointer.CreateMut(ptr, size, align);
            GlobalAllocator.Copy(dest, slot.AsValueConstPtr(), size);
            slot.ForgetInit();
        }
    }

    private static void SetSlice(ref Span<NdSlice> span, usize index, usize start, usize length)
    {
        var slot = MaybeUninit<NdSlice>.Init(new NdSlice(start, length));
        unsafe
        {
            let size = span.Raw.Data.Size;
            let align = span.Raw.Data.Alignment;
            let ptr = SpanIntrinsics.chic_rt_span_ptr_at_mut(ref span.Raw, index);
            let dest = ValuePointer.CreateMut(ptr, size, align);
            GlobalAllocator.Copy(dest, slot.AsValueConstPtr(), size);
            slot.ForgetInit();
        }
    }

    private static bool EqualsApprox(double left, double right)
    {
        let delta = left - right;
        if (delta < 0.0) { return -delta < 1e-9; }
        return delta < 1e-9;
    }

    private static bool TestBasics()
    {
        var shape = Span<usize>.StackAlloc(2);
        SetUsize(ref shape, 0usize, 2usize);
        SetUsize(ref shape, 1usize, 3usize);

        var data = Span<double>.StackAlloc(6);
        SetDouble(ref data, 0usize, 1.0);
        SetDouble(ref data, 1usize, 2.0);
        SetDouble(ref data, 2usize, 3.0);
        SetDouble(ref data, 3usize, 4.0);
        SetDouble(ref data, 4usize, 5.0);
        SetDouble(ref data, 5usize, 6.0);

        let array = NdArray<double>.FromSlice(data.AsReadOnly(), shape.AsReadOnly());
        if (array.Rank != 2 || array.Length != 6usize)
        {
            return false;
        }

        let arrayView = array.AsView();
        if (!EqualsApprox(arrayView.Get2(0usize, 0usize), 1.0)) { return false; }
        if (!EqualsApprox(arrayView.Get2(1usize, 2usize), 6.0)) { return false; }

        var slices = Span<NdSlice>.StackAlloc(2);
        SetSlice(ref slices, 0usize, 0usize, 1usize);
        SetSlice(ref slices, 1usize, 0usize, 3usize);
        let firstRow = arrayView.Slice(slices.AsReadOnly());
        if (firstRow.Length != 3usize || !EqualsApprox(firstRow.Get2(0usize, 1usize), 2.0))
        {
            return false;
        }

        var mutable = array.AsViewMut();
        mutable.Set2(0usize, 1usize, 20.0);
        if (!EqualsApprox(array.AsView().Get2(0usize, 1usize), 20.0))
        {
            return false;
        }
        return true;
    }

    private static bool TestBroadcast()
    {
        var shape = Span<usize>.StackAlloc(2);
        SetUsize(ref shape, 0usize, 2usize);
        SetUsize(ref shape, 1usize, 3usize);

        var data = Span<double>.StackAlloc(6);
        SetDouble(ref data, 0usize, 1.0);
        SetDouble(ref data, 1usize, 2.0);
        SetDouble(ref data, 2usize, 3.0);
        SetDouble(ref data, 3usize, 4.0);
        SetDouble(ref data, 4usize, 5.0);
        SetDouble(ref data, 5usize, 6.0);

        let array = NdArray<double>.FromSlice(data.AsReadOnly(), shape.AsReadOnly());
        let shifted = array.Add(2.0);
        if (!EqualsApprox(shifted.AsView().Get2(0usize, 0usize), 3.0)) { return false; }
        if (!EqualsApprox(shifted.AsView().Get2(1usize, 2usize), 8.0)) { return false; }

        var biasShape = Span<usize>.StackAlloc(2);
        SetUsize(ref biasShape, 0usize, 1usize);
        SetUsize(ref biasShape, 1usize, 3usize);
        var biasData = Span<double>.StackAlloc(3);
        SetDouble(ref biasData, 0usize, 1.0);
        SetDouble(ref biasData, 1usize, 1.0);
        SetDouble(ref biasData, 2usize, 1.0);

        let bias = NdArray<double>.FromSlice(biasData.AsReadOnly(), biasShape.AsReadOnly());
        let combined = array.AsView().Add(bias.AsView());
        if (!EqualsApprox(combined.AsView().Get2(0usize, 0usize), 2.0)) { return false; }
        if (!EqualsApprox(combined.AsView().Get2(1usize, 2usize), 7.0)) { return false; }
        return true;
    }

    private static bool TestReshapeAndPermute()
    {
        var shape = Span<usize>.StackAlloc(2);
        SetUsize(ref shape, 0usize, 2usize);
        SetUsize(ref shape, 1usize, 3usize);

        var data = Span<double>.StackAlloc(6);
        SetDouble(ref data, 0usize, 1.0);
        SetDouble(ref data, 1usize, 2.0);
        SetDouble(ref data, 2usize, 3.0);
        SetDouble(ref data, 3usize, 4.0);
        SetDouble(ref data, 4usize, 5.0);
        SetDouble(ref data, 5usize, 6.0);

        let array = NdArray<double>.FromSlice(data.AsReadOnly(), shape.AsReadOnly());
        var reshapeDims = Span<usize>.StackAlloc(2);
        SetUsize(ref reshapeDims, 0usize, 3usize);
        SetUsize(ref reshapeDims, 1usize, 2usize);
        let reshaped = array.AsView().Reshape(reshapeDims.AsReadOnly());
        if (!EqualsApprox(reshaped.Get2(2usize, 1usize), 6.0)) { return false; }

        let transposed = array.AsView().Transpose2D();
        if (transposed.GetShape().Rank != 2 || transposed.GetShape().Length != 6usize) { return false; }
        if (!EqualsApprox(transposed.Get2(0usize, 1usize), 4.0)) { return false; }
        if (!EqualsApprox(transposed.Get2(2usize, 0usize), 3.0)) { return false; }
        return true;
    }

    private static bool TestLinalg()
    {
        var vecShape = Span<usize>.StackAlloc(1);
        SetUsize(ref vecShape, 0usize, 3usize);
        var leftData = Span<double>.StackAlloc(3);
        SetDouble(ref leftData, 0usize, 1.0);
        SetDouble(ref leftData, 1usize, 2.0);
        SetDouble(ref leftData, 2usize, 3.0);
        var rightData = Span<double>.StackAlloc(3);
        SetDouble(ref rightData, 0usize, 4.0);
        SetDouble(ref rightData, 1usize, 5.0);
        SetDouble(ref rightData, 2usize, 6.0);

        let leftVec = NdArray<double>.FromSlice(leftData.AsReadOnly(), vecShape.AsReadOnly());
        let rightVec = NdArray<double>.FromSlice(rightData.AsReadOnly(), vecShape.AsReadOnly());
        let dot = Linalg.Dot(leftVec.AsView(), rightVec.AsView());
        if (!EqualsApprox(dot, 32.0)) { return false; }

        var matShape = Span<usize>.StackAlloc(2);
        SetUsize(ref matShape, 0usize, 2usize);
        SetUsize(ref matShape, 1usize, 2usize);
        var aData = Span<double>.StackAlloc(4);
        SetDouble(ref aData, 0usize, 1.0);
        SetDouble(ref aData, 1usize, 2.0);
        SetDouble(ref aData, 2usize, 3.0);
        SetDouble(ref aData, 3usize, 4.0);
        var bData = Span<double>.StackAlloc(4);
        SetDouble(ref bData, 0usize, 5.0);
        SetDouble(ref bData, 1usize, 6.0);
        SetDouble(ref bData, 2usize, 7.0);
        SetDouble(ref bData, 3usize, 8.0);

        let a = NdArray<double>.FromSlice(aData.AsReadOnly(), matShape.AsReadOnly());
        let b = NdArray<double>.FromSlice(bData.AsReadOnly(), matShape.AsReadOnly());
        let product = Linalg.MatMul(a.AsView(), b.AsView());
        let resultView = product.AsView();
        if (!EqualsApprox(resultView.Get2(0usize, 0usize), 19.0)) { return false; }
        if (!EqualsApprox(resultView.Get2(0usize, 1usize), 22.0)) { return false; }
        if (!EqualsApprox(resultView.Get2(1usize, 0usize), 43.0)) { return false; }
        if (!EqualsApprox(resultView.Get2(1usize, 1usize), 50.0)) { return false; }
        return true;
    }

    public static bool RunAll()
    {
        if (!TestBasics()) { return false; }
        if (!TestBroadcast()) { return false; }
        if (!TestReshapeAndPermute()) { return false; }
        if (!TestLinalg()) { return false; }
        return true;
    }
}

public int Main()
{
    return NdArrayTests.RunAll() ? 0 : 1;
}
