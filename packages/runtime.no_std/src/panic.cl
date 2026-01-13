#![no_std]
namespace Std.Runtime.NoStd;
/// Minimal panic/abort shims for `#![no_std]` crates.
/// These functions intentionally avoid platform calls and never return.
public static class PanicHandlers
{
    internal static bool TestEnabled = false;
    internal static uint TestSpinCount = 0u;
    @export("chic_rt_panic") public static int Panic(int code) {
        return Halt(code);
    }
    @export("chic_rt_abort") public static int Abort(int code) {
        return Halt(code);
    }
    private static int Halt(int code) {
        // Spin forever so control never returns to the caller.
        var spins = TestSpinCount;
        while (true)
        {
            if (TestEnabled)
            {
                if (spins == 0u)
                {
                    break;
                }
                spins -= 1u;
                if (spins == 0u)
                {
                    break;
                }
            }
        }
        // Unreachable, but keep the signature satisfied.
        return code;
    }
}
