namespace Std.Runtime.Native;
// Minimal floating environment tracking for the native runtime. The Chic implementation
// mirrors the Rust `float_env` facade: rounding mode setters/getters and sticky IEEE flags.
public enum RoundingMode
{
    NearestTiesToEven = 0, NearestTiesToAway = 1, TowardZero = 2, TowardPositive = 3, TowardNegative = 4,
}
public struct FloatFlags
{
    public bool Invalid;
    public bool DivideByZero;
    public bool Overflow;
    public bool Underflow;
    public bool Inexact;
    public bool Any() {
        return Invalid || DivideByZero || Overflow || Underflow || Inexact;
    }
}
internal static class FloatEnv
{
    private static int _mode = 0;
    private static bool _invalid;
    private static bool _divideByZero;
    private static bool _overflow;
    private static bool _underflow;
    private static bool _inexact;
    public static int Mode => _mode;
    public static void SetMode(int mode) {
        _mode = mode;
    }
    public static FloatFlags Flags {
        get {
            return new FloatFlags {
                Invalid = _invalid, DivideByZero = _divideByZero, Overflow = _overflow, Underflow = _underflow, Inexact = _inexact,
            }
            ;
        }
    }
    public static void Record(FloatFlags flags) {
        if (! flags.Any ())
        {
            return;
        }
        _invalid = _invalid || flags.Invalid;
        _divideByZero = _divideByZero || flags.DivideByZero;
        _overflow = _overflow || flags.Overflow;
        _underflow = _underflow || flags.Underflow;
        _inexact = _inexact || flags.Inexact;
    }
    public static void Clear() {
        _invalid = false;
        _divideByZero = false;
        _overflow = false;
        _underflow = false;
        _inexact = false;
    }
}
@extern("C") @export("chic_rt_float_flags_read") public static uint chic_rt_float_flags_read() {
    let flags = FloatEnv.Flags;
    var mask = 0u;
    if (flags.Invalid) mask = mask | 0x1u;
    if (flags.DivideByZero) mask = mask | 0x2u;
    if (flags.Overflow) mask = mask | 0x4u;
    if (flags.Underflow) mask = mask | 0x8u;
    if (flags.Inexact) mask = mask | 0x10u;
    return mask;
}
@extern("C") @export("chic_rt_float_flags_clear") public static void chic_rt_float_flags_clear() {
    FloatEnv.Clear();
}
@extern("C") @export("chic_rt_set_rounding_mode") public static int chic_rt_set_rounding_mode(int mode) {
    if (mode <0 || mode >4)
    {
        return - 1;
    }
    FloatEnv.SetMode(mode);
    return 0;
}
@extern("C") @export("chic_rt_rounding_mode") public static int chic_rt_rounding_mode() {
    return FloatEnv.Mode;
}
