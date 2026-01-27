namespace Std.Datetime;
import Std.Numeric;
public struct Instant
{
    private readonly long _unixTicks;
    private init(long unixTicks) {
        _unixTicks = unixTicks;
    }
    public static Instant FromUnixTicks(long unixTicks) {
        return new Instant(unixTicks);
    }
    public static Instant NowUtc() {
        var nanos = Std.Platform.Time.UtcNanoseconds();
        var ticks = NumericUnchecked.ToInt64(nanos / 100ul);
        return new Instant(ticks);
    }
    public static Instant Monotonic() {
        var nanos = Std.Platform.Time.MonotonicNanoseconds();
        var ticks = NumericUnchecked.ToInt64(nanos / 100ul);
        return new Instant(ticks);
    }
    public long UnixTicks => _unixTicks;
    public Duration Since(Instant earlier) {
        return Duration.FromTicks(_unixTicks - earlier._unixTicks);
    }
    public Instant Add(Duration duration) {
        return new Instant(_unixTicks + duration.Ticks);
    }
    public DateTime ToDateTimeUtc() {
        // Unix epoch start ticks relative to 0001-01-01
        var epochTicks = 621355968000000000L;
        var absolute = epochTicks + _unixTicks;
        return DateTime.FromTicks(absolute, DateTimeKind.Utc);
    }
}
