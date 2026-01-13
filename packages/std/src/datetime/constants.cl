namespace Std.Datetime;
internal static class DateTimeConstants
{
    public const long TicksPerMillisecond = 10_000L;
    public const long TicksPerSecond = TicksPerMillisecond * 1000L;
    public const long TicksPerMinute = TicksPerSecond * 60L;
    public const long TicksPerHour = TicksPerMinute * 60L;
    public const long TicksPerDay = TicksPerHour * 24L;
    // Range mirrors .NET: 0001-01-01 through 9999-12-31.
    public const long MaxTicks = 3155378975999999999L;
    public const long MinTicks = 0L;
}
