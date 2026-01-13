namespace Std.Datetime;
import Std.Numeric;
public struct Time
{
    public readonly int Hour;
    public readonly int Minute;
    public readonly int Second;
    public readonly int Nanosecond;
    private readonly long _ticksOfDay;
    private init(int hour, int minute, int second, int nanosecond, long ticksOfDay) {
        Hour = hour;
        Minute = minute;
        Second = second;
        Nanosecond = nanosecond;
        _ticksOfDay = ticksOfDay;
    }
    public static bool TryCreate(int hour, int minute, int second, int nanosecond, out Std.Datetime.Time value) {
        value = new Time(0, 0, 0, 0, 0L);
        if (! DateTimeUtil.TryTimeToTicks (hour, minute, second, nanosecond, out var ticks)) {
            return false;
        }
        value = new Time(hour, minute, second, nanosecond, ticks);
        return true;
    }
    public static Std.Datetime.Time FromParts(int hour, int minute, int second, int nanosecond) {
        if (! TryCreate (hour, minute, second, nanosecond, out var value)) {
            throw new Std.ArgumentOutOfRangeException("Invalid time");
        }
        return value;
    }
    public long TicksOfDay => _ticksOfDay;
    internal static Std.Datetime.Time FromTicksOfDay(long ticksOfDay) {
        var remaining = ticksOfDay;
        let hour = NumericUnchecked.ToInt32(remaining / DateTimeConstants.TicksPerHour);
        remaining -= NumericUnchecked.ToInt64(hour) * DateTimeConstants.TicksPerHour;
        let minute = NumericUnchecked.ToInt32(remaining / DateTimeConstants.TicksPerMinute);
        remaining -= NumericUnchecked.ToInt64(minute) * DateTimeConstants.TicksPerMinute;
        let second = NumericUnchecked.ToInt32(remaining / DateTimeConstants.TicksPerSecond);
        remaining -= NumericUnchecked.ToInt64(second) * DateTimeConstants.TicksPerSecond;
        let nano = NumericUnchecked.ToInt32(remaining * 100L);
        return new Time(hour, minute, second, nano, ticksOfDay);
    }
}
