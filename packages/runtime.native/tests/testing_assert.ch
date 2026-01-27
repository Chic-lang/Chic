namespace Std.Runtime.Native.Testing;
import Std.Runtime.Native;
/// <summary>
/// Entry point for fluent assertions in runtime.native testcases.
/// </summary>
public static class Assert
{
    internal static void Fail() {
        // Runtime.native test harness treats any pending exception as a failing testcase.
        PendingExceptionRuntime.chic_rt_throw(0, 1);
    }
    public static BoolAssertionContext That(bool value) {
        return new BoolAssertionContext(value);
    }
    public static IntAssertionContext That(int value) {
        return new IntAssertionContext(value);
    }
    public static UIntAssertionContext That(uint value) {
        return new UIntAssertionContext(value);
    }
    public static LongAssertionContext That(long value) {
        return new LongAssertionContext(value);
    }
    public static ULongAssertionContext That(ulong value) {
        return new ULongAssertionContext(value);
    }
    public static USizeAssertionContext That(usize value) {
        return new USizeAssertionContext(value);
    }
}
