namespace Std.Core;
/// <summary>
/// Minimal panic surface for core/no_std consumers that does not depend on the
/// higher-level startup/diagnostics layers.
/// </summary>
public static class CorePanic
{
    @extern("C") private static extern int chic_rt_panic(int code);
    @extern("C") private static extern int chic_rt_abort(int code);
    public static int Panic(int code) {
        return chic_rt_panic(code);
    }
    public static int Abort(int code) {
        return chic_rt_abort(code);
    }
}
