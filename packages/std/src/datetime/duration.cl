namespace Std.Datetime;
import Std.Numeric;
import Std.Span;
import Std.Strings;
public struct Duration
{
    private readonly long _ticks;
    private init(long ticks) {
        _ticks = ticks;
    }
    public static Duration Zero => new Duration(0L);
    public static Duration Infinite => new Duration(- 1L);
    public static Duration FromTicks(long ticks) => new Duration(ticks);
    public static Duration FromSeconds(long seconds) {
        return new Duration(seconds * DateTimeConstants.TicksPerSecond);
    }
    public static Duration FromMilliseconds(long millis) {
        return new Duration(millis * DateTimeConstants.TicksPerMillisecond);
    }
    public long Ticks => _ticks;
    public double TotalSeconds => NumericUnchecked.ToFloat64(_ticks) / NumericUnchecked.ToFloat64(DateTimeConstants.TicksPerSecond);
    public static Duration Add(Duration left, Duration right) {
        var sum = left._ticks + right._ticks;
        return new Duration(sum);
    }
    public static Duration Subtract(Duration left, Duration right) {
        var diff = left._ticks - right._ticks;
        return new Duration(diff);
    }
    public Duration Negate() => new Duration(- _ticks);
    public static int Compare(Duration left, Duration right) {
        if (left._ticks <right._ticks)
        {
            return - 1;
        }
        if (left._ticks >right._ticks)
        {
            return 1;
        }
        return 0;
    }
    public bool Equals(Duration other) => _ticks == other._ticks;
    public override string ToString() => DateTimeFormatting.FormatDuration(this, null);
    public string ToString(string format) => DateTimeFormatting.FormatDuration(this, format);
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return DateTimeFormatting.TryFormatDuration(this, destination, out written, format);
    }
}
