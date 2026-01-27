namespace Std.Numeric;
import Std.Strings;
internal enum ParseStatus
{
    Success = 0, Invalid = 1, Overflow = 2,
}
internal static class NumericParse
{
    private const int DecimalMaxScale = 28;
    internal static void ThrowParseException(ParseStatus status, string typeName) {
        if (status == ParseStatus.Overflow)
        {
            throw new Std.OverflowException(typeName + " value was outside the valid range");
        }
        throw new Std.FormatException(typeName + " text was not in a recognised format");
    }
    private static ReadOnlySpan <byte >Trim(ReadOnlySpan <byte >value) {
        let length = value.Length;
        if (length == 0)
        {
            return value;
        }
        var start = 0usize;
        while (start <length && IsWhitespace (value[start]))
        {
            start += 1;
        }
        var end = length;
        while (end >start && IsWhitespace (value[end - 1]))
        {
            end -= 1;
        }
        let trimmedLength = end - start;
        return value.Slice(start, trimmedLength);
    }
    private static ParseStatus TryParseIntegerCore(ReadOnlySpan <byte >text, bool allowNegative, bool allowPositiveSign,
    out bool negative, out ulong magnitude, ulong positiveLimit, ulong negativeLimit) {
        negative = false;
        magnitude = 0ul;
        var working = Trim(text);
        if (working.IsEmpty)
        {
            return ParseStatus.Invalid;
        }
        if (!working.IsEmpty)
        {
            if (working[0] == NumericUnchecked.ToByte ('+'))
            {
                if (!allowPositiveSign)
                {
                    return ParseStatus.Invalid;
                }
                working = working.Slice(1);
            }
            else if (working[0] == NumericUnchecked.ToByte ('-'))
            {
                if (!allowNegative)
                {
                    return ParseStatus.Invalid;
                }
                negative = true;
                working = working.Slice(1);
            }
        }
        if (working.IsEmpty)
        {
            return ParseStatus.Invalid;
        }
        let limitValue = negative ?negativeLimit : positiveLimit;
        let limitDiv10 = limitValue / 10ul;
        let limitMod10 = limitValue % 10ul;
        var seenDigit = false;
        var prevUnderscore = false;
        var remaining = working;
        while (!remaining.IsEmpty)
        {
            let head = remaining[0];
            if (head == NumericUnchecked.ToByte ('_'))
            {
                if (!seenDigit || prevUnderscore)
                {
                    magnitude = 0ul;
                    return ParseStatus.Invalid;
                }
                prevUnderscore = true;
                remaining = remaining.Slice(1);
                continue;
            }
            if (TryParseDecimalDigit (head, out var digit)) {
                if (magnitude >limitDiv10 || (magnitude == limitDiv10 && digit >limitMod10))
                {
                    magnitude = 0ul;
                    return ParseStatus.Overflow;
                }
                magnitude = (magnitude * 10ul) + digit;
                seenDigit = true;
                prevUnderscore = false;
                remaining = remaining.Slice(1);
                continue;
            }
            magnitude = 0ul;
            return ParseStatus.Invalid;
        }
        if (!seenDigit || prevUnderscore)
        {
            magnitude = 0ul;
            return ParseStatus.Invalid;
        }
        return ParseStatus.Success;
    }
    private static ParseStatus TryParseIntegerCore128(ReadOnlySpan <byte >text, bool allowNegative, bool allowPositiveSign,
    out bool negative, out u128 magnitude, u128 positiveLimit, u128 negativeLimit) {
        negative = false;
        magnitude = 0u128;
        var working = Trim(text);
        if (working.IsEmpty)
        {
            return ParseStatus.Invalid;
        }
        if (!working.IsEmpty)
        {
            if (working[0] == NumericUnchecked.ToByte ('+'))
            {
                if (!allowPositiveSign)
                {
                    return ParseStatus.Invalid;
                }
                working = working.Slice(1);
            }
            else if (working[0] == NumericUnchecked.ToByte ('-'))
            {
                if (!allowNegative)
                {
                    return ParseStatus.Invalid;
                }
                negative = true;
                working = working.Slice(1);
            }
        }
        if (working.IsEmpty)
        {
            return ParseStatus.Invalid;
        }
        let limitValue = negative ?negativeLimit : positiveLimit;
        let limitDiv10 = limitValue / 10u128;
        let limitMod10 = limitValue % 10u128;
        var seenDigit = false;
        var prevUnderscore = false;
        var remaining = working;
        while (!remaining.IsEmpty)
        {
            let head = remaining[0];
            if (head == NumericUnchecked.ToByte ('_'))
            {
                if (!seenDigit || prevUnderscore)
                {
                    magnitude = 0u128;
                    return ParseStatus.Invalid;
                }
                prevUnderscore = true;
                remaining = remaining.Slice(1);
                continue;
            }
            if (TryParseDecimalDigit (head, out var digit)) {
                let digitValue = NumericUnchecked.ToUInt128(digit);
                if (magnitude >limitDiv10 || (magnitude == limitDiv10 && digitValue >limitMod10))
                {
                    magnitude = 0u128;
                    return ParseStatus.Overflow;
                }
                magnitude = (magnitude * 10u128) + digitValue;
                seenDigit = true;
                prevUnderscore = false;
                remaining = remaining.Slice(1);
                continue;
            }
            magnitude = 0u128;
            return ParseStatus.Invalid;
        }
        if (!seenDigit || prevUnderscore)
        {
            magnitude = 0u128;
            return ParseStatus.Invalid;
        }
        return ParseStatus.Success;
    }
    private static bool IsWhitespace(byte ch) {
        return ch == NumericUnchecked.ToByte(' ') || ch == NumericUnchecked.ToByte('\t') || ch == NumericUnchecked.ToByte('\r') || ch == NumericUnchecked.ToByte('\n');
    }
    private static bool TryParseDecimalDigit(byte ch, out ulong digit) {
        if (ch >= NumericUnchecked.ToByte ('0') && ch <= NumericUnchecked.ToByte ('9'))
        {
            let digitValue = NumericUnchecked.ToUInt32(ch - NumericUnchecked.ToByte('0'));
            digit = digitValue;
            return true;
        }
        digit = 0ul;
        return false;
    }
    public static bool TryParseInt32(ReadOnlySpan <byte >text, out int value) {
        var status = ParseStatus.Invalid;
        return TryParseInt32(text, out value, out status);
    }
    public static bool TryParseInt32(ReadOnlySpan <byte >text, out int value, out ParseStatus status) {
        value = 0;
        status = TryParseIntegerCore(text, true, true, out var negative, out var magnitude, 2147483647ul, 2147483648ul);
        if (status != ParseStatus.Success)
        {
            return false;
        }
        if (negative)
        {
            if (magnitude == 0ul)
            {
                value = 0;
                return true;
            }
            if (magnitude == 2147483648ul)
            {
                value = NumericConstants.Int32Min;
                return true;
            }
            let truncated = NumericUnchecked.ToUInt32(magnitude);
            let limited = NumericUnchecked.ToInt32(truncated);
            value = - limited;
            return true;
        }
        let positive = NumericUnchecked.ToUInt32(magnitude);
        value = NumericUnchecked.ToInt32(positive);
        return true;
    }
    public static bool TryParseInt64(ReadOnlySpan <byte >text, out long value) {
        var status = ParseStatus.Invalid;
        return TryParseInt64(text, out value, out status);
    }
    public static bool TryParseInt64(ReadOnlySpan <byte >text, out long value, out ParseStatus status) {
        value = 0L;
        status = TryParseIntegerCore(text, true, true, out var negative, out var magnitude, NumericConstants.Int64Max, 9223372036854775808ul);
        if (status != ParseStatus.Success)
        {
            return false;
        }
        if (negative)
        {
            if (magnitude == 0ul)
            {
                value = 0L;
                return true;
            }
            if (magnitude == 9223372036854775808ul)
            {
                value = NumericConstants.Int64Min;
                return true;
            }
            value = - NumericUnchecked.ToInt64(magnitude);
            return true;
        }
        value = NumericUnchecked.ToInt64(magnitude);
        return true;
    }
    public static bool TryParseUInt32(ReadOnlySpan <byte >text, out uint value) {
        var status = ParseStatus.Invalid;
        return TryParseUInt32(text, out value, out status);
    }
    public static bool TryParseUInt32(ReadOnlySpan <byte >text, out uint value, out ParseStatus status) {
        value = 0u;
        status = TryParseIntegerCore(text, false, true, out var negative, out var magnitude, NumericConstants.UInt32Max,
        0ul);
        if (status != ParseStatus.Success)
        {
            return false;
        }
        value = NumericUnchecked.ToUInt32(magnitude);
        return true;
    }
    public static bool TryParseUInt64(ReadOnlySpan <byte >text, out ulong value) {
        var status = ParseStatus.Invalid;
        return TryParseUInt64(text, out value, out status);
    }
    public static bool TryParseUInt64(ReadOnlySpan <byte >text, out ulong value, out ParseStatus status) {
        value = 0ul;
        status = TryParseIntegerCore(text, false, true, out var negative, out var magnitude, NumericConstants.UInt64Max,
        0ul);
        if (status != ParseStatus.Success)
        {
            return false;
        }
        value = magnitude;
        return true;
    }
    public static bool TryParseInt128(ReadOnlySpan <byte >text, out i128 value) {
        var status = ParseStatus.Invalid;
        return TryParseInt128(text, out value, out status);
    }
    public static bool TryParseInt128(ReadOnlySpan <byte >text, out i128 value, out ParseStatus status) {
        value = 0i128;
        status = TryParseIntegerCore128(text, true, true, out var negative, out var magnitude, NumericUnchecked.ToUInt128(NumericConstants.Int128Max),
        NumericConstants.Int128MinMagnitude);
        if (status != ParseStatus.Success)
        {
            return false;
        }
        if (negative)
        {
            if (magnitude == NumericConstants.Int128MinMagnitude)
            {
                value = NumericConstants.Int128Min;
                return true;
            }
            value = - NumericUnchecked.ToInt128(magnitude);
            return true;
        }
        value = NumericUnchecked.ToInt128(magnitude);
        return true;
    }
    public static bool TryParseInt128(string text, out i128 value) {
        if (text == null)
        {
            value = 0i128;
            return false;
        }
        var status = ParseStatus.Invalid;
        return TryParseInt128(Std.Span.ReadOnlySpan.FromString(text), out value, out status);
    }
    public static bool TryParseUInt128(ReadOnlySpan <byte >text, out u128 value) {
        var status = ParseStatus.Invalid;
        return TryParseUInt128(text, out value, out status);
    }
    public static bool TryParseUInt128(ReadOnlySpan <byte >text, out u128 value, out ParseStatus status) {
        value = 0u128;
        status = TryParseIntegerCore128(text, false, true, out var sign, out var magnitude, NumericConstants.UInt128Max,
        0u128);
        if (status != ParseStatus.Success)
        {
            return false;
        }
        value = magnitude;
        return true;
    }
    public static bool TryParseUInt128(string text, out u128 value) {
        if (text == null)
        {
            value = 0u128;
            return false;
        }
        var status = ParseStatus.Invalid;
        return TryParseUInt128(Std.Span.ReadOnlySpan.FromString(text), out value, out status);
    }
    public static bool TryParseSByte(ReadOnlySpan <byte >text, out sbyte value) {
        if (!TryParseInt32 (text, out var parsed)) {
            value = 0;
            return false;
        }
        if (parsed <NumericConstants.SByteMin || parsed >NumericConstants.SByteMax)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToSByte(parsed);
        return true;
    }
    public static bool TryParseByte(ReadOnlySpan <byte >text, out byte value) {
        if (!TryParseUInt32 (text, out var parsed)) {
            value = 0u8;
            return false;
        }
        if (parsed >0xFFu)
        {
            value = 0u8;
            return false;
        }
        value = NumericUnchecked.ToByte(parsed);
        return true;
    }
    public static bool TryParseInt16(ReadOnlySpan <byte >text, out short value) {
        if (!TryParseInt32 (text, out var parsed)) {
            value = 0;
            return false;
        }
        if (parsed <NumericConstants.Int16Min || parsed >NumericConstants.Int16Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt16(parsed);
        return true;
    }
    public static bool TryParseUInt16(ReadOnlySpan <byte >text, out ushort value) {
        if (!TryParseUInt32 (text, out var parsed)) {
            value = 0u16;
            return false;
        }
        if (parsed >NumericConstants.UInt16Max)
        {
            value = 0u16;
            return false;
        }
        value = NumericUnchecked.ToUInt16(parsed);
        return true;
    }
    public static bool TryParseIntPtr(ReadOnlySpan <byte >text, out nint value) {
        var status = ParseStatus.Invalid;
        return TryParseIntPtr(text, out value, out status);
    }
    public static bool TryParseIntPtr(ReadOnlySpan <byte >text, out nint value, out ParseStatus status) {
        if (NumericPlatform.PointerBits == 32u)
        {
            if (!TryParseInt32 (text, out var parsed32, out status)) {
                value = NumericUnchecked.ToNintNarrow(0u);
                return false;
            }
            value = NumericUnchecked.ToNintFromInt32(parsed32);
            status = ParseStatus.Success;
            return true;
        }
        if (!TryParseInt64 (text, out var parsed, out status)) {
            value = NumericUnchecked.ToNintNarrow(0u);
            return false;
        }
        status = ParseStatus.Success;
        value = NumericUnchecked.ToNintFromInt64(parsed);
        return true;
    }
    public static bool TryParseUIntPtr(ReadOnlySpan <byte >text, out nuint value) {
        var status = ParseStatus.Invalid;
        return TryParseUIntPtr(text, out value, out status);
    }
    public static bool TryParseUIntPtr(ReadOnlySpan <byte >text, out nuint value, out ParseStatus status) {
        if (NumericPlatform.PointerBits == 32u)
        {
            if (!TryParseUInt32 (text, out var parsed32, out status)) {
                value = NumericUnchecked.ToNuintNarrow(0u);
                return false;
            }
            value = NumericUnchecked.ToNuintNarrow(parsed32);
            status = ParseStatus.Success;
            return true;
        }
        if (!TryParseUInt64 (text, out var parsed, out status)) {
            value = NumericUnchecked.ToNuintNarrow(0u);
            return false;
        }
        status = ParseStatus.Success;
        value = NumericUnchecked.ToNuintWiden(parsed);
        return true;
    }
    public static bool TryParseInt32(string text, out int value) {
        if (text == null)
        {
            value = 0;
            return false;
        }
        return TryParseInt32(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseInt64(string text, out long value) {
        if (text == null)
        {
            value = 0L;
            return false;
        }
        return TryParseInt64(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseUInt32(string text, out uint value) {
        if (text == null)
        {
            value = 0u;
            return false;
        }
        return TryParseUInt32(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseUInt64(string text, out ulong value) {
        if (text == null)
        {
            value = 0ul;
            return false;
        }
        return TryParseUInt64(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseSByte(string text, out sbyte value) {
        if (text == null)
        {
            value = 0;
            return false;
        }
        return TryParseSByte(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseByte(string text, out byte value) {
        if (text == null)
        {
            value = 0u8;
            return false;
        }
        return TryParseByte(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseInt16(string text, out short value) {
        if (text == null)
        {
            value = 0;
            return false;
        }
        return TryParseInt16(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseUInt16(string text, out ushort value) {
        if (text == null)
        {
            value = 0u16;
            return false;
        }
        return TryParseUInt16(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseIntPtr(string text, out nint value) {
        if (text == null)
        {
            value = NumericUnchecked.ToNintNarrow(0u);
            return false;
        }
        return TryParseIntPtr(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseUIntPtr(string text, out nuint value) {
        if (text == null)
        {
            value = NumericUnchecked.ToNuintNarrow(0u);
            return false;
        }
        return TryParseUIntPtr(Std.Span.ReadOnlySpan.FromString(text), out value);
    }
    public static bool TryParseDecimal(string text, out decimal value) {
        if (text == null)
        {
            value = 0m;
            return false;
        }
        return TryParseDecimal(Std.Span.ReadOnlySpan.FromString(text), NumericUnchecked.ToByte('.'), out value);
    }
    public static bool TryParseDecimal(string text, string culture, out decimal value) {
        if (text == null)
        {
            value = 0m;
            return false;
        }
        let data = NumericCultureInfo.Resolve(culture);
        return TryParseDecimal(Std.Span.ReadOnlySpan.FromString(text), data.DecimalSeparator, out value);
    }
    public static bool TryParseDecimal(ReadOnlySpan <byte >text, out decimal value) {
        var status = ParseStatus.Invalid;
        return TryParseDecimal(text, NumericUnchecked.ToByte('.'), out value, out status);
    }
    public static bool TryParseDecimal(ReadOnlySpan <byte >text, byte decimalSeparator, out decimal value) {
        var status = ParseStatus.Invalid;
        return TryParseDecimal(text, decimalSeparator, out value, out status);
    }
    public static bool TryParseDecimal(ReadOnlySpan <byte >text, byte decimalSeparator, out decimal value, out ParseStatus status) {
        value = 0m;
        status = ParseStatus.Invalid;
        var working = Trim(text);
        if (working.IsEmpty)
        {
            return false;
        }
        var negative = false;
        if (working[0] == NumericUnchecked.ToByte ('+'))
        {
            working = working.Slice(1);
        }
        else if (working[0] == NumericUnchecked.ToByte ('-'))
        {
            negative = true;
            working = working.Slice(1);
        }
        if (working.IsEmpty)
        {
            return false;
        }
        var seenDigit = false;
        var seenDecimal = false;
        var prevUnderscore = false;
        var scale = 0;
        var index = 0usize;
        var magnitude = 0m;
        try {
            while (index <working.Length)
            {
                let ch = working[index];
                if (ch == NumericUnchecked.ToByte ('_'))
                {
                    if (!seenDigit || prevUnderscore)
                    {
                        return false;
                    }
                    prevUnderscore = true;
                    index += 1;
                    continue;
                }
                if (ch == decimalSeparator)
                {
                    if (seenDecimal)
                    {
                        return false;
                    }
                    if (!seenDigit || prevUnderscore)
                    {
                        return false;
                    }
                    seenDecimal = true;
                    prevUnderscore = false;
                    index += 1;
                    continue;
                }
                if (ch == NumericUnchecked.ToByte ('e') || ch == NumericUnchecked.ToByte ('E'))
                {
                    break;
                }
                if (TryParseDecimalDigit (ch, out var digit)) {
                    magnitude = (magnitude * 10m) + NumericUnchecked.ToUInt32(digit);
                    if (seenDecimal)
                    {
                        scale += 1;
                        if (scale >DecimalMaxScale)
                        {
                            status = ParseStatus.Overflow;
                            value = 0m;
                            return false;
                        }
                    }
                    seenDigit = true;
                    prevUnderscore = false;
                    index += 1;
                    continue;
                }
                return false;
            }
        }
        catch(Std.OverflowException) {
            status = ParseStatus.Overflow;
            value = 0m;
            return false;
        }
        if (!seenDigit || prevUnderscore)
        {
            return false;
        }
        var exponentValue = 0;
        var exponentNegative = false;
        var exponentSeen = false;
        if (index <working.Length)
        {
            index += 1;
            if (index >= working.Length)
            {
                return false;
            }
            let head = working[index];
            if (head == NumericUnchecked.ToByte ('+'))
            {
                index += 1;
            }
            else if (head == NumericUnchecked.ToByte ('-'))
            {
                exponentNegative = true;
                index += 1;
            }
            if (index >= working.Length)
            {
                return false;
            }
            prevUnderscore = false;
            while (index <working.Length)
            {
                let ch = working[index];
                if (ch == NumericUnchecked.ToByte ('_'))
                {
                    if (!exponentSeen || prevUnderscore)
                    {
                        return false;
                    }
                    prevUnderscore = true;
                    index += 1;
                    continue;
                }
                if (!TryParseDecimalDigit (ch, out var digit)) {
                    return false;
                }
                exponentSeen = true;
                prevUnderscore = false;
                exponentValue = (exponentValue * 10) + NumericUnchecked.ToInt32(digit);
                if (exponentValue >1000)
                {
                    status = ParseStatus.Overflow;
                    value = 0m;
                    return false;
                }
                index += 1;
            }
            if (!exponentSeen || prevUnderscore)
            {
                return false;
            }
        }
        var netScale = scale;
        if (exponentSeen)
        {
            if (exponentNegative)
            {
                netScale += exponentValue;
            }
            else
            {
                netScale -= exponentValue;
            }
        }
        try {
            var result = magnitude;
            if (netScale >0)
            {
                if (netScale >DecimalMaxScale)
                {
                    status = ParseStatus.Overflow;
                    value = 0m;
                    return false;
                }
                result = result / Pow10(netScale);
            }
            else if (netScale <0)
            {
                let scaleFactor = - netScale;
                if (scaleFactor >DecimalMaxScale)
                {
                    status = ParseStatus.Overflow;
                    value = 0m;
                    return false;
                }
                result = result * Pow10(scaleFactor);
            }
            value = negative ?- result : result;
            status = ParseStatus.Success;
            return true;
        }
        catch(Std.OverflowException) {
            status = ParseStatus.Overflow;
            value = 0m;
            return false;
        }
        return false;
    }
    private static decimal Pow10(int exponent) {
        var result = 1m;
        var remaining = exponent;
        while (remaining >0)
        {
            result = result * 10m;
            remaining -= 1;
        }
        return result;
    }
}
