namespace Std.Platform;
import Std.Numeric;
import Std.Core;
@repr(c) public struct Timespec
{
    public long Seconds;
    public long Nanos;
}
public static class Time
{
    private const int CLOCK_REALTIME = 0;
    private const int CLOCK_MONOTONIC = 1;
    @extern("C") private static extern int clock_gettime(int clockId, out Timespec ts);
    @extern("C") private static extern int nanosleep(out Timespec req, out Timespec rem);
    public static ulong MonotonicNanoseconds() {
        var ts = CoreIntrinsics.DefaultValue <Timespec >();
        let status = clock_gettime(CLOCK_MONOTONIC, out ts);
        if (status != 0)
        {
            throw new Std.InvalidOperationException("failed to read monotonic clock");
        }
        let secs = NumericUnchecked.ToUInt64(ts.Seconds);
        let nanos = NumericUnchecked.ToUInt64(ts.Nanos);
        return secs * 1_000_000_000UL + nanos;
    }
    public static ulong UtcNanoseconds() {
        var ts = CoreIntrinsics.DefaultValue <Timespec >();
        let status = clock_gettime(CLOCK_REALTIME, out ts);
        if (status != 0)
        {
            throw new Std.InvalidOperationException("failed to read realtime clock");
        }
        let secs = NumericUnchecked.ToUInt64(ts.Seconds);
        let nanos = NumericUnchecked.ToUInt64(ts.Nanos);
        return secs * 1_000_000_000UL + nanos;
    }
    public static void SleepMillis(ulong millis) {
        var req = CoreIntrinsics.DefaultValue <Timespec >();
        req.Seconds = NumericUnchecked.ToInt64(millis / 1000UL);
        req.Nanos = NumericUnchecked.ToInt64((millis % 1000UL) * 1_000_000UL);
        var rem = CoreIntrinsics.DefaultValue <Timespec >();
        let status = nanosleep(out req, out rem);
        if (status != 0)
        {
            throw new Std.InvalidOperationException("sleep was interrupted");
        }
    }
}
