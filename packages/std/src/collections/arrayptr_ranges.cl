namespace Std.Collections;
import Std.Range;
import Std.Span;
public extension ArrayPtr
{
    public Span <T >AsSpan <T >(ref this ArrayPtr array) {
        return Span <T >.FromArray(ref array);
    }
    public ReadOnlySpan <T >AsReadOnlySpan <T >(in this ArrayPtr array) {
        return ReadOnlySpan <T >.FromArray(in array);
    }
    public Span <T >Slice <T >(ref this ArrayPtr array, Range range) {
        let bounds = SpanGuards.ResolveRangeExclusive(range, array.Length);
        return Span <T >.FromArray(ref array).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this ArrayPtr array, RangeInclusive range) {
        let bounds = SpanGuards.ResolveRangeInclusive(range, array.Length);
        return Span <T >.FromArray(ref array).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this ArrayPtr array, RangeFrom range) {
        let bounds = SpanGuards.ResolveRangeFrom(range, array.Length);
        return Span <T >.FromArray(ref array).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this ArrayPtr array, RangeTo range) {
        let bounds = SpanGuards.ResolveRangeTo(range, array.Length);
        return Span <T >.FromArray(ref array).Slice(bounds.Start, bounds.Length);
    }
    public Span <T >Slice <T >(ref this ArrayPtr array, RangeFull range) {
        return Span <T >.FromArray(ref array);
    }
}
