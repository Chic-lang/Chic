namespace Std.Range;
import Std.Runtime.Collections;
import Std.Core;
import Std.Core.Testing;
public enum RangeError
{
    Success = 0, OutOfBounds = 2, Invalid = 3,
}
public struct Index
{
    public usize Value;
    public bool FromEnd;
    public init(usize value, bool fromEnd) {
        Value = value;
        FromEnd = fromEnd;
    }
    public static Index FromStart(usize value) {
        return new Index(value, false);
    }
    public static Index FromEnd(usize value) {
        return new Index(value, true);
    }
}
public struct Range
{
    public Index Start;
    public Index End;
}
public struct RangeFrom
{
    public Index Start;
}
public struct RangeTo
{
    public Index End;
}
public struct RangeInclusive
{
    public Index Start;
    public Index End;
}
public struct RangeFull
{
}
public struct RangeBounds
{
    public usize Start;
    public usize Length;
}
internal static class RangeMath
{
    public static bool TryOffset(Index index, usize length, out usize offset) {
        if (index.FromEnd && index.Value == 0)
        {
            offset = 0;
            return false;
        }
        if (index.FromEnd)
        {
            if (index.Value >length)
            {
                offset = 0;
                return false;
            }
            offset = length - index.Value;
            return true;
        }
        offset = index.Value;
        return index.Value <= length;
    }
    public static bool TryResolve(Range range, usize length, out RangeBounds bounds) {
        return TryResolve(range.Start, range.End, length, false, out bounds);
    }
    public static bool TryResolveInclusive(RangeInclusive range, usize length, out RangeBounds bounds) {
        return TryResolve(range.Start, range.End, length, true, out bounds);
    }
    public static RangeBounds Resolve(Range range, usize length) {
        var bounds = CoreIntrinsics.DefaultValue <RangeBounds >();
        RangeGuards.Assert(TryResolve(range, length, out bounds), RangeError.OutOfBounds);
        return bounds;
    }
    public static RangeBounds ResolveInclusive(RangeInclusive range, usize length) {
        var bounds = CoreIntrinsics.DefaultValue <RangeBounds >();
        RangeGuards.Assert(TryResolveInclusive(range, length, out bounds), RangeError.OutOfBounds);
        return bounds;
    }
    public static bool TryResolveFrom(RangeFrom range, usize length, out RangeBounds bounds) {
        var resolved = CoreIntrinsics.DefaultValue <RangeBounds >();
        var startOffset = 0usize;
        if (!TryOffset (range.Start, length, out startOffset)) {
            bounds = resolved;
            return false;
        }
        resolved.Start = startOffset;
        if (startOffset >length)
        {
            bounds = resolved;
            return false;
        }
        resolved.Length = length - startOffset;
        bounds = resolved;
        return true;
    }
    public static RangeBounds ResolveFrom(RangeFrom range, usize length) {
        var bounds = CoreIntrinsics.DefaultValue <RangeBounds >();
        RangeGuards.Assert(TryResolveFrom(range, length, out bounds), RangeError.OutOfBounds);
        return bounds;
    }
    public static bool TryResolveTo(RangeTo range, usize length, out RangeBounds bounds) {
        var resolved = CoreIntrinsics.DefaultValue <RangeBounds >();
        var end = 0usize;
        if (!TryOffset (range.End, length, out end)) {
            bounds = resolved;
            return false;
        }
        resolved.Length = end;
        bounds = resolved;
        return true;
    }
    public static RangeBounds ResolveTo(RangeTo range, usize length) {
        var bounds = CoreIntrinsics.DefaultValue <RangeBounds >();
        RangeGuards.Assert(TryResolveTo(range, length, out bounds), RangeError.OutOfBounds);
        return bounds;
    }
    public static RangeBounds ResolveFull(usize length) {
        var bounds = CoreIntrinsics.DefaultValue <RangeBounds >();
        bounds.Start = 0;
        bounds.Length = length;
        return bounds;
    }
    private static bool TryResolve(Index start, Index end, usize length, bool inclusive, out RangeBounds bounds) {
        var resolved = CoreIntrinsics.DefaultValue <RangeBounds >();
        var startOffset = 0usize;
        if (!TryOffset (start, length, out startOffset)) {
            bounds = resolved;
            return false;
        }
        resolved.Start = startOffset;
        var endOffset = 0usize;
        if (!TryOffset (end, length, out endOffset)) {
            bounds = resolved;
            return false;
        }
        if (endOffset <resolved.Start)
        {
            bounds = resolved;
            return false;
        }
        if (inclusive)
        {
            if (endOffset >= length)
            {
                bounds = resolved;
                return false;
            }
            resolved.Length = endOffset - resolved.Start + 1usize;
        }
        else
        {
            resolved.Length = endOffset - resolved.Start;
        }
        bounds = resolved;
        return true;
    }
}
testcase Given_range_try_resolve_exclusive_returns_true_When_executed_Then_range_try_resolve_exclusive_returns_true()
{
    var range = new Range();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromStart(4usize);
    let ok = RangeMath.TryResolve(range, 6usize, out var bounds);
    let _ = bounds;
    Assert.That(ok).IsTrue();
}
testcase Given_range_try_resolve_exclusive_start_When_executed_Then_range_try_resolve_exclusive_start()
{
    var range = new Range();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromStart(4usize);
    let _ = RangeMath.TryResolve(range, 6usize, out var bounds);
    Assert.That(bounds.Start == 1usize).IsTrue();
}
testcase Given_range_try_resolve_exclusive_length_When_executed_Then_range_try_resolve_exclusive_length()
{
    var range = new Range();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromStart(4usize);
    let _ = RangeMath.TryResolve(range, 6usize, out var bounds);
    Assert.That(bounds.Length == 3usize).IsTrue();
}
testcase Given_range_try_resolve_out_of_bounds_returns_false_When_executed_Then_range_try_resolve_out_of_bounds_returns_false()
{
    var range = new Range();
    range.Start = Index.FromStart(2usize);
    range.End = Index.FromStart(9usize);
    let ok = RangeMath.TryResolve(range, 4usize, out var bounds);
    let _ = bounds;
    Assert.That(ok).IsFalse();
}
testcase Given_range_try_resolve_out_of_bounds_start_zero_When_executed_Then_range_try_resolve_out_of_bounds_start_zero()
{
    var range = new Range();
    range.Start = Index.FromStart(2usize);
    range.End = Index.FromStart(9usize);
    let _ = RangeMath.TryResolve(range, 4usize, out var bounds);
    Assert.That(bounds.Start == 0usize).IsTrue();
}
testcase Given_range_try_resolve_out_of_bounds_length_zero_When_executed_Then_range_try_resolve_out_of_bounds_length_zero()
{
    var range = new Range();
    range.Start = Index.FromStart(2usize);
    range.End = Index.FromStart(9usize);
    let _ = RangeMath.TryResolve(range, 4usize, out var bounds);
    Assert.That(bounds.Length == 0usize).IsTrue();
}
testcase Given_range_resolve_inclusive_start_When_executed_Then_range_resolve_inclusive_start()
{
    var range = new RangeInclusive();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromStart(2usize);
    let bounds = RangeMath.ResolveInclusive(range, 5usize);
    Assert.That(bounds.Start == 1usize).IsTrue();
}
testcase Given_range_resolve_inclusive_length_When_executed_Then_range_resolve_inclusive_length()
{
    var range = new RangeInclusive();
    range.Start = Index.FromStart(1usize);
    range.End = Index.FromStart(2usize);
    let bounds = RangeMath.ResolveInclusive(range, 5usize);
    Assert.That(bounds.Length == 2usize).IsTrue();
}
testcase Given_range_resolve_from_start_When_executed_Then_range_resolve_from_start()
{
    var from = new RangeFrom();
    from.Start = Index.FromStart(2usize);
    let fromBounds = RangeMath.ResolveFrom(from, 6usize);
    Assert.That(fromBounds.Start == 2usize).IsTrue();
}
testcase Given_range_resolve_from_length_When_executed_Then_range_resolve_from_length()
{
    var from = new RangeFrom();
    from.Start = Index.FromStart(2usize);
    let fromBounds = RangeMath.ResolveFrom(from, 6usize);
    Assert.That(fromBounds.Length == 4usize).IsTrue();
}
testcase Given_range_resolve_to_start_When_executed_Then_range_resolve_to_start()
{
    var to = new RangeTo();
    to.End = Index.FromStart(3usize);
    let toBounds = RangeMath.ResolveTo(to, 6usize);
    Assert.That(toBounds.Start == 0usize).IsTrue();
}
testcase Given_range_resolve_to_length_When_executed_Then_range_resolve_to_length()
{
    var to = new RangeTo();
    to.End = Index.FromStart(3usize);
    let toBounds = RangeMath.ResolveTo(to, 6usize);
    Assert.That(toBounds.Length == 3usize).IsTrue();
}
testcase Given_range_resolve_throws_on_invalid_When_executed_Then_range_resolve_throws_on_invalid()
{
    var range = new Range();
    range.Start = Index.FromEnd(1usize);
    range.End = Index.FromEnd(0usize);
    var threw = false;
    try {
        let _ = RangeMath.Resolve(range, 0usize);
    }
    catch(ArgumentOutOfRangeException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}
testcase Given_range_try_resolve_from_returns_true_When_executed_Then_range_try_resolve_from_returns_true()
{
    var from = new RangeFrom();
    from.Start = Index.FromStart(1usize);
    let okFrom = RangeMath.TryResolveFrom(from, 3usize, out var fromBounds);
    let _ = fromBounds;
    Assert.That(okFrom).IsTrue();
}
testcase Given_range_try_resolve_from_start_When_executed_Then_range_try_resolve_from_start()
{
    var from = new RangeFrom();
    from.Start = Index.FromStart(1usize);
    let _ = RangeMath.TryResolveFrom(from, 3usize, out var fromBounds);
    Assert.That(fromBounds.Start == 1usize).IsTrue();
}
testcase Given_range_try_resolve_from_length_When_executed_Then_range_try_resolve_from_length()
{
    var from = new RangeFrom();
    from.Start = Index.FromStart(1usize);
    let _ = RangeMath.TryResolveFrom(from, 3usize, out var fromBounds);
    Assert.That(fromBounds.Length == 2usize).IsTrue();
}
testcase Given_range_try_resolve_to_returns_true_When_executed_Then_range_try_resolve_to_returns_true()
{
    var to = new RangeTo();
    to.End = Index.FromEnd(1usize);
    let okTo = RangeMath.TryResolveTo(to, 4usize, out var toBounds);
    let _ = toBounds;
    Assert.That(okTo).IsTrue();
}
testcase Given_range_try_resolve_to_length_When_executed_Then_range_try_resolve_to_length()
{
    var to = new RangeTo();
    to.End = Index.FromEnd(1usize);
    let _ = RangeMath.TryResolveTo(to, 4usize, out var toBounds);
    Assert.That(toBounds.Length == 3usize).IsTrue();
}
testcase Given_range_try_offset_rejects_from_end_zero_When_executed_Then_range_try_offset_rejects_from_end_zero()
{
    let index = Index.FromEnd(0usize);
    let ok = RangeMath.TryOffset(index, 4usize, out var offset);
    let _ = offset;
    Assert.That(ok).IsFalse();
}
testcase Given_range_try_offset_from_end_zero_offset_is_zero_When_executed_Then_range_try_offset_from_end_zero_offset_is_zero()
{
    let index = Index.FromEnd(0usize);
    let _ = RangeMath.TryOffset(index, 4usize, out var offset);
    Assert.That(offset == 0usize).IsTrue();
}
testcase Given_range_guards_invalid_branch_throws_When_executed_Then_range_guards_invalid_branch_throws()
{
    var threw = false;
    try {
        RangeGuards.Assert(false, RangeError.Invalid);
    }
    catch(ArgumentException) {
        threw = true;
    }
    Assert.That(threw).IsTrue();
}
public static class RangeGuards
{
    public static void Assert(bool condition, RangeError rangeError) {
        if (condition)
        {
            return;
        }
        if (rangeError == RangeError.OutOfBounds)
        {
            throw new Std.ArgumentOutOfRangeException("range is out of bounds");
        }
        if (rangeError == RangeError.Invalid)
        {
            throw new Std.ArgumentException("range is invalid");
        }
        throw new Std.InvalidOperationException("unknown range error");
    }
}
