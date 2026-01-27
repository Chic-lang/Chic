namespace Foundation.Collections;
import Std.Runtime.Collections;
import Std.Memory;
import Std.Span;
import Std.Core;
import Std.Core.Testing;

public enum VecError
{
    Success = 0, AllocationFailed = 1, InvalidPointer = 2, CapacityOverflow = 3, OutOfBounds = 4, LengthOverflow = 5, IterationComplete = 6,
}

public static class VecIntrinsics
{
    @extern("C") public static extern VecPtr chic_rt_vec_new(usize elementSize, usize elementAlignment, isize dropFn);
    @extern("C") public static extern VecPtr chic_rt_vec_with_capacity(usize elementSize, usize elementAlignment, usize capacity,
    isize dropFn);
    @extern("C") public static extern VecError chic_rt_vec_reserve(ref VecPtr vec, usize additional);
    @extern("C") public static extern VecError chic_rt_vec_shrink_to_fit(ref VecPtr vec);
    @extern("C") public static extern VecError chic_rt_vec_push(ref VecPtr vec, in ValueConstPtr value);
    @extern("C") public static extern VecError chic_rt_vec_pop(ref VecPtr vec, in ValueMutPtr value);
    @extern("C") public static extern VecError chic_rt_vec_view(in VecPtr vec, out VecViewPtr destination);
    @extern("C") public static extern VecIterPtr chic_rt_vec_iter(in VecPtr vec);
    @extern("C") public static extern VecError chic_rt_vec_iter_next(ref VecIterPtr iter, in ValueMutPtr destination);
    @extern("C") public static extern ValueConstPtr chic_rt_vec_iter_next_ptr(ref VecIterPtr iter);
    @extern("C") public static extern ValueConstPtr chic_rt_vec_data(in VecPtr vec);
    @extern("C") public static extern ValueMutPtr chic_rt_vec_data_mut(ref VecPtr vec);
    @extern("C") public static extern VecError chic_rt_vec_insert(ref VecPtr vec, usize index, in ValueConstPtr value);
    @extern("C") public static extern VecError chic_rt_vec_remove(ref VecPtr vec, usize index, in ValueMutPtr destination);
    @extern("C") public static extern VecError chic_rt_vec_swap_remove(ref VecPtr vec, usize index, in ValueMutPtr destination);
    @extern("C") public static extern VecError chic_rt_vec_truncate(ref VecPtr vec, usize newLength);
    @extern("C") public static extern VecError chic_rt_vec_clear(ref VecPtr vec);
    @extern("C") public static extern void chic_rt_vec_drop(ref VecPtr vec);
    @extern("C") public static extern VecError chic_rt_vec_set_len(ref VecPtr vec, usize newLength);
    @extern("C") public static extern usize chic_rt_vec_len(in VecPtr vec);
    @extern("C") public static extern usize chic_rt_vec_capacity(in VecPtr vec);
    @extern("C") public static extern VecError chic_rt_vec_clone(ref VecPtr dest, in VecPtr src);
}
public static class Vec
{
    private static isize DropGlueOf <T >() {
        return(isize) __drop_glue_of <T >();
    }
    public static VecPtr New <T >() {
        return VecIntrinsics.chic_rt_vec_new((usize) __sizeof <T >(), (usize) __alignof <T >(), DropGlueOf <T >());
    }
    public static VecPtr WithCapacity <T >(usize capacity) {
        return VecIntrinsics.chic_rt_vec_with_capacity((usize) __sizeof <T >(), (usize) __alignof <T >(), capacity,
        DropGlueOf <T >());
    }
    public static VecError Reserve <T >(ref VecPtr vec, usize additional) {
        return VecIntrinsics.chic_rt_vec_reserve(ref vec, additional);
    }
    public static VecError ShrinkToFit <T >(ref VecPtr vec) {
        return VecIntrinsics.chic_rt_vec_shrink_to_fit(ref vec);
    }
    public static usize Len(in VecPtr vec) => VecIntrinsics.chic_rt_vec_len(in vec);
    public static bool IsEmpty(in VecPtr vec) => Len(in vec) == 0;
    public static usize Capacity(in VecPtr vec) => VecIntrinsics.chic_rt_vec_capacity(in vec);
    public static VecViewPtr View(in VecPtr vec) {
        var viewPtr = CoreIntrinsics.DefaultValue<VecViewPtr>();
        let status = VecIntrinsics.chic_rt_vec_view(in vec, out viewPtr);
        if (status != VecError.Success)
        {
            return CoreIntrinsics.DefaultValue<VecViewPtr>();
        }
        return viewPtr;
    }
    public static VecError Push <T >(ref VecPtr vec, T value) {
        var slot = Std.Memory.MaybeUninit <T >.Init(value);
        return PushInitialized(ref vec, ref slot);
    }
    public static VecError Pop <T >(ref VecPtr vec, out T value) {
        var slot = Std.Memory.MaybeUninit <T >.Uninit();
        var status = PopInto(ref vec, ref slot);
        if (status == VecError.Success && slot.IsInitialized ())
        {
            value = slot.AssumeInit();
        }
        else
        {
            value = Std.Memory.Intrinsics.ZeroValue <T >();
        }
        return status;
    }
    public static VecError PopInto <T >(ref VecPtr vec, ref Std.Memory.MaybeUninit <T >slot) {
        slot.ForgetInit();
        let length = Len(in vec);
        if (length == 0)
        {
            return VecError.Success;
        }
        let handle = slot.AsValueMutPtr();
        var status = VecIntrinsics.chic_rt_vec_pop(ref vec, in handle);
        if (status == VecError.Success)
        {
            slot.MarkInitialized();
        }
        return status;
    }
    public static VecError PushInitialized <T >(ref VecPtr vec, ref Std.Memory.MaybeUninit <T >slot) {
        if (! slot.IsInitialized ())
        {
            return VecError.InvalidPointer;
        }
        let handle = slot.AsValueConstPtr();
        var status = VecIntrinsics.chic_rt_vec_push(ref vec, in handle);
        if (status == VecError.Success)
        {
            slot.ForgetInit();
        }
        return status;
    }
    public static VecError InsertInitialized <T >(ref VecPtr vec, usize index, ref Std.Memory.MaybeUninit <T >slot) {
        if (! slot.IsInitialized ())
        {
            return VecError.InvalidPointer;
        }
        let handle = slot.AsValueConstPtr();
        var status = VecIntrinsics.chic_rt_vec_insert(ref vec, index, in handle);
        if (status == VecError.Success)
        {
            slot.ForgetInit();
        }
        return status;
    }
    public static VecError RemoveInto <T >(ref VecPtr vec, usize index, ref Std.Memory.MaybeUninit <T >slot) {
        slot.ForgetInit();
        let length = Len(in vec);
        if (index >= length)
        {
            return VecError.OutOfBounds;
        }
        let handle = slot.AsValueMutPtr();
        var status = VecIntrinsics.chic_rt_vec_remove(ref vec, index, in handle);
        if (status == VecError.Success)
        {
            slot.MarkInitialized();
        }
        return status;
    }
    public static VecError SwapRemoveInto <T >(ref VecPtr vec, usize index, ref Std.Memory.MaybeUninit <T >slot) {
        slot.ForgetInit();
        let length = Len(in vec);
        if (index >= length)
        {
            return VecError.OutOfBounds;
        }
        let handle = slot.AsValueMutPtr();
        var status = VecIntrinsics.chic_rt_vec_swap_remove(ref vec, index, in handle);
        if (status == VecError.Success)
        {
            slot.MarkInitialized();
        }
        return status;
    }
    public static VecError Truncate(ref VecPtr vec, usize newLength) {
        return VecIntrinsics.chic_rt_vec_truncate(ref vec, newLength);
    }
    public static VecError Clear(ref VecPtr vec) {
        return VecIntrinsics.chic_rt_vec_clear(ref vec);
    }
    public static VecError Clone(ref VecPtr destination, in VecPtr source) {
        return VecIntrinsics.chic_rt_vec_clone(ref destination, in source);
    }
    public static Span <T >AsSpan <T >(ref VecPtr vec) {
        let handle = VecIntrinsics.chic_rt_vec_data_mut(ref vec);
        return Span <T >.FromValuePointer(handle, Len(in vec));
    }
    public static ReadOnlySpan <T >AsReadOnlySpan <T >(in VecPtr vec) {
        let handle = VecIntrinsics.chic_rt_vec_data(in vec);
        return ReadOnlySpan <T >.FromValuePointer(handle, Len(in vec));
    }
    public static VecIterPtr Iter(in VecPtr vec) {
        return VecIntrinsics.chic_rt_vec_iter(in vec);
    }
    public static bool IterNext <T >(ref VecIterPtr iter, out T value) {
        var slot = Std.Memory.MaybeUninit <T >.Uninit();
        let handle = slot.AsValueMutPtr();
        var status = VecIntrinsics.chic_rt_vec_iter_next(ref iter, in handle);
        if (status == VecError.Success)
        {
            slot.MarkInitialized();
            value = slot.AssumeInit();
            return true;
        }
        value = Std.Memory.Intrinsics.ZeroValue <T >();
        return false;
    }
    public static ValueConstPtr IterNextPtr(ref VecIterPtr iter) {
        return VecIntrinsics.chic_rt_vec_iter_next_ptr(ref iter);
    }
    public static ValueConstPtr Data(in VecPtr vec) {
        return VecIntrinsics.chic_rt_vec_data(in vec);
    }
}

