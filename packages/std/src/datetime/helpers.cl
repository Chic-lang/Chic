namespace Std.Datetime;
public static class DateTimeHelpers
{
    public static bool TryParseIso(string text, out DateTime value) {
        return DateTimeParsing.TryParseIso(text, out value);
    }
    public static bool TryParseRfc3339(string text, out DateTime value) {
        return DateTimeParsing.TryParseRfc3339(text, out value);
    }
    public static bool TryParseCustom(string format, string text, out DateTime value) {
        return DateTimeParsing.TryParseCustom(format, text, out value);
    }
}
