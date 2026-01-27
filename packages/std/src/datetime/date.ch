namespace Std.Datetime;
import Std.Numeric;
public struct Date
{
    public readonly int Year;
    public readonly int Month;
    public readonly int Day;
    private readonly long _ticksAtMidnight;
    private init(int year, int month, int day, long ticksAtMidnight) {
        Year = year;
        Month = month;
        Day = day;
        _ticksAtMidnight = ticksAtMidnight;
    }
    internal static Date FromTicks(long ticks, int year, int month, int day) {
        return new Date(year, month, day, ticks);
    }
    public static bool TryCreate(int year, int month, int day, out Date value) {
        value = new Date(0, 0, 0, 0L);
        if (!DateTimeUtil.TryDateToTicks (year, month, day, out var ticks)) {
            return false;
        }
        value = new Date(year, month, day, ticks);
        return true;
    }
    public static Date FromParts(int year, int month, int day) {
        if (!TryCreate (year, month, day, out var value)) {
            throw new Std.ArgumentOutOfRangeException("Invalid date");
        }
        return value;
    }
    public long TicksAtMidnight => _ticksAtMidnight;
    public int DayOfYear {
        get {
            var total = 0;
            var m = 1;
            while (m <Month)
            {
                total += DateTimeUtil.DaysInMonth(Year, m);
                m += 1;
            }
            return total + Day;
        }
    }
    public int DayOfWeek => DateTimeUtil.DayOfWeekFromTicks(_ticksAtMidnight);
}
