namespace Std.Datetime;
import Std.Strings;
import Std.Numeric;
import Std.Span;
import Std.Core;
internal struct ParsedDateTimeParts
{
    public int Year;
    public int Month;
    public int Day;
    public int Hour;
    public int Minute;
    public int Second;
    public int FractionTicks;
    public DateTimeKind Kind;
    public int OffsetMinutes;
}
internal static class DateTimeParsing
{
    private const byte ASCII_ZERO = 0x30u8;
    private const byte ASCII_NINE = 0x39u8;
    private const byte ASCII_DASH = 0x2Du8;
    private const byte ASCII_PLUS = 0x2Bu8;
    private const byte ASCII_COLON = 0x3Au8;
    private const byte ASCII_DOT = 0x2Eu8;
    private const byte ASCII_T = 0x54u8;
    private const byte ASCII_Z = 0x5Au8;
    private static bool TryParseDigits(ReadOnlySpan <byte >text, usize start, usize count, out int value) {
        value = 0;
        if (text.Length <start + count)
        {
            return false;
        }
        for (var i = 0usize; i <count; i += 1) {
            let b = text[start + i];
            if (b <ASCII_ZERO || b >ASCII_NINE)
            {
                return false;
            }
            value = value * 10 + (int)(b - ASCII_ZERO);
        }
        return true;
    }
    private static bool TryParseOffset(ReadOnlySpan <byte >text, usize index, out int offsetMinutes, out usize consumed) {
        offsetMinutes = 0;
        consumed = 0;
        if (index >= text.Length)
        {
            return false;
        }
        let head = text[index];
        if (head == ASCII_Z)
        {
            consumed = 1;
            offsetMinutes = 0;
            return true;
        }
        if (head != ASCII_PLUS && head != ASCII_DASH)
        {
            return false;
        }
        let sign = head == ASCII_DASH ?- 1 : 1;
        var hours = 0;
        var minutes = 0;
        if (!TryParseDigits (text, index + 1, 2, out hours)) {
            return false;
        }
        if (index + 3 >= text.Length || text[index + 3] != ASCII_COLON)
        {
            return false;
        }
        if (!TryParseDigits (text, index + 4, 2, out minutes)) {
            return false;
        }
        consumed = 6;
        offsetMinutes = sign * (hours * 60 + minutes);
        return true;
    }
    private static bool TryFinalize(ParsedDateTimeParts parts, out DateTime result) {
        result = DateTime.FromTicks(DateTimeConstants.MinTicks, DateTimeKind.Unspecified);
        if (!DateTimeUtil.TryDateToTicks (parts.Year, parts.Month, parts.Day, out var dateTicks)) {
            return false;
        }
        let nanos = parts.FractionTicks * 100;
        if (!DateTimeUtil.TryTimeToTicks (parts.Hour, parts.Minute, parts.Second, nanos, out var timeTicks)) {
            return false;
        }
        let total = dateTicks + timeTicks;
        if (total <DateTimeConstants.MinTicks || total >DateTimeConstants.MaxTicks)
        {
            return false;
        }
        var dt = DateTime.FromTicks(total, parts.Kind);
        if (parts.Kind == DateTimeKind.Local)
        {
            dt = dt.WithOffset(parts.OffsetMinutes, null);
        }
        result = dt;
        return true;
    }
    public static bool TryParseIso(string text, out DateTime result) {
        let span = Std.Span.ReadOnlySpan.FromString(text);
        result = DateTime.FromTicks(DateTimeConstants.MinTicks, DateTimeKind.Unspecified);
        if (span.Length <19)
        {
            return false;
        }
        var parts = CoreIntrinsics.DefaultValue <ParsedDateTimeParts >();
        if (!TryParseDigits (span, 0, 4, out parts.Year)) {
            return false;
        }
        if (span[4] != ASCII_DASH || !TryParseDigits (span, 5, 2, out parts.Month)) {
            return false;
        }
        if (span[7] != ASCII_DASH || !TryParseDigits (span, 8, 2, out parts.Day)) {
            return false;
        }
        if (span[10] != ASCII_T || !TryParseDigits (span, 11, 2, out parts.Hour)) {
            return false;
        }
        if (span[13] != ASCII_COLON || !TryParseDigits (span, 14, 2, out parts.Minute)) {
            return false;
        }
        if (span[16] != ASCII_COLON || !TryParseDigits (span, 17, 2, out parts.Second)) {
            return false;
        }
        var cursor = 19usize;
        if (cursor <span.Length && span[cursor] == ASCII_DOT)
        {
            cursor += 1;
            var digits = 0;
            var fraction = 0;
            while (cursor <span.Length && digits <7)
            {
                let b = span[cursor];
                if (b <ASCII_ZERO || b >ASCII_NINE)
                {
                    break;
                }
                fraction = fraction * 10 + (int)(b - ASCII_ZERO);
                digits += 1;
                cursor += 1;
            }
            while (digits <7)
            {
                fraction *= 10;
                digits += 1;
            }
            parts.FractionTicks = fraction;
        }
        if (cursor <span.Length)
        {
            if (!TryParseOffset (span, cursor, out var offset, out var consumed)) {
                return false;
            }
            if (span[cursor] == ASCII_Z)
            {
                parts.Kind = DateTimeKind.Utc;
                cursor += consumed;
            }
            else
            {
                parts.Kind = DateTimeKind.Local;
                parts.OffsetMinutes = offset;
                cursor += consumed;
            }
        }
        return TryFinalize(parts, out result);
    }
    public static bool TryParseRfc3339(string text, out DateTime result) {
        // RFC 3339 is a profile of ISO-8601; reuse ISO parser.
        return TryParseIso(text, out result);
    }
    public static bool TryParseCustom(string format, string text, out DateTime result) {
        result = DateTime.FromTicks(DateTimeConstants.MinTicks, DateTimeKind.Unspecified);
        let span = Std.Span.ReadOnlySpan.FromString(text);
        var parts = CoreIntrinsics.DefaultValue <ParsedDateTimeParts >();
        parts.Year = 1;
        parts.Month = 1;
        parts.Day = 1;
        parts.Hour = 0;
        parts.Minute = 0;
        parts.Second = 0;
        parts.FractionTicks = 0;
        parts.Kind = DateTimeKind.Unspecified;
        parts.OffsetMinutes = 0;
        var fi = 0usize;
        var ti = 0usize;
        while (fi <format.Length)
        {
            let f = format[fi];
            var run = 1usize;
            while (fi + run <format.Length && format[fi + run] == f)
            {
                run += 1;
            }
            if (f == 'y')
            {
                if (!TryParseDigits (span, ti, run == 2 ?2 : 4, out var value)) {
                    return false;
                }
                parts.Year = value;
                ti += run == 2 ?2 : 4;
            }
            else if (f == 'M')
            {
                if (!TryParseDigits (span, ti, run == 2 ?2 : 1, out var value)) {
                    return false;
                }
                parts.Month = value;
                ti += run == 2 ?2 : 1;
            }
            else if (f == 'd')
            {
                if (!TryParseDigits (span, ti, run == 2 ?2 : 1, out var value)) {
                    return false;
                }
                parts.Day = value;
                ti += run == 2 ?2 : 1;
            }
            else if (f == 'H')
            {
                if (!TryParseDigits (span, ti, 2, out var value)) {
                    return false;
                }
                parts.Hour = value;
                ti += 2;
            }
            else if (f == 'm')
            {
                if (!TryParseDigits (span, ti, 2, out var value)) {
                    return false;
                }
                parts.Minute = value;
                ti += 2;
            }
            else if (f == 's')
            {
                if (!TryParseDigits (span, ti, 2, out var value)) {
                    return false;
                }
                parts.Second = value;
                ti += 2;
            }
            else if (f == 'f' || f == 'F')
            {
                var digits = NumericUnchecked.ToInt32(run);
                var accum = 0;
                for (var i = 0; i <digits; i += 1) {
                    if (ti + (usize) i >= span.Length)
                    {
                        return false;
                    }
                    let b = span[ti + (usize) i];
                    if (b <ASCII_ZERO || b >ASCII_NINE)
                    {
                        if (f == 'F')
                        {
                            digits = i;
                            break;
                        }
                        return false;
                    }
                    accum = accum * 10 + (int)(b - ASCII_ZERO);
                }
                while (digits <7)
                {
                    accum *= 10;
                    digits += 1;
                }
                parts.FractionTicks = accum;
                ti += run;
            }
            else if (f == 'K' || f == 'z')
            {
                var offset = 0;
                var consumed = 0usize;
                if (!TryParseOffset (span, ti, out offset, out consumed)) {
                    return false;
                }
                if (span[ti] == ASCII_Z)
                {
                    parts.Kind = DateTimeKind.Utc;
                }
                else
                {
                    parts.Kind = DateTimeKind.Local;
                    parts.OffsetMinutes = offset;
                }
                ti += consumed;
            }
            else
            {
                for (var i = 0usize; i <run; i += 1) {
                    let expected = NumericUnchecked.ToByte((int) f);
                    if (ti >= span.Length || span[ti] != expected)
                    {
                        return false;
                    }
                    ti += 1;
                }
            }
            fi += run;
        }
        return TryFinalize(parts, out result);
    }
}
