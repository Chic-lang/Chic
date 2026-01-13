namespace Std.Numeric;
internal static class CharRuntimeIntrinsics
{
    @extern("C") public static extern int chic_rt_char_is_scalar(char value);
    @extern("C") public static extern int chic_rt_char_is_digit(char value);
    @extern("C") public static extern int chic_rt_char_is_letter(char value);
    @extern("C") public static extern int chic_rt_char_is_whitespace(char value);
    @extern("C") public static extern ulong chic_rt_char_to_upper(char value);
    @extern("C") public static extern ulong chic_rt_char_to_lower(char value);
    @extern("C") public static extern ulong chic_rt_char_from_codepoint(uint value);
    @extern("C") public static extern int chic_rt_char_status(ulong value);
    @extern("C") public static extern char chic_rt_char_value(ulong value);
    @extern("C") public static extern string chic_rt_string_from_char(char value);
}
