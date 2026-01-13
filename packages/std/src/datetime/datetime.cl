namespace Std.Datetime;
import Std.Numeric;
import Std.Globalization;
public struct DateTime : IConvertible
{
    private readonly long _ticks;
    private readonly DateTimeKind _kind;
    private readonly int _offsetMinutes;
    private readonly string _zoneName;
    private init(long ticks, DateTimeKind kind, int offsetMinutes, string zoneName) {
        _ticks = ticks;
        _kind = kind;
        _offsetMinutes = offsetMinutes;
        _zoneName = zoneName;
    }
    public static DateTime FromParts(Date date, Std.Datetime.Time time, DateTimeKind kind) {
        var ticks = date.TicksAtMidnight + time.TicksOfDay;
        return FromTicks(ticks, kind);
    }
    public static DateTime FromTicks(long ticks, DateTimeKind kind) {
        if (ticks <DateTimeConstants.MinTicks || ticks >DateTimeConstants.MaxTicks)
        {
            throw new Std.ArgumentOutOfRangeException("ticks");
        }
        return new DateTime(ticks, kind, 0, null);
    }
    public long Ticks => _ticks;
    public DateTimeKind Kind => _kind;
    public int OffsetMinutes => _offsetMinutes;
    public string ZoneName => _zoneName;
    public Date Date {
        get {
            var year;
            var month;
            var day;
            DateTimeUtil.GetDateParts(_ticks, out year, out month, out day);
            var midnight = (_ticks / DateTimeConstants.TicksPerDay) * DateTimeConstants.TicksPerDay;
            return Date.FromTicks(midnight, year, month, day);
        }
    }
    public Std.Datetime.Time TimeOfDay {
        get {
            var ticksOfDay = _ticks % DateTimeConstants.TicksPerDay;
            return Std.Datetime.Time.FromTicksOfDay(ticksOfDay);
        }
    }
    public DateTime SpecifyKind(DateTimeKind kind) {
        return new DateTime(_ticks, kind, _offsetMinutes, _zoneName);
    }
    public DateTime WithOffset(int minutes, string zoneName) {
        return new DateTime(_ticks, DateTimeKind.Local, minutes, zoneName);
    }
    private long ToUtcTicks() {
        if (_kind == DateTimeKind.Utc)
        {
            return _ticks;
        }
        if (_kind == DateTimeKind.Local)
        {
            return _ticks - (long) _offsetMinutes * DateTimeConstants.TicksPerMinute;
        }
        return _ticks;
    }
    public DateTime ToUniversalTime() {
        if (_kind == DateTimeKind.Utc)
        {
            return this;
        }
        var utcTicks = ToUtcTicks();
        return new DateTime(utcTicks, DateTimeKind.Utc, 0, _zoneName);
    }
    public DateTime ToLocalTime(string zone) {
        var offset = TimeZones.ResolveOffset(zone, this);
        var adjustedTicks;
        if (! DateTimeUtil.TryAddTicks (_ticks, (long) offset.TotalOffsetMinutes * DateTimeConstants.TicksPerMinute, out adjustedTicks)) {
            throw new Std.OverflowException();
        }
        return new DateTime(adjustedTicks, DateTimeKind.Local, offset.TotalOffsetMinutes, zone);
    }
    public DateTime Add(Duration duration) {
        var result;
        if (! DateTimeUtil.TryAddTicks (_ticks, duration.Ticks, out result)) {
            throw new Std.OverflowException();
        }
        return new DateTime(result, _kind, _offsetMinutes, _zoneName);
    }
    public DateTime AddDays(int days) {
        var delta = (long) days * DateTimeConstants.TicksPerDay;
        var result;
        if (! DateTimeUtil.TryAddTicks (_ticks, delta, out result)) {
            throw new Std.OverflowException();
        }
        return new DateTime(result, _kind, _offsetMinutes, _zoneName);
    }
    public DateTime AddSeconds(int seconds) {
        var delta = (long) seconds * DateTimeConstants.TicksPerSecond;
        var result;
        if (! DateTimeUtil.TryAddTicks (_ticks, delta, out result)) {
            throw new Std.OverflowException();
        }
        return new DateTime(result, _kind, _offsetMinutes, _zoneName);
    }
    public Duration Subtract(DateTime other) {
        var utcSelf = ToUtcTicks();
        var utcOther = other.ToUtcTicks();
        return Duration.FromTicks(utcSelf - utcOther);
    }
    public DateTime Subtract(Duration duration) {
        var result;
        if (! DateTimeUtil.TryAddTicks (_ticks, - duration.Ticks, out result)) {
            throw new Std.OverflowException();
        }
        return new DateTime(result, _kind, _offsetMinutes, _zoneName);
    }
    public static int Compare(DateTime left, DateTime right) {
        var leftTicks = left._kind == DateTimeKind.Unspecified ?left._ticks : left.ToUtcTicks();
        var rightTicks = right._kind == DateTimeKind.Unspecified ?right._ticks : right.ToUtcTicks();
        if (leftTicks <rightTicks)
        {
            return - 1;
        }
        if (leftTicks >rightTicks)
        {
            return 1;
        }
        return 0;
    }
    public bool Equals(DateTime other) {
        return Compare(this, other) == 0;
    }
    public override string ToString() {
        return DateTimeFormatting.FormatDateTime(this, null, InvariantDateTimeCulture.Instance);
    }
    public string ToString(IFormatProvider provider) => DateTimeFormatting.FormatDateTime(this, null, DateTimeCultures.Resolve(ConvertibleHelpers.ResolveCulture(provider)));
    public string ToString(string format, IDateTimeCulture culture) {
        return DateTimeFormatting.FormatDateTime(this, format, culture);
    }
    public string ToString(string format) {
        return DateTimeFormatting.FormatDateTime(this, format, InvariantDateTimeCulture.Instance);
    }
    public bool ToBoolean(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Boolean");
    public char ToChar(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Char");
    public sbyte ToSByte(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "SByte");
    public byte ToByte(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Byte");
    public short ToInt16(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Int16");
    public ushort ToUInt16(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "UInt16");
    public int ToInt32(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Int32");
    public uint ToUInt32(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "UInt32");
    public long ToInt64(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Int64");
    public ulong ToUInt64(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "UInt64");
    public nint ToNInt(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "nint");
    public nuint ToNUInt(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "nuint");
    public isize ToISize(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "isize");
    public usize ToUSize(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "usize");
    public Int128 ToInt128(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Int128");
    public UInt128 ToUInt128(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "UInt128");
    public float ToSingle(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Float32");
    public double ToDouble(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Float64");
    public Float128 ToFloat128(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Float128");
    public Decimal ToDecimal(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("DateTime", "Decimal");
    public DateTime ToDateTime(IFormatProvider provider) => this;
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("DateTime");
}
