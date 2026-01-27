namespace Std.Runtime.Native;
// Minimal character classification and casing helpers exported for the Chic runtime.
// These mirror the Rust runtime surface and keep the status packing identical:
// upper 32 bits carry `CharError`, lower 16 bits carry the resulting scalar.
@export("chic_rt_char_is_scalar") public static int chic_rt_char_is_scalar(ushort value) {
    return IsScalar(value) ?1 : 0;
}
@export("chic_rt_char_is_digit") public static int chic_rt_char_is_digit(ushort value) {
    if (! IsScalar (value))
    {
        return - 1;
    }
    return(value >= (ushort) '0' && value <= (ushort) '9') ?1 : 0;
}
@export("chic_rt_char_is_letter") public static int chic_rt_char_is_letter(ushort value) {
    if (! IsScalar (value))
    {
        return - 1;
    }
    let isUpper = value >= (ushort) 'A' && value <= (ushort) 'Z';
    let isLower = value >= (ushort) 'a' && value <= (ushort) 'z';
    return(isUpper || isLower) ?1 : 0;
}
@export("chic_rt_char_is_whitespace") public static int chic_rt_char_is_whitespace(ushort value) {
    if (! IsScalar (value))
    {
        return - 1;
    }
    switch (value)
    {
        case 0x0009:
            // \t
        case 0x000A:
            // \n
        case 0x000B:
            // \v
        case 0x000C:
            // \f
        case 0x000D:
            // \r
        case 0x0020:
            // space
            return 1;
        default :
            return 0;
        }
    }
    @export("chic_rt_char_to_upper") public static ulong chic_rt_char_to_upper(ushort value) {
        if (! IsScalar (value))
        {
            return Pack(CharError.InvalidScalar, 0);
        }
        if (value >= (ushort) 'a' && value <= (ushort) 'z')
        {
            return Pack(CharError.Success, (ushort)(value - 32));
        }
        return Pack(CharError.Success, value);
    }
    @export("chic_rt_char_to_lower") public static ulong chic_rt_char_to_lower(ushort value) {
        if (! IsScalar (value))
        {
            return Pack(CharError.InvalidScalar, 0);
        }
        if (value >= (ushort) 'A' && value <= (ushort) 'Z')
        {
            return Pack(CharError.Success, (ushort)(value + 32));
        }
        return Pack(CharError.Success, value);
    }
    @export("chic_rt_char_from_codepoint") public static ulong chic_rt_char_from_codepoint(uint value) {
        if (! IsScalar (value))
        {
            return Pack(CharError.InvalidScalar, 0);
        }
        return Pack(CharError.Success, (ushort) value);
    }
    @export("chic_rt_char_status") public static int chic_rt_char_status(ulong packed) {
        return(int)(packed >> 32);
    }
    @export("chic_rt_char_value") public static ushort chic_rt_char_value(ulong packed) {
        return(ushort)(packed & 0xFFFF);
    }
    private static bool IsScalar(uint value) {
        if (value >0xFFFF)
        {
            return false;
        }
        return value <0xD800 || value >0xDFFF;
    }
    internal enum CharError : int
    {
        Success = 0, InvalidScalar = 1, NullPointer = 2, ComplexMapping = 3,
    }
    private static ulong Pack(CharError status, ushort value) {
        return((ulong)(uint) status << 32) | (ulong) value;
    }