static class VecTestHelpers
{
    public static VecPtr NewVecWith1(int first) {
        var vec = Vec.New<int>();
        let _ = Vec.Push<int>(ref vec, first);
        return vec;
    }
    public static VecPtr NewVecWith2(int first, int second) {
        var vec = Vec.New<int>();
        let _ = Vec.Push<int>(ref vec, first);
        let _ = Vec.Push<int>(ref vec, second);
        return vec;
    }
    public static VecPtr NewVecWith3(int first, int second, int third) {
        var vec = Vec.New<int>();
        let _ = Vec.Push<int>(ref vec, first);
        let _ = Vec.Push<int>(ref vec, second);
        let _ = Vec.Push<int>(ref vec, third);
        return vec;
    }
    public static void Drop(ref VecPtr vec) {
        VecIntrinsics.chic_rt_vec_drop(ref vec);
    }
}

testcase Given_vec_new_len_is_zero_When_executed_Then_vec_new_len_is_zero()
{
    var vec = Vec.New<int>();
    Assert.That(Vec.Len(in vec) == 0usize).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_new_is_empty_When_executed_Then_vec_new_is_empty()
{
    var vec = Vec.New<int>();
    Assert.That(Vec.IsEmpty(in vec)).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_push_returns_success_When_executed_Then_vec_push_returns_success()
{
    var vec = Vec.New<int>();
    let status = Vec.Push<int>(ref vec, 10);
    Assert.That(status == VecError.Success).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_push_increments_len_When_executed_Then_vec_push_increments_len()
{
    var vec = VecTestHelpers.NewVecWith1(10);
    Assert.That(Vec.Len(in vec) == 1usize).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_pop_returns_success_When_executed_Then_vec_pop_returns_success()
{
    var vec = VecTestHelpers.NewVecWith2(10, 20);
    let status = Vec.Pop<int>(ref vec, out var value);
    let _ = value;
    Assert.That(status == VecError.Success).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_pop_returns_last_value_When_executed_Then_vec_pop_returns_last_value()
{
    var vec = VecTestHelpers.NewVecWith2(10, 20);
    let _ = Vec.Pop<int>(ref vec, out var value);
    Assert.That(value == 20).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_pop_decrements_len_When_executed_Then_vec_pop_decrements_len()
{
    var vec = VecTestHelpers.NewVecWith2(10, 20);
    let _ = Vec.Pop<int>(ref vec, out var value);
    let _ = value;
    Assert.That(Vec.Len(in vec) == 1usize).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_first_is_true_When_executed_Then_vec_iter_first_is_true()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let ok1 = Vec.IterNext<int>(ref iter, out var first);
    let _ = first;
    Assert.That(ok1).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_second_is_true_When_executed_Then_vec_iter_second_is_true()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    let ok2 = Vec.IterNext<int>(ref iter, out var second);
    let _ = first;
    let _ = second;
    Assert.That(ok2).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_third_is_true_When_executed_Then_vec_iter_third_is_true()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    let _ = Vec.IterNext<int>(ref iter, out var second);
    let ok3 = Vec.IterNext<int>(ref iter, out var third);
    let _ = first;
    let _ = second;
    let _ = third;
    Assert.That(ok3).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_after_end_is_false_When_executed_Then_vec_iter_after_end_is_false()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    let _ = Vec.IterNext<int>(ref iter, out var second);
    let _ = Vec.IterNext<int>(ref iter, out var third);
    let ok4 = Vec.IterNext<int>(ref iter, out var last);
    let _ = first;
    let _ = second;
    let _ = third;
    let _ = last;
    Assert.That(ok4).IsFalse();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_first_value_When_executed_Then_vec_iter_first_value()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    Assert.That(first == 1).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_second_value_When_executed_Then_vec_iter_second_value()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    let _ = Vec.IterNext<int>(ref iter, out var second);
    let _ = first;
    Assert.That(second == 2).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_third_value_When_executed_Then_vec_iter_third_value()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    let _ = Vec.IterNext<int>(ref iter, out var second);
    let _ = Vec.IterNext<int>(ref iter, out var third);
    let _ = first;
    let _ = second;
    Assert.That(third == 3).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_iter_after_end_default_value_When_executed_Then_vec_iter_after_end_default_value()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    var iter = Vec.Iter(in vec);
    let _ = Vec.IterNext<int>(ref iter, out var first);
    let _ = Vec.IterNext<int>(ref iter, out var second);
    let _ = Vec.IterNext<int>(ref iter, out var third);
    let _ = Vec.IterNext<int>(ref iter, out var last);
    let _ = first;
    let _ = second;
    let _ = third;
    Assert.That(last == 0).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_remove_returns_success_When_executed_Then_vec_remove_returns_success()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var removed = MaybeUninit<int>.Uninit();
    let status = Vec.RemoveInto<int>(ref vec, 1usize, ref removed);
    Assert.That(status == VecError.Success).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_remove_initializes_output_When_executed_Then_vec_remove_initializes_output()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var removed = MaybeUninit<int>.Uninit();
    let _ = Vec.RemoveInto<int>(ref vec, 1usize, ref removed);
    Assert.That(removed.IsInitialized()).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_remove_output_value_When_executed_Then_vec_remove_output_value()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var removed = MaybeUninit<int>.Uninit();
    let _ = Vec.RemoveInto<int>(ref vec, 1usize, ref removed);
    Assert.That<int>(removed.AssumeInit()).IsEqualTo(20);
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_swap_remove_returns_success_When_executed_Then_vec_swap_remove_returns_success()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var swapped = MaybeUninit<int>.Uninit();
    let status = Vec.SwapRemoveInto<int>(ref vec, 0usize, ref swapped);
    Assert.That(status == VecError.Success).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_swap_remove_initializes_output_When_executed_Then_vec_swap_remove_initializes_output()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var swapped = MaybeUninit<int>.Uninit();
    let _ = Vec.SwapRemoveInto<int>(ref vec, 0usize, ref swapped);
    Assert.That(swapped.IsInitialized()).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_swap_remove_output_value_When_executed_Then_vec_swap_remove_output_value()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var swapped = MaybeUninit<int>.Uninit();
    let _ = Vec.SwapRemoveInto<int>(ref vec, 0usize, ref swapped);
    Assert.That<int>(swapped.AssumeInit()).IsEqualTo(10);
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_remove_updates_len_When_executed_Then_vec_remove_updates_len()
{
    var vec = VecTestHelpers.NewVecWith3(10, 20, 30);
    var removed = MaybeUninit<int>.Uninit();
    let _ = Vec.RemoveInto<int>(ref vec, 1usize, ref removed);
    Assert.That(Vec.Len(in vec) == 2usize).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_remove_out_of_bounds_returns_error_When_executed_Then_vec_remove_out_of_bounds_returns_error()
{
    var vec = VecTestHelpers.NewVecWith1(10);
    var out_of_bounds = MaybeUninit<int>.Uninit();
    let status = Vec.RemoveInto<int>(ref vec, 4usize, ref out_of_bounds);
    Assert.That(status == VecError.OutOfBounds).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_truncate_returns_success_When_executed_Then_vec_truncate_returns_success()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    let status = Vec.Truncate(ref vec, 2usize);
    Assert.That(status == VecError.Success).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_truncate_updates_len_When_executed_Then_vec_truncate_updates_len()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    let _ = Vec.Truncate(ref vec, 2usize);
    Assert.That(Vec.Len(in vec) == 2usize).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_clear_returns_success_When_executed_Then_vec_clear_returns_success()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    let status = Vec.Clear(ref vec);
    Assert.That(status == VecError.Success).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_clear_resets_len_When_executed_Then_vec_clear_resets_len()
{
    var vec = VecTestHelpers.NewVecWith3(1, 2, 3);
    let _ = Vec.Clear(ref vec);
    Assert.That(Vec.Len(in vec) == 0usize).IsTrue();
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_as_span_length_When_executed_Then_vec_as_span_length()
{
    var vec = VecTestHelpers.NewVecWith1(42);
    {
        let span = Vec.AsSpan<int>(ref vec);
        Assert.That(span.Length == 1usize).IsTrue();
    }
    VecTestHelpers.Drop(ref vec);
}

testcase Given_vec_as_read_only_span_length_When_executed_Then_vec_as_read_only_span_length()
{
    var vec = VecTestHelpers.NewVecWith1(42);
    {
        let ro = Vec.AsReadOnlySpan<int>(in vec);
        Assert.That(ro.Length == 1usize).IsTrue();
    }
    VecTestHelpers.Drop(ref vec);
}
