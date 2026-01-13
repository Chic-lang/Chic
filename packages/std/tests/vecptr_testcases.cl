namespace Std.Collections;
import Std.Range;
import Std.Span;
import Std.Testing;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
static class VecPtrTestHelpers
{
    public static Vec <int >NewEmpty() {
        return VecIntrinsics.Create <int >();
    }
    public static Vec <int >NewWith1(int first) {
        var vec = VecIntrinsics.Create <int >();
        let _ = VecUtil.Push <int >(ref vec, first);
        return vec;
    }
    public static Vec <int >NewWith3(int first, int second, int third) {
        var vec = VecIntrinsics.Create <int >();
        let _ = VecUtil.Push <int >(ref vec, first);
        let _ = VecUtil.Push <int >(ref vec, second);
        let _ = VecUtil.Push <int >(ref vec, third);
        return vec;
    }
    public static void Drop(ref Vec <int >vec) {
        FVecIntrinsics.chic_rt_vec_drop(ref vec);
    }
}
testcase Given_vecutil_push_returns_success_When_executed_Then_vecutil_push_returns_success()
{
    var vec = VecPtrTestHelpers.NewEmpty();
    let pushed = VecUtil.Push <int >(ref vec, 5);
    Assert.That(pushed).IsEqualTo(VecError.Success);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecutil_trypop_returns_true_When_executed_Then_vecutil_trypop_returns_true()
{
    var vec = VecPtrTestHelpers.NewWith1(5);
    let popped = VecUtil.TryPop <int >(ref vec, out var value);
    Assert.That(popped).IsTrue();
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecutil_trypop_returns_value_When_executed_Then_vecutil_trypop_returns_value()
{
    var vec = VecPtrTestHelpers.NewWith1(5);
    let _ = VecUtil.TryPop <int >(ref vec, out var value);
    Assert.That(value).IsEqualTo(5);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecutil_trypop_empty_returns_false_When_executed_Then_vecutil_trypop_empty_returns_false()
{
    var vec = VecPtrTestHelpers.NewEmpty();
    let empty = VecUtil.TryPop <int >(ref vec, out var missing);
    Assert.That(empty).IsFalse();
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecutil_trypop_empty_returns_default_value_When_executed_Then_vecutil_trypop_empty_returns_default_value()
{
    var vec = VecPtrTestHelpers.NewEmpty();
    let _ = VecUtil.TryPop <int >(ref vec, out var missing);
    Assert.That(missing).IsEqualTo(0);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecptr_slice_range_length_When_executed_Then_vecptr_slice_range_length()
{
    var vec = VecPtrTestHelpers.NewWith3(1, 2, 3);
    var range = new Range();
    range.Start = Index.FromStart(0usize);
    range.End = Index.FromStart(2usize);
    let slice = vec.Slice <int >(range);
    Assert.That(slice.Length).IsEqualTo(2usize);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecptr_slice_range_inclusive_length_When_executed_Then_vecptr_slice_range_inclusive_length()
{
    var vec = VecPtrTestHelpers.NewWith3(1, 2, 3);
    var inclusive = new RangeInclusive();
    inclusive.Start = Index.FromStart(0usize);
    inclusive.End = Index.FromStart(1usize);
    let slice_inclusive = vec.Slice <int >(inclusive);
    Assert.That(slice_inclusive.Length).IsEqualTo(2usize);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecptr_slice_range_from_length_When_executed_Then_vecptr_slice_range_from_length()
{
    var vec = VecPtrTestHelpers.NewWith3(1, 2, 3);
    var from = new RangeFrom();
    from.Start = Index.FromStart(1usize);
    let slice_from = vec.Slice <int >(from);
    Assert.That(slice_from.Length).IsEqualTo(2usize);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecptr_slice_range_to_length_When_executed_Then_vecptr_slice_range_to_length()
{
    var vec = VecPtrTestHelpers.NewWith3(1, 2, 3);
    var to = new RangeTo();
    to.End = Index.FromStart(2usize);
    let slice_to = vec.Slice <int >(to);
    Assert.That(slice_to.Length).IsEqualTo(2usize);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecptr_slice_range_full_length_When_executed_Then_vecptr_slice_range_full_length()
{
    var vec = VecPtrTestHelpers.NewWith3(1, 2, 3);
    let full = new RangeFull();
    let slice_full = vec.Slice <int >(full);
    Assert.That(slice_full.Length).IsEqualTo(3usize);
    VecPtrTestHelpers.Drop(ref vec);
}
testcase Given_vecptr_as_readonly_span_When_executed_Then_values_match()
{
    var vec = VecPtrTestHelpers.NewWith3(1, 2, 3);
    let span = VecPtr.AsReadOnlySpan <int >(in vec);
    Assert.That(span.Length).IsEqualTo(3usize);
    Assert.That(span[0usize]).IsEqualTo(1);
    Assert.That(span[1usize]).IsEqualTo(2);
    Assert.That(span[2usize]).IsEqualTo(3);
    VecPtrTestHelpers.Drop(ref vec);
}
