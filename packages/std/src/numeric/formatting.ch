namespace Std.Numeric;
import Std;
import Std.Core;
import Std.Memory;
import Std.Runtime.Collections;
import Std.Span;
import Std.Strings;
internal readonly struct NumericCultureData
{
    public readonly byte DecimalSeparator;
    public readonly byte GroupSeparator;
    public readonly int GroupSize;
    public init(byte decimalSeparator, byte groupSeparator, int groupSize) {
        DecimalSeparator = decimalSeparator;
        GroupSeparator = groupSeparator;
        GroupSize = groupSize;
    }
}
internal readonly struct FormatToken
{
    public readonly char Symbol;
    public readonly int Precision;
    public init(char symbol, int precision) {
        Symbol = symbol;
        Precision = precision;
    }
}
/// Shared numeric formatting helpers with culture-aware, two-string semantics.
internal static class NumericFormatting
{
    private const usize MAX_STACK_BUFFER = 128;
    private const byte ASCII_MINUS = 0x2Du8;
    private const byte ASCII_PLUS = 0x2Bu8;
    private const byte ASCII_ZERO = 0x30u8;
    private const int ASCII_ZERO_INT = 48;
    private const int ASCII_UPPER_A_INT = 65;
    private const int ASCII_LOWER_A_INT = 97;
    private const int MAX_FRACTION_PRECISION = 9;
    private const int DEFAULT_FIXED_PRECISION = 2;
    private const int DEFAULT_GENERAL_PRECISION = 6;
    private const int DEFAULT_EXPONENT_PRECISION = 6;
    private const int MIN_EXPONENT_DIGITS = 3;
    private const int DECIMAL_MAX_SCALE = 28;
    // Public entry points -------------------------------------------------
    public static string FormatSByte(sbyte value, string format, string culture) => FormatSignedInt(value, 8, format, culture,
    "SByte");
    public static string FormatByte(byte value, string format, string culture) => FormatUnsignedInt(NumericUnchecked.ToUInt32(value),
    8, format, culture, "Byte");
    public static string FormatInt16(short value, string format, string culture) => FormatSignedInt(value, 16, format, culture,
    "Int16");
    public static string FormatUInt16(ushort value, string format, string culture) => FormatUnsignedInt(NumericUnchecked.ToUInt32(value),
    16, format, culture, "UInt16");
    public static string FormatInt32(int value, string format, string culture) => FormatSignedInt(value, 32, format, culture,
    "Int32");
    public static string FormatUInt32(uint value, string format, string culture) => FormatUnsignedInt(value, 32, format,
    culture, "UInt32");
    public static string FormatInt64(long value, string format, string culture) => FormatSignedInt(value, 64, format, culture,
    "Int64");
    public static string FormatUInt64(ulong value, string format, string culture) => FormatUnsignedInt(value, 64, format,
    culture, "UInt64");
    public static string FormatInt128(int128 value, string format, string culture) => FormatSignedInt128(value, 128, format,
    culture, "Int128");
    public static string FormatUInt128(u128 value, string format, string culture) => FormatUnsignedInt128(value, 128, format,
    culture, "UInt128");
    public static string FormatIntPtr(nint value, string format, string culture) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return FormatSignedInt(NumericUnchecked.ToInt32(value), 32, format, culture, "IntPtr");
        }
        return FormatSignedInt(NumericUnchecked.ToInt64(value), 64, format, culture, "IntPtr");
    }
    public static string FormatUIntPtr(nuint value, string format, string culture) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return FormatUnsignedInt(NumericUnchecked.ToUInt32(value), 32, format, culture, "UIntPtr");
        }
        return FormatUnsignedInt(NumericUnchecked.ToUInt64(value), 64, format, culture, "UIntPtr");
    }
    public static string FormatFloat32(float value, string format, string culture) => FormatFloating((double) value, format,
    culture, "Float32");
    public static string FormatFloat64(double value, string format, string culture) => FormatFloating(value, format, culture,
    "Float64");
    public static string FormatDecimal(decimal value, string format, string culture) {
        var buffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatDecimal (value, buffer, out var written, format, culture)) {
            return ThrowFormatFailure("Decimal");
        }
        return FinishFormat(buffer, written, "Decimal");
    }
    public static bool TryFormatSByte(sbyte value, Span <byte >destination, out usize written, string format, string culture) => TryFormatSignedInt(value,
    8, destination, out written, format, culture);
    @never_inline public static bool TryFormatByte(byte value, Span <byte >destination, out usize written, string format,
    string culture) => TryFormatUnsignedInt(NumericUnchecked.ToUInt32(value), 8, destination, out written, format, culture);
    public static bool TryFormatInt16(short value, Span <byte >destination, out usize written, string format, string culture) => TryFormatSignedInt(value,
    16, destination, out written, format, culture);
    public static bool TryFormatUInt16(ushort value, Span <byte >destination, out usize written, string format, string culture) => TryFormatUnsignedInt(NumericUnchecked.ToUInt32(value),
    16, destination, out written, format, culture);
    public static bool TryFormatInt32(int value, Span <byte >destination, out usize written, string format, string culture) => TryFormatSignedInt(value,
    32, destination, out written, format, culture);
    public static bool TryFormatUInt32(uint value, Span <byte >destination, out usize written, string format, string culture) => TryFormatUnsignedInt(value,
    32, destination, out written, format, culture);
    public static bool TryFormatInt64(long value, Span <byte >destination, out usize written, string format, string culture) => TryFormatSignedInt(value,
    64, destination, out written, format, culture);
    public static bool TryFormatUInt64(ulong value, Span <byte >destination, out usize written, string format, string culture) => TryFormatUnsignedInt(value,
    64, destination, out written, format, culture);
    public static bool TryFormatInt128(int128 value, Span <byte >destination, out usize written, string format, string culture) => TryFormatSignedInt128(value,
    128, destination, out written, format, culture);
    public static bool TryFormatUInt128(u128 value, Span <byte >destination, out usize written, string format, string culture) => TryFormatUnsignedInt128(value,
    128, destination, out written, format, culture);
    public static bool TryFormatIntPtr(nint value, Span <byte >destination, out usize written, string format, string culture) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return TryFormatSignedInt(NumericUnchecked.ToInt32(value), 32, destination, out written, format, culture);
        }
        return TryFormatSignedInt(NumericUnchecked.ToInt64(value), 64, destination, out written, format, culture);
    }
    public static bool TryFormatUIntPtr(nuint value, Span <byte >destination, out usize written, string format, string culture) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return TryFormatUnsignedInt(NumericUnchecked.ToUInt32(value), 32, destination, out written, format, culture);
        }
        return TryFormatUnsignedInt(NumericUnchecked.ToUInt64(value), 64, destination, out written, format, culture);
    }
    public static bool TryFormatFloat32(float value, Span <byte >destination, out usize written, string format, string culture) => TryFormatFloatingInternal((double) value,
    destination, out written, format, culture);
    @never_inline public static bool TryFormatFloat64(double value, Span <byte >destination, out usize written, string format,
    string culture) => TryFormatFloatingInternal(value, destination, out written, format, culture);
    public static bool TryFormatDecimal(decimal value, Span <byte >destination, out usize written, string format, string culture) {
        var formatToken = ParseFormat(format);
        var cultureData = ResolveCulture(culture);
        var invariantBuffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryBuildInvariantDecimalString (value, invariantBuffer, out var invariantWritten)) {
            written = 0usize;
            return false;
        }
        var invariant = Utf8String.FromSpan(invariantBuffer.AsReadOnly().Slice(0, invariantWritten));
        return TryFormatDecimalText(invariant, formatToken, cultureData, destination, out written);
    }
    // Integer helpers -----------------------------------------------------
    @never_inline private static bool TryFormatDecimalText(string invariant, FormatToken token, NumericCultureData culture,
    Span <byte >destination, out usize written) {
        if (invariant == null)
        {
            written = 0usize;
            return false;
        }
        var symbol = token.Symbol == '\0' ?'G' : token.Symbol;
        var upper = symbol;
        if (upper >= 'a' && upper <= 'z')
        {
            upper = (char)(upper - 32);
        }
        var useGrouping = upper == 'N';
        var precision = token.Precision;
        if (upper == 'N' || upper == 'F')
        {
            precision = precision >= 0 ?precision : DEFAULT_FIXED_PRECISION;
        }
        if (upper != 'G' && upper != 'N' && upper != 'F')
        {
            throw new FormatException("Invalid format specifier for decimal value");
        }
        var negative = invariant.Length >0 && invariant[0] == '-';
        var start = negative ?1 : 0;
        var dotIndex = - 1;
        var idx = start;
        while (idx <invariant.Length)
        {
            if (invariant[idx] == '.')
            {
                dotIndex = idx;
                break;
            }
            idx += 1;
        }
        if (dotIndex <0)
        {
            dotIndex = invariant.Length;
        }
        var integerDigits = dotIndex - start;
        var fractionStart = dotIndex <invariant.Length ?dotIndex + 1 : invariant.Length;
        var availableFractionDigits = invariant.Length - fractionStart;
        var targetFractionDigits = availableFractionDigits;
        if (upper == 'N' || upper == 'F')
        {
            targetFractionDigits = precision;
        }
        var hasFraction = targetFractionDigits >0;
        var groupSeparatorCount = useGrouping ?ComputeGroupSeparatorCount(NumericUnchecked.ToUSize(integerDigits), culture.GroupSize) : 0usize;
        var total = (usize)(negative ?1 : 0) + NumericUnchecked.ToUSize(integerDigits) + groupSeparatorCount + (hasFraction ?1usize + NumericUnchecked.ToUSize(targetFractionDigits) : 0usize);
        if (total >destination.Length)
        {
            written = 0usize;
            return false;
        }
        var offset = 0usize;
        if (negative)
        {
            destination[0] = ASCII_MINUS;
            offset = 1usize;
        }
        var integerSpan = destination.Slice(offset, NumericUnchecked.ToUSize(integerDigits) + groupSeparatorCount);
        WriteIntegerDigits(invariant, start, integerDigits, useGrouping, culture, integerSpan);
        offset += integerSpan.Length;
        if (hasFraction)
        {
            destination[offset] = culture.DecimalSeparator;
            offset += 1usize;
            var fractionSpan = destination.Slice(offset, NumericUnchecked.ToUSize(targetFractionDigits));
            WriteFractionDigits(invariant, fractionStart, availableFractionDigits, targetFractionDigits, fractionSpan);
            offset += NumericUnchecked.ToUSize(targetFractionDigits);
        }
        written = offset;
        return true;
    }
    private static bool TryBuildInvariantDecimalString(decimal value, Span <byte >destination, out usize written) {
        written = 0usize;
        try {
            var negative = value <0m;
            var abs = negative ?- value : value;
            var scaled = abs;
            var scale = 0;
            while (scale <DECIMAL_MAX_SCALE)
            {
                var fractional = scaled % 1m;
                if (fractional == 0m)
                {
                    break;
                }
                scaled = scaled * 10m;
                scale += 1;
            }
            while (scale >0 && (scaled % 10m) == 0m)
            {
                scaled = scaled / 10m;
                scale -= 1;
            }
            var digitBuffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
            var digitCount = 0;
            if (scaled == 0m)
            {
                digitBuffer[0] = ASCII_ZERO;
                digitCount = 1;
            }
            else
            {
                var temp = scaled;
                while (temp >0m && digitCount < (int) MAX_STACK_BUFFER)
                {
                    temp = temp / 10m;
                    digitCount += 1;
                }
                if (digitCount == 0 || digitCount > (int) MAX_STACK_BUFFER)
                {
                    written = 0usize;
                    return false;
                }
                temp = scaled;
                var writeIndex = digitCount;
                while (writeIndex >0)
                {
                    var remainder = temp % 10m;
                    var digit = 0;
                    var remainderCopy = remainder;
                    while (remainderCopy >= 1m)
                    {
                        remainderCopy -= 1m;
                        digit += 1;
                    }
                    temp = (temp - remainder) / 10m;
                    writeIndex -= 1;
                    digitBuffer[writeIndex] = NumericUnchecked.ToByte(ASCII_ZERO_INT + digit);
                }
            }
            var total = digitCount;
            if (scale >0)
            {
                total += 1;
                if (scale >digitCount)
                {
                    total += scale - digitCount;
                }
            }
            if (negative)
            {
                total += 1;
            }
            var needed = NumericUnchecked.ToUSize(total);
            if (needed >destination.Length)
            {
                written = 0usize;
                return false;
            }
            var offset = 0usize;
            if (negative)
            {
                destination[offset] = ASCII_MINUS;
                offset += 1usize;
            }
            if (scale == 0)
            {
                destination.Slice(offset, digitCount).CopyFrom(digitBuffer.AsReadOnly().Slice(0, digitCount));
                written = offset + digitCount;
                return true;
            }
            if (scale >= digitCount)
            {
                destination[offset] = ASCII_ZERO;
                offset += 1usize;
                destination[offset] = NumericUnchecked.ToByte('.');
                offset += 1usize;
                var zeros = scale - digitCount;
                for (var i = 0; i <zeros; i += 1) {
                    destination[offset + (usize) i] = ASCII_ZERO;
                }
                offset += (usize) zeros;
                destination.Slice(offset, digitCount).CopyFrom(digitBuffer.AsReadOnly().Slice(0, digitCount));
                written = offset + digitCount;
                return true;
            }
            var integerDigits = digitCount - scale;
            destination.Slice(offset, integerDigits).CopyFrom(digitBuffer.AsReadOnly().Slice(0, integerDigits));
            offset += integerDigits;
            destination[offset] = NumericUnchecked.ToByte('.');
            offset += 1usize;
            destination.Slice(offset, scale).CopyFrom(digitBuffer.AsReadOnly().Slice(integerDigits, scale));
            written = offset + scale;
            return true;
        }
        catch(OverflowException) {
            written = 0usize;
            return false;
        }
        written = 0usize;
        return false;
    }
    @never_inline private static void WriteIntegerDigits(string text, int start, int count, bool useGrouping, NumericCultureData culture,
    Span <byte >destination) {
        if (! useGrouping || count <= culture.GroupSize || culture.GroupSize <= 0)
        {
            var i = 0;
            while (i <count)
            {
                destination[i] = NumericUnchecked.ToByte(text[start + i]);
                i += 1;
            }
            return;
        }
        var groupSize = culture.GroupSize;
        var leading = count % groupSize;
        if (leading == 0)
        {
            leading = groupSize;
        }
        var destIndex = 0usize;
        var index = start;
        var copyCount = leading;
        while (true)
        {
            var copied = 0;
            while (copied <copyCount)
            {
                destination[destIndex] = NumericUnchecked.ToByte(text[index]);
                destIndex += 1usize;
                index += 1;
                copied += 1;
            }
            var remaining = count - (index - start);
            if (remaining <= 0)
            {
                break;
            }
            destination[destIndex] = culture.GroupSeparator;
            destIndex += 1usize;
            copyCount = groupSize;
        }
    }
    @never_inline private static void WriteFractionDigits(string text, int start, int availableDigits, int requiredDigits,
    Span <byte >destination) {
        var copy = requiredDigits <availableDigits ?requiredDigits : availableDigits;
        var i = 0;
        while (i <copy)
        {
            destination[i] = NumericUnchecked.ToByte(text[start + i]);
            i += 1;
        }
        while (i <requiredDigits)
        {
            destination[i] = ASCII_ZERO;
            i += 1;
        }
    }
    private static string FormatSignedInt(long value, int bitWidth, string format, string culture, string typeName) {
        var buffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatSignedInt (value, bitWidth, buffer, out var written, format, culture)) {
            return ThrowFormatFailure(typeName);
        }
        return FinishFormat(buffer, written, typeName);
    }
    private static string FormatUnsignedInt(ulong value, int bitWidth, string format, string culture, string typeName) {
        var buffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatUnsignedInt (value, bitWidth, buffer, out var written, format, culture)) {
            return ThrowFormatFailure(typeName);
        }
        return FinishFormat(buffer, written, typeName);
    }
    private static string FormatSignedInt128(int128 value, int bitWidth, string format, string culture, string typeName) {
        var buffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatSignedInt128 (value, bitWidth, buffer, out var written, format, culture)) {
            return ThrowFormatFailure(typeName);
        }
        return FinishFormat(buffer, written, typeName);
    }
    private static string FormatUnsignedInt128(u128 value, int bitWidth, string format, string culture, string typeName) {
        var buffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatUnsignedInt128 (value, bitWidth, buffer, out var written, format, culture)) {
            return ThrowFormatFailure(typeName);
        }
        return FinishFormat(buffer, written, typeName);
    }
    private static bool TryFormatSignedInt(long value, int bitWidth, Span <byte >destination, out usize written, string format,
    string culture) {
        var token = ParseFormat(format);
        var cultureData = ResolveCulture(culture);
        var scratch = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatSignedIntCore (value, bitWidth, scratch, out var local, token, cultureData)) {
            written = 0usize;
            return false;
        }
        if (destination.Length <local)
        {
            written = 0usize;
            return false;
        }
        destination.Slice(0, local).CopyFrom(scratch.AsReadOnly().Slice(0, local));
        written = local;
        return true;
    }
    private static bool TryFormatUnsignedInt(ulong value, int bitWidth, Span <byte >destination, out usize written, string format,
    string culture) {
        var token = ParseFormat(format);
        var cultureData = ResolveCulture(culture);
        var scratch = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatUnsignedIntCore (value, bitWidth, scratch, out var local, token, cultureData)) {
            written = 0usize;
            return false;
        }
        if (destination.Length <local)
        {
            written = 0usize;
            return false;
        }
        destination.Slice(0, local).CopyFrom(scratch.AsReadOnly().Slice(0, local));
        written = local;
        return true;
    }
    private static bool TryFormatSignedInt128(int128 value, int bitWidth, Span <byte >destination, out usize written, string format,
    string culture) {
        var token = ParseFormat(format);
        var cultureData = ResolveCulture(culture);
        var scratch = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatSignedIntCore128 (value, bitWidth, scratch, out var local, token, cultureData)) {
            written = 0usize;
            return false;
        }
        if (destination.Length <local)
        {
            written = 0usize;
            return false;
        }
        destination.Slice(0, local).CopyFrom(scratch.AsReadOnly().Slice(0, local));
        written = local;
        return true;
    }
    private static bool TryFormatUnsignedInt128(u128 value, int bitWidth, Span <byte >destination, out usize written, string format,
    string culture) {
        var token = ParseFormat(format);
        var cultureData = ResolveCulture(culture);
        var scratch = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
        if (! TryFormatUnsignedIntCore128 (value, bitWidth, scratch, out var local, token, cultureData)) {
            written = 0usize;
            return false;
        }
        if (destination.Length <local)
        {
            written = 0usize;
            return false;
        }
        destination.Slice(0, local).CopyFrom(scratch.AsReadOnly().Slice(0, local));
        written = local;
        return true;
    }
    private static bool TryFormatSignedIntCore(long value, int bitWidth, Span <byte >destination, out usize written, FormatToken token,
    NumericCultureData culture) {
        var negative = value <0L;
        var magnitude = GetMagnitude(value);
        var normalized = NormalizeFormatChar(token.Symbol);
        switch (normalized)
        {
            case 'G':
            case 'D':
                return TryFormatDecimalValue(magnitude, negative, token.Precision >= 0 ?token.Precision : 0, 0, 0ul, false,
                culture, destination, out written);
            case 'N':
                return TryFormatDecimalValue(magnitude, negative, 0, token.Precision >= 0 ?token.Precision : 0, 0ul, true,
                culture, destination, out written);
            case 'X':
                var minHexDigits = token.Precision >= 0 ?token.Precision : 0;
                if (minHexDigits == 0 && negative)
                {
                    minHexDigits = bitWidth / 4;
                }
                return TryFormatHex(NumericUnchecked.ToUInt64(value), bitWidth, IsUpper(token.Symbol), minHexDigits, destination,
                out written);
            default :
                throw new FormatException("Invalid format specifier for integer value");
            }
        }
        private static bool TryFormatUnsignedIntCore(ulong value, int bitWidth, Span <byte >destination, out usize written,
        FormatToken token, NumericCultureData culture) {
            var normalized = NormalizeFormatChar(token.Symbol);
            switch (normalized)
            {
                case 'G':
                case 'D':
                    return TryFormatDecimalValue(value, false, token.Precision >= 0 ?token.Precision : 0, 0, 0ul, false,
                    culture, destination, out written);
                case 'N':
                    return TryFormatDecimalValue(value, false, 0, token.Precision >= 0 ?token.Precision : 0, 0ul, true, culture,
                    destination, out written);
                case 'X':
                    var minHexDigits = token.Precision >= 0 ?token.Precision : 0;
                    return TryFormatHex(value, bitWidth, IsUpper(token.Symbol), minHexDigits, destination, out written);
                default :
                    throw new FormatException("Invalid format specifier for integer value");
                }
            }
            private static ulong GetMagnitude(long value) {
                if (value >= 0L)
                {
                    return NumericUnchecked.ToUInt64(value);
                }
                if (value == NumericConstants.Int64Min)
                {
                    return 9223372036854775808ul;
                }
                return NumericUnchecked.ToUInt64(- value);
            }
            private static bool TryFormatSignedIntCore128(int128 value, int bitWidth, Span <byte >destination, out usize written,
            FormatToken token, NumericCultureData culture) {
                var negative = value <0;
                var magnitude = GetMagnitude128(value);
                var normalized = NormalizeFormatChar(token.Symbol);
                switch (normalized)
                {
                    case 'G':
                    case 'D':
                        return TryFormatDecimalValue128(magnitude, negative, token.Precision >= 0 ?token.Precision : 0, 0,
                        0ul, false, culture, destination, out written);
                    case 'N':
                        return TryFormatDecimalValue128(magnitude, negative, 0, token.Precision >= 0 ?token.Precision : 0,
                        0ul, true, culture, destination, out written);
                    case 'X':
                        var minHexDigits = token.Precision >= 0 ?token.Precision : 0;
                        if (minHexDigits == 0 && negative)
                        {
                            minHexDigits = bitWidth / 4;
                        }
                        return TryFormatHex128(NumericUnchecked.ToUInt128(value), bitWidth, IsUpper(token.Symbol), minHexDigits,
                        destination, out written);
                    default :
                        throw new FormatException("Invalid format specifier for integer value");
                    }
                }
                private static bool TryFormatUnsignedIntCore128(u128 value, int bitWidth, Span <byte >destination, out usize written,
                FormatToken token, NumericCultureData culture) {
                    var normalized = NormalizeFormatChar(token.Symbol);
                    switch (normalized)
                    {
                        case 'G':
                        case 'D':
                            return TryFormatDecimalValue128(value, false, token.Precision >= 0 ?token.Precision : 0, 0, 0ul,
                            false, culture, destination, out written);
                        case 'N':
                            return TryFormatDecimalValue128(value, false, 0, token.Precision >= 0 ?token.Precision : 0, 0ul,
                            true, culture, destination, out written);
                        case 'X':
                            var minHexDigits = token.Precision >= 0 ?token.Precision : 0;
                            return TryFormatHex128(value, bitWidth, IsUpper(token.Symbol), minHexDigits, destination, out written);
                        default :
                            throw new FormatException("Invalid format specifier for integer value");
                        }
                    }
                    private static u128 GetMagnitude128(int128 value) {
                        if (value >= 0)
                        {
                            return NumericUnchecked.ToUInt128(value);
                        }
                        if (value == NumericConstants.Int128Min)
                        {
                            return NumericConstants.Int128MinMagnitude;
                        }
                        return NumericUnchecked.ToUInt128(- value);
                    }
                    // Floating-point helpers ---------------------------------------------
                    private static string FormatFloating(double value, string format, string culture, string typeName) {
                        var buffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
                        if (! TryFormatFloatingInternal (value, buffer, out var written, format, culture)) {
                            return ThrowFormatFailure(typeName);
                        }
                        return FinishFormat(buffer, written, typeName);
                    }
                    private static bool TryFormatFloatingInternal(double value, Span <byte >destination, out usize written,
                    string format, string culture) {
                        var token = ParseFormat(format);
                        var cultureData = ResolveCulture(culture);
                        var scratch = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
                        if (! TryFormatFloatingCore (value, token, cultureData, scratch, out var local)) {
                            written = 0usize;
                            return false;
                        }
                        if (destination.Length <local)
                        {
                            written = 0usize;
                            return false;
                        }
                        destination.Slice(0, local).CopyFrom(scratch.AsReadOnly().Slice(0, local));
                        written = local;
                        return true;
                    }
                    private static bool TryFormatFloatingCore(double value, FormatToken token, NumericCultureData culture,
                    Span <byte >destination, out usize written) {
                        if (TryWriteSpecialFloating (value, destination, out written)) {
                            return true;
                        }
                        var normalized = NormalizeFormatChar(token.Symbol);
                        switch (normalized)
                        {
                            case 'G':
                                return TryFormatGeneral(value, token.Precision, culture, destination, out written);
                            case 'F':
                                return TryFormatFixed(value, token.Precision >= 0 ?token.Precision : DEFAULT_FIXED_PRECISION,
                                false, culture, destination, out written);
                            case 'N':
                                return TryFormatFixed(value, token.Precision >= 0 ?token.Precision : DEFAULT_FIXED_PRECISION,
                                true, culture, destination, out written);
                            case 'E':
                                return TryFormatExponent(value, token.Precision >= 0 ?token.Precision : DEFAULT_EXPONENT_PRECISION,
                                IsUpper(token.Symbol), culture, destination, out written);
                            default :
                                throw new FormatException("Invalid format specifier for floating-point value");
                            }
                        }
                        @never_inline private static bool TryFormatGeneral(double value, int precision, NumericCultureData culture,
                        Span <byte >destination, out usize written) {
                            var effectivePrecision = precision >= 0 ?precision : DEFAULT_GENERAL_PRECISION;
                            if (effectivePrecision >MAX_FRACTION_PRECISION)
                            {
                                effectivePrecision = MAX_FRACTION_PRECISION;
                            }
                            var abs = value <0.0d ?- value : value;
                            if (abs != 0.0d && (abs >= 1000000000.0d || abs <0.0001d))
                            {
                                return TryFormatExponent(value, effectivePrecision, true, culture, destination, out written);
                            }
                            if (! TryFormatFixed (value, effectivePrecision, false, culture, destination, out written)) {
                                return false;
                            }
                            written = TrimTrailingZeros(destination, written, culture.DecimalSeparator);
                            return true;
                        }
                        @never_inline private static bool TryFormatFixed(double value, int precision, bool useGrouping, NumericCultureData culture,
                        Span <byte >destination, out usize written) {
                            ValidateFractionalPrecision(precision);
                            var negative = value <0.0d;
                            var abs = negative ?- value : value;
                            var scale = Pow10UInt(precision);
                            let scaleDouble = NumericUnchecked.ToFloat64(scale);
                            var scaled = abs * scaleDouble;
                            var roundedDouble = scaled + 0.5d;
                            if (roundedDouble >NumericUnchecked.ToFloat64 (NumericConstants.UInt64Max))
                            {
                                // Too large for fixed formatting; fall back to exponential.
                                return TryFormatExponent(value, precision, true, culture, destination, out written);
                            }
                            var rounded = NumericUnchecked.ToUInt64(roundedDouble);
                            var integerPart = rounded / scale;
                            var fractional = rounded % scale;
                            return TryFormatDecimalValue(integerPart, negative, 0, precision, fractional, useGrouping, culture,
                            destination, out written);
                        }
                        @never_inline private static bool TryFormatExponent(double value, int precision, bool uppercase,
                        NumericCultureData culture, Span <byte >destination, out usize written) {
                            ValidateFractionalPrecision(precision);
                            var negative = value <0.0d;
                            var abs = negative ?- value : value;
                            if (abs == 0.0d)
                            {
                                return TryFormatDecimalValue(0ul, negative, 1, precision, 0ul, false, culture, destination,
                                out written);
                            }
                            var exponent = 0;
                            var mantissa = abs;
                            while (mantissa >= 10.0d)
                            {
                                mantissa /= 10.0d;
                                exponent += 1;
                            }
                            while (mantissa <1.0d)
                            {
                                mantissa *= 10.0d;
                                exponent -= 1;
                            }
                            var scale = Pow10UInt(precision);
                            let scaleDouble = NumericUnchecked.ToFloat64(scale);
                            var scaled = mantissa * scaleDouble;
                            var rounded = NumericUnchecked.ToUInt64(scaled + 0.5d);
                            if (rounded >= 10ul * scale)
                            {
                                rounded /= 10ul;
                                exponent += 1;
                            }
                            var integerPart = rounded / scale;
                            var fractional = rounded % scale;
                            var mantissaBuffer = StackAlloc.Span <byte >(MAX_STACK_BUFFER);
                            if (! TryFormatDecimalValue (integerPart, negative, 1, precision, fractional, false, culture,
                            mantissaBuffer, out var mantissaWritten)) {
                                written = 0usize;
                                return false;
                            }
                            var exponentValue = exponent <0 ?- exponent : exponent;
                            var exponentDigits = CountDecimalDigits((ulong) exponentValue);
                            if (exponentDigits < (usize) MIN_EXPONENT_DIGITS)
                            {
                                exponentDigits = (usize) MIN_EXPONENT_DIGITS;
                            }
                            var total = mantissaWritten + 1usize + 1usize + exponentDigits;
                            if (total >destination.Length)
                            {
                                written = 0usize;
                                return false;
                            }
                            destination.Slice(0, mantissaWritten).CopyFrom(mantissaBuffer.AsReadOnly().Slice(0, mantissaWritten));
                            var offset = mantissaWritten;
                            destination[offset] = uppercase ?NumericUnchecked.ToByte('E') : NumericUnchecked.ToByte('e');
                            offset += 1usize;
                            destination[offset] = exponent <0 ?ASCII_MINUS : ASCII_PLUS;
                            offset += 1usize;
                            WritePaddedInteger((ulong) exponentValue, exponentDigits, destination.Slice(offset, exponentDigits));
                            written = total;
                            return true;
                        }
                        // Shared decimal/hex helpers -----------------------------------------
                        private static bool TryFormatDecimalValue(ulong magnitude, bool negative, int minDigits, int fractionalDigits,
                        ulong fractionalValue, bool useGrouping, NumericCultureData culture, Span <byte >destination, out usize written) {
                            if (fractionalDigits <0)
                            {
                                throw new FormatException("Negative precision is not supported");
                            }
                            var integerDigits = CountDecimalDigits(magnitude);
                            if (integerDigits < (usize) minDigits)
                            {
                                integerDigits = (usize) minDigits;
                            }
                            var separatorCount = useGrouping ?ComputeGroupSeparatorCount(integerDigits, culture.GroupSize) : 0usize;
                            var hasFraction = fractionalDigits >0;
                            var total = integerDigits + separatorCount + (negative ?1usize : 0usize);
                            if (hasFraction)
                            {
                                total += 1usize + (usize) fractionalDigits;
                            }
                            if (total >destination.Length)
                            {
                                written = 0usize;
                                return false;
                            }
                            var offset = 0usize;
                            if (negative)
                            {
                                destination[0] = ASCII_MINUS;
                                offset = 1usize;
                            }
                            var integerSpan = destination.Slice(offset, integerDigits + separatorCount);
                            WriteIntegerWithGrouping(magnitude, integerDigits, useGrouping, culture, integerSpan);
                            offset += integerSpan.Length;
                            if (hasFraction)
                            {
                                destination[offset] = culture.DecimalSeparator;
                                offset += 1usize;
                                WriteFraction(fractionalValue, fractionalDigits, destination.Slice(offset, (usize) fractionalDigits));
                                offset += (usize) fractionalDigits;
                            }
                            written = offset;
                            return true;
                        }
                        private static void WriteIntegerWithGrouping(ulong magnitude, usize digitCount, bool useGrouping,
                        NumericCultureData culture, Span <byte >destination) {
                            var groupSize = culture.GroupSize;
                            var remainingDigits = digitCount;
                            var index = destination.Length;
                            var groupCounter = 0;
                            var remaining = magnitude;
                            while (remainingDigits >0)
                            {
                                var digit = remaining % 10ul;
                                remaining /= 10ul;
                                index -= 1usize;
                                destination[index] = NumericUnchecked.ToByte(ASCII_ZERO_INT + NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(digit)));
                                remainingDigits -= 1usize;
                                groupCounter += 1;
                                if (useGrouping && remainingDigits >0 && groupCounter == groupSize)
                                {
                                    index -= 1usize;
                                    destination[index] = culture.GroupSeparator;
                                    groupCounter = 0;
                                }
                            }
                        }
                        private static bool TryFormatDecimalValue128(u128 magnitude, bool negative, int minDigits, int fractionalDigits,
                        ulong fractionalValue, bool useGrouping, NumericCultureData culture, Span <byte >destination, out usize written) {
                            if (fractionalDigits <0)
                            {
                                throw new Std.FormatException("Negative precision is not supported");
                            }
                            var integerDigits = CountDecimalDigits128(magnitude);
                            if (integerDigits < (usize) minDigits)
                            {
                                integerDigits = (usize) minDigits;
                            }
                            var separatorCount = useGrouping ?ComputeGroupSeparatorCount(integerDigits, culture.GroupSize) : 0usize;
                            var hasFraction = fractionalDigits >0;
                            var total = integerDigits + separatorCount + (negative ?1usize : 0usize);
                            if (hasFraction)
                            {
                                total += 1usize + (usize) fractionalDigits;
                            }
                            if (total >destination.Length)
                            {
                                written = 0usize;
                                return false;
                            }
                            var offset = 0usize;
                            if (negative)
                            {
                                destination[0] = ASCII_MINUS;
                                offset = 1usize;
                            }
                            var integerSpan = destination.Slice(offset, integerDigits + separatorCount);
                            WriteIntegerWithGrouping128(magnitude, integerDigits, useGrouping, culture, integerSpan);
                            offset += integerSpan.Length;
                            if (hasFraction)
                            {
                                destination[offset] = culture.DecimalSeparator;
                                offset += 1usize;
                                WriteFraction(fractionalValue, fractionalDigits, destination.Slice(offset, (usize) fractionalDigits));
                                offset += (usize) fractionalDigits;
                            }
                            written = offset;
                            return true;
                        }
                        private static void WriteIntegerWithGrouping128(u128 magnitude, usize digitCount, bool useGrouping,
                        NumericCultureData culture, Span <byte >destination) {
                            var groupSize = culture.GroupSize;
                            var remainingDigits = digitCount;
                            var index = destination.Length;
                            var groupCounter = 0;
                            var remaining = magnitude;
                            while (remainingDigits >0)
                            {
                                var digit = remaining % 10u128;
                                remaining /= 10u128;
                                index -= 1usize;
                                destination[index] = NumericUnchecked.ToByte(ASCII_ZERO_INT + NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(digit)));
                                remainingDigits -= 1usize;
                                groupCounter += 1;
                                if (useGrouping && remainingDigits >0 && groupCounter == groupSize)
                                {
                                    index -= 1usize;
                                    destination[index] = culture.GroupSeparator;
                                    groupCounter = 0;
                                }
                            }
                        }
                        @never_inline private static void WriteFraction(ulong fractionalValue, int digits, Span <byte >destination) {
                            var remaining = fractionalValue;
                            var index = digits;
                            while (index >0)
                            {
                                index -= 1;
                                let digit = NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(remaining % 10ul));
                                destination[index] = NumericUnchecked.ToByte(ASCII_ZERO_INT + digit);
                                remaining /= 10ul;
                            }
                        }
                        @never_inline private static bool TryFormatHex(ulong value, int bitWidth, bool uppercase, int minDigits,
                        Span <byte >destination, out usize written) {
                            let digits = minDigits >0 ?(usize) minDigits : CountHexDigits(value);
                            if (digits == 0usize)
                            {
                                written = 0usize;
                                return false;
                            }
                            if (destination.Length <digits)
                            {
                                written = 0usize;
                                return false;
                            }
                            var remaining = value;
                            for (var index = digits; index >0usize; index -= 1usize) {
                                let nibble = NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(remaining & 0xFul));
                                destination[index - 1usize] = HexDigit(nibble, uppercase);
                                remaining >>= 4;
                            }
                            written = digits;
                            return true;
                        }
                        private static bool TryFormatHex128(u128 value, int bitWidth, bool uppercase, int minDigits, Span <byte >destination,
                        out usize written) {
                            let digits = minDigits >0 ?(usize) minDigits : CountHexDigits128(value);
                            if (digits == 0usize)
                            {
                                written = 0usize;
                                return false;
                            }
                            if (destination.Length <digits)
                            {
                                written = 0usize;
                                return false;
                            }
                            var remaining = value;
                            for (var index = digits; index >0usize; index -= 1usize) {
                                let nibble = NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(remaining & 0xFu128));
                                destination[index - 1usize] = HexDigit(nibble, uppercase);
                                remaining >>= 4;
                            }
                            written = digits;
                            return true;
                        }
                        @never_inline private static void WritePaddedInteger(ulong value, usize minDigits, Span <byte >destination) {
                            var digits = CountDecimalDigits(value);
                            var totalDigits = digits;
                            if (totalDigits <minDigits)
                            {
                                totalDigits = minDigits;
                            }
                            if (destination.Length <totalDigits)
                            {
                                return;
                            }
                            var index = totalDigits;
                            var remaining = value;
                            while (index >0usize)
                            {
                                let digit = NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(remaining % 10ul));
                                destination[index - 1usize] = NumericUnchecked.ToByte(ASCII_ZERO_INT + digit);
                                remaining /= 10ul;
                                index -= 1usize;
                            }
                        }
                        // Parsing and shared utilities ---------------------------------------
                        private static FormatToken ParseFormat(string format) {
                            if (format == null || format.Length == 0)
                            {
                                return new FormatToken('G', - 1);
                            }
                            var symbol = format[0];
                            var precision = - 1;
                            var index = 1;
                            while (index <format.Length)
                            {
                                let ch = format[index];
                                if (ch <'0' || ch >'9')
                                {
                                    throw new FormatException("Invalid format string");
                                }
                                var digit = (int) ch - (int) '0';
                                if (precision <0)
                                {
                                    precision = 0;
                                }
                                precision = precision * 10 + digit;
                                index += 1;
                            }
                            return new FormatToken(symbol, precision);
                        }
                        private static NumericCultureData ResolveCulture(string culture) {
                            if (culture == null || culture.Length == 0 || EqualsIgnoreAsciiCase (culture, "invariant"))
                            {
                                return new NumericCultureData(NumericUnchecked.ToByte('.'), NumericUnchecked.ToByte(','),
                                3);
                            }
                            if (EqualsIgnoreAsciiCase (culture, "en-US"))
                            {
                                return new NumericCultureData(NumericUnchecked.ToByte('.'), NumericUnchecked.ToByte(','),
                                3);
                            }
                            if (EqualsIgnoreAsciiCase (culture, "fr-FR"))
                            {
                                return new NumericCultureData(NumericUnchecked.ToByte(','), NumericUnchecked.ToByte(' '),
                                3);
                            }
                            throw new ArgumentException("Unsupported culture");
                        }
                        private static bool EqualsIgnoreAsciiCase(string left, string right) {
                            if (left == null || right == null)
                            {
                                return false;
                            }
                            if (left.Length != right.Length)
                            {
                                return false;
                            }
                            var index = 0;
                            while (index <left.Length)
                            {
                                let l = ToUpperAscii(left[index]);
                                let r = ToUpperAscii(right[index]);
                                if (l != r)
                                {
                                    return false;
                                }
                                index += 1;
                            }
                            return true;
                        }
                        private static char ToUpperAscii(char value) {
                            if (value >= 'a' && value <= 'z')
                            {
                                return NumericUnchecked.ToChar(NumericUnchecked.ToInt32(value) - 32);
                            }
                            return value;
                        }
                        private static char NormalizeFormatChar(char value) => ToUpperAscii(value);
                        @never_inline private static bool IsUpper(char value) => value >= 'A' && value <= 'Z';
                        @never_inline private static void ValidateFractionalPrecision(int precision) {
                            if (precision <0)
                            {
                                throw new FormatException("Negative precision is not supported");
                            }
                            if (precision >MAX_FRACTION_PRECISION)
                            {
                                throw new FormatException("Requested precision exceeds supported limits");
                            }
                        }
                        @never_inline private static ulong Pow10UInt(int exponent) {
                            var result = 1ul;
                            var index = 0;
                            while (index <exponent)
                            {
                                if (result >NumericConstants.UInt64Max / 10ul)
                                {
                                    throw new FormatException("Precision too large");
                                }
                                result *= 10ul;
                                index += 1;
                            }
                            return result;
                        }
                        private static usize CountDecimalDigits(ulong value) {
                            var digits = 1usize;
                            var remaining = value;
                            while (remaining >= 10ul)
                            {
                                remaining /= 10ul;
                                digits += 1usize;
                            }
                            return digits;
                        }
                        @never_inline private static usize CountHexDigits(ulong value) {
                            var digits = 1usize;
                            var remaining = value;
                            while (remaining >0xFul)
                            {
                                remaining >>= 4;
                                digits += 1usize;
                            }
                            return digits;
                        }
                        private static usize CountDecimalDigits128(u128 value) {
                            var digits = 1usize;
                            var remaining = value;
                            while (remaining >= 10u128)
                            {
                                remaining /= 10u128;
                                digits += 1usize;
                            }
                            return digits;
                        }
                        private static usize CountHexDigits128(u128 value) {
                            var digits = 1usize;
                            var remaining = value;
                            while (remaining >0xFu128)
                            {
                                remaining >>= 4;
                                digits += 1usize;
                            }
                            return digits;
                        }
                        @never_inline private static usize ComputeGroupSeparatorCount(usize digitCount, int groupSize) {
                            if (digitCount <= (usize) groupSize)
                            {
                                return 0usize;
                            }
                            return(digitCount - 1usize) / (usize) groupSize;
                        }
                        @never_inline private static byte HexDigit(int value, bool uppercase) {
                            if (value <10)
                            {
                                return NumericUnchecked.ToByte(ASCII_ZERO_INT + value);
                            }
                            let baseValue = uppercase ?ASCII_UPPER_A_INT : ASCII_LOWER_A_INT;
                            return NumericUnchecked.ToByte(baseValue + (value - 10));
                        }
                        private static bool TryWriteSpecialFloating(double value, Span <byte >destination, out usize written) {
                            written = 0usize;
                            if (value != value)
                            {
                                return TryWriteAscii(destination, out written, "NaN");
                            }
                            let posInf = 1.0d / 0.0d;
                            let negInf = - 1.0d / 0.0d;
                            if (value == posInf)
                            {
                                return TryWriteAscii(destination, out written, "Infinity");
                            }
                            if (value == negInf)
                            {
                                return TryWriteAscii(destination, out written, "-Infinity");
                            }
                            return false;
                        }
                        @never_inline private static bool TryWriteAscii(Span <byte >destination, out usize written, string text) {
                            written = 0usize;
                            if (text == null)
                            {
                                return false;
                            }
                            let len = (usize) text.Length;
                            if (destination.Length <len)
                            {
                                return false;
                            }
                            var i = 0usize;
                            while (i <len)
                            {
                                destination[i] = NumericUnchecked.ToByte(text[i]);
                                i += 1usize;
                            }
                            written = len;
                            return true;
                        }
                        @never_inline private static usize TrimTrailingZeros(Span <byte >buffer, usize length, byte decimalSeparator) {
                            var end = length;
                            while (end >0usize && buffer[end - 1usize] == ASCII_ZERO)
                            {
                                end -= 1usize;
                            }
                            if (end >0usize && buffer[end - 1usize] == decimalSeparator)
                            {
                                end -= 1usize;
                            }
                            return end;
                        }
                        private static string ThrowFormatFailure(string typeName) {
                            throw new FormatException(typeName + " formatting failed");
                        }
                        private static string FinishFormat(Span <byte >buffer, usize written, string typeName) {
                            var readonlyView = buffer.AsReadOnly();
                            var handle = readonlyView.Raw;
                            if (written >handle.Length)
                            {
                                return ThrowFormatFailure(typeName);
                            }
                            var slice = CoreIntrinsics.DefaultValue <StrPtr >();
                            slice.Pointer = handle.Data.Pointer;
                            slice.Length = written;
                            return SpanIntrinsics.chic_rt_string_from_slice(slice);
                        }
        }
