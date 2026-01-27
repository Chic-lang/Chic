namespace Std.Datetime;
import Std.Numeric;
internal static class DateTimeUtil
{
    private static int DaysBeforeMonth(int year, int month) {
        var leap = IsLeapYear(year);
        switch (month)
        {
            case 1:
                return 0;
            case 2:
                return 31;
            case 3:
                return leap ?60 : 59;
            case 4:
                return leap ?91 : 90;
            case 5:
                return leap ?121 : 120;
            case 6:
                return leap ?152 : 151;
            case 7:
                return leap ?182 : 181;
            case 8:
                return leap ?213 : 212;
            case 9:
                return leap ?244 : 243;
            case 10:
                return leap ?274 : 273;
            case 11:
                return leap ?305 : 304;
            case 12:
                return leap ?335 : 334;
            default :
                return 0;
            }
        }
        public static int DaysInMonth(int year, int month) {
            switch (month)
            {
                case 1:
                case 3:
                case 5:
                case 7:
                case 8:
                case 10:
                case 12:
                    return 31;
                case 4:
                case 6:
                case 9:
                case 11:
                    return 30;
                case 2:
                    return IsLeapYear(year) ?29 : 28;
                default :
                    return 0;
                }
            }
            public static bool IsLeapYear(int year) {
                if (year <1 || year >9999)
                {
                    return false;
                }
                if ( (year % 4) != 0)
                {
                    return false;
                }
                if ( (year % 100) == 0)
                {
                    return(year % 400) == 0;
                }
                return true;
            }
            public static bool TryDateToTicks(int year, int month, int day, out long ticks) {
                ticks = 0L;
                if (year <1 || year >9999 || month <1 || month >12)
                {
                    return false;
                }
                var monthStart = DaysBeforeMonth(year, month);
                var daysInMonth = DaysInMonth(year, month);
                var monthEnd = monthStart + daysInMonth;
                if (day <1 || day >daysInMonth)
                {
                    return false;
                }
                var yearMinusOne = year - 1;
                var totalDays = yearMinusOne * 365 + yearMinusOne / 4 - yearMinusOne / 100 + yearMinusOne / 400;
                totalDays += monthStart + (day - 1);
                var days64 = NumericUnchecked.ToInt64(totalDays);
                ticks = days64 * DateTimeConstants.TicksPerDay;
                return true;
            }
            public static bool TryTimeToTicks(int hour, int minute, int second, int nanosecond, out long ticks) {
                ticks = 0L;
                if (hour <0 || hour >23 || minute <0 || minute >59 || second <0 || second >59)
                {
                    return false;
                }
                if (nanosecond <0 || nanosecond >= 1_000_000_000)
                {
                    return false;
                }
                var totalSeconds = (long) hour * 3600L + (long) minute * 60L + (long) second;
                var totalTicks = totalSeconds * DateTimeConstants.TicksPerSecond;
                var fracTicks = nanosecond / 100;
                // 100 ns units
                totalTicks += fracTicks;
                if (totalTicks <0 || totalTicks >= DateTimeConstants.TicksPerDay)
                {
                    return false;
                }
                ticks = totalTicks;
                return true;
            }
            public static int DayOfWeekFromTicks(long ticks) {
                // 0001-01-01 was a Monday; DayOfWeek 0=Sunday in .NET
                var days = ticks / DateTimeConstants.TicksPerDay;
                var dayOfWeek = (days + 1L) % 7L;
                return NumericUnchecked.ToInt32(dayOfWeek);
            }
            public static void GetDateParts(long ticks, out int year, out int month, out int day) {
                var days = ticks / DateTimeConstants.TicksPerDay;
                var y400 = NumericUnchecked.ToInt32(days / 146097L);
                var daysLeft = days - (long) y400 * 146097L;
                var y100 = NumericUnchecked.ToInt32(daysLeft / 36524L);
                if (y100 == 4)
                {
                    y100 = 3;
                }
                daysLeft -= (long) y100 * 36524L;
                var y4 = NumericUnchecked.ToInt32(daysLeft / 1461L);
                daysLeft -= (long) y4 * 1461L;
                var y1 = NumericUnchecked.ToInt32(daysLeft / 365L);
                if (y1 == 4)
                {
                    y1 = 3;
                }
                daysLeft -= (long) y1 * 365L;
                year = y400 * 400 + y100 * 100 + y4 * 4 + y1 + 1;
                var remaining = NumericUnchecked.ToInt32(daysLeft);
                var m = 1;
                while (m <= 12)
                {
                    var dim = DaysInMonth(year, m);
                    if (remaining <dim)
                    {
                        break;
                    }
                    remaining -= dim;
                    m += 1;
                }
                month = m;
                day = remaining + 1;
            }
            public static bool TryAddTicks(long baseTicks, long delta, out long result) {
                result = 0L;
                var candidate = baseTicks + delta;
                if (candidate <DateTimeConstants.MinTicks || candidate >DateTimeConstants.MaxTicks)
                {
                    return false;
                }
                result = candidate;
                return true;
            }
            public static void GetTimeParts(long ticksOfDay, out int hour, out int minute, out int second, out long fractionTicks) {
                var remaining = ticksOfDay;
                hour = NumericUnchecked.ToInt32(remaining / DateTimeConstants.TicksPerHour);
                remaining -= (long) hour * DateTimeConstants.TicksPerHour;
                minute = NumericUnchecked.ToInt32(remaining / DateTimeConstants.TicksPerMinute);
                remaining -= (long) minute * DateTimeConstants.TicksPerMinute;
                second = NumericUnchecked.ToInt32(remaining / DateTimeConstants.TicksPerSecond);
                remaining -= (long) second * DateTimeConstants.TicksPerSecond;
                fractionTicks = remaining;
            }
        }
