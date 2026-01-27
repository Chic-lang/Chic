namespace Std.Collections;
import Std.Range;
import Std.Span;
public static class StringRanges
{
    public static ReadOnlySpan <byte >Slice(this string value, Range range) {
        let slice = Std.Span.ReadOnlySpan.FromString(value);
        return slice.Slice(range);
    }
    public static ReadOnlySpan <byte >Slice(this string value, RangeInclusive range) {
        let slice = Std.Span.ReadOnlySpan.FromString(value);
        return slice.Slice(range);
    }
    public static ReadOnlySpan <byte >Slice(this string value, RangeFrom range) {
        let slice = Std.Span.ReadOnlySpan.FromString(value);
        return slice.Slice(range);
    }
    public static ReadOnlySpan <byte >Slice(this string value, RangeTo range) {
        let slice = Std.Span.ReadOnlySpan.FromString(value);
        return slice.Slice(range);
    }
    public static ReadOnlySpan <byte >Slice(this string value, RangeFull range) {
        let slice = Std.Span.ReadOnlySpan.FromString(value);
        return slice;
    }
}
