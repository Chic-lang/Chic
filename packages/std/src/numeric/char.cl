namespace Std.Numeric;
import Std;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Strings;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "char", kind = "char", bits = 16, aliases = ["char",
"Char", "Std.Char", "Std.Numeric.Char", "System.Char"], c_type = "uint16_t") public readonly struct Char : IComparable <Char >, IComparable <char >, IConvertible, IEquatable <Char >, IEquatable <char >, IParsable <Char >, ISpanParsable <Char >, IUtf8SpanParsable <Char >, IAdditionOperators <Char, Char, Char >, IAdditiveIdentity <Char, Char >, IBinaryInteger <Char >, IBinaryNumber <Char >, IBitwiseOperators <Char, Char, Char >, IComparisonOperators <Char, Char, bool >, IDecrementOperators <Char >, IDivisionOperators <Char, Char, Char >, IEqualityOperators <Char, Char, bool >, IIncrementOperators <Char >, IMinMaxValue <Char >, IModulusOperators <Char, Char, Char >, IMultiplicativeIdentity <Char, Char >, IMultiplyOperators <Char, Char, Char >, INumber <Char >, INumberBase <Char >, IShiftOperators <Char, int, Char >, ISubtractionOperators <Char, Char, Char >, IUnaryNegationOperators <Char, Char >, IUnaryPlusOperators <Char, Char >, IUnsignedNumber <Char >, Clone, Copy
{
    private readonly char value;
    public const char MinValue = (char) 0x0000;
    public const char MaxValue = (char) 0xFFFF;
    public init(char value) {
        this.value = value;
    }
    public char ToChar() => value;
    public static Char From(char value) => new Char(value);
    public static Char Zero => new Char((char) 0);
    public static Char One => new Char((char) 1);
    public static Char AdditiveIdentity() => Zero;
    public static Char MultiplicativeIdentity() => One;
    public static Char MinValueConst => new Char(MinValue);
    public static Char MaxValueConst => new Char(MaxValue);
    public string ToString(IFormatProvider provider) => CharRuntimeIntrinsics.chic_rt_string_from_char(value);
    public bool ToBoolean(IFormatProvider provider) => value != (char) 0;
    public char ToChar(IFormatProvider provider) => value;
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(NumericUnchecked.ToUInt64(value));
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(NumericUnchecked.ToUInt64(value));
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(NumericUnchecked.ToUInt64(value));
    public ushort ToUInt16(IFormatProvider provider) => value;
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(NumericUnchecked.ToUInt64(value));
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(NumericUnchecked.ToUInt64(value));
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked(NumericUnchecked.ToUInt64(value));
    public ulong ToUInt64(IFormatProvider provider) => NumericUnchecked.ToUInt64(value);
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(NumericUnchecked.ToUInt64(value));
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(NumericUnchecked.ToUInt64(value));
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(NumericUnchecked.ToUInt64(value));
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(NumericUnchecked.ToUInt64(value));
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(NumericUnchecked.ToUInt64(value));
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(NumericUnchecked.ToUInt64(value));
    public float ToSingle(IFormatProvider provider) => (float) value;
    public double ToDouble(IFormatProvider provider) => (double) value;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromInt64(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Char",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Char");
    public override string ToString() => CharRuntimeIntrinsics.chic_rt_string_from_char(value);
    public int CompareTo(Char other) {
        if (value <other.value)
        {
            return - 1;
        }
        if (value >other.value)
        {
            return 1;
        }
        return 0;
    }
    public int CompareTo(char other) {
        return CompareTo(new Char(other));
    }
    public int CompareTo(Object other) => throw new Std.ArgumentException("object is not a Char");
    public static int Compare(Char left, Char right) {
        return left.CompareTo(right);
    }
    public static bool operator == (Char left, Char right) => left.value == right.value;
    public static bool operator != (Char left, Char right) => left.value != right.value;
    public static bool operator <(Char left, Char right) => left.value <right.value;
    public static bool operator <= (Char left, Char right) => left.value <= right.value;
    public static bool operator >(Char left, Char right) => left.value >right.value;
    public static bool operator >= (Char left, Char right) => left.value >= right.value;
    public override bool Equals(Object other) => false;
    public static bool Equals(Char left, Char right) => left.value == right.value;
    public bool Equals(Char other) => value == other.value;
    public bool Equals(char other) => value == other;
    public override int GetHashCode() => (int) value;
    public static Char operator + (Char left, Char right) => new Char(NumericUnchecked.ToChar((int) left.value + (int) right.value));
    public static Char operator - (Char left, Char right) => new Char(NumericUnchecked.ToChar((int) left.value - (int) right.value));
    public static Char operator * (Char left, Char right) => new Char(NumericUnchecked.ToChar((int) left.value * (int) right.value));
    public static Char operator / (Char left, Char right) => new Char(NumericUnchecked.ToChar((int) left.value / (int) right.value));
    public static Char operator % (Char left, Char right) => new Char(NumericUnchecked.ToChar((int) left.value % (int) right.value));
    public static Char operator + (Char value) => value;
    public static Char operator - (Char value) => new Char(NumericUnchecked.ToChar(- (int)(ushort) value.value));
    public static Char operator ++ (Char value) => new Char(NumericUnchecked.ToChar((int) value.value + 1));
    public static Char operator -- (Char value) => new Char(NumericUnchecked.ToChar((int) value.value - 1));
    public static Char operator & (Char left, Char right) => new Char((char)((ushort) left.value & (ushort) right.value));
    public static Char operator | (Char left, Char right) => new Char((char)((ushort) left.value | (ushort) right.value));
    public static Char operator ^ (Char left, Char right) => new Char((char)((ushort) left.value ^ (ushort) right.value));
    public static Char operator ~ (Char value) => new Char((char)(((ushort) value.value) ^ 0xFFFFu16));
    public static Char operator << (Char value, int offset) {
        let masked = offset & 0x0F;
        return new Char((char)(((ushort) value.value) << masked));
    }
    public static Char operator >> (Char value, int offset) {
        let masked = offset & 0x0F;
        return new Char((char)(((ushort) value.value) >> masked));
    }
    public static Char Min(Char left, Char right) => left.value <= right.value ?left : right;
    public static Char Max(Char left, Char right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Char left, Char right, out Char result) {
        var raw = (ushort) 0;
        if (!NumericArithmetic.TryAddUInt16 ( (ushort) left.value, (ushort) right.value, out raw)) {
            result = Zero;
            return false;
        }
        result = new Char((char) raw);
        return true;
    }
    public static bool TrySubtract(Char left, Char right, out Char result) {
        var raw = (ushort) 0;
        if (!NumericArithmetic.TrySubtractUInt16 ( (ushort) left.value, (ushort) right.value, out raw)) {
            result = Zero;
            return false;
        }
        result = new Char((char) raw);
        return true;
    }
    public static bool TryMultiply(Char left, Char right, out Char result) {
        var raw = (ushort) 0;
        if (!NumericArithmetic.TryMultiplyUInt16 ( (ushort) left.value, (ushort) right.value, out raw)) {
            result = Zero;
            return false;
        }
        result = new Char((char) raw);
        return true;
    }
    public static int LeadingZeroCount(Char value) {
        return NumericBitOperations.LeadingZeroCountUInt16((ushort) value.value);
    }
    public static int TrailingZeroCount(Char value) {
        return NumericBitOperations.TrailingZeroCountUInt16((ushort) value.value);
    }
    public static int PopCount(Char value) {
        return NumericBitOperations.PopCountUInt16((ushort) value.value);
    }
    public static Char RotateLeft(Char value, int offset) {
        return new Char(NumericBitOperations.RotateLeftUInt16((ushort) value.value, offset));
    }
    public static Char RotateRight(Char value, int offset) {
        return new Char(NumericBitOperations.RotateRightUInt16((ushort) value.value, offset));
    }
    public static Char ReverseEndianness(Char value) {
        return new Char(NumericBitOperations.ReverseEndiannessUInt16((ushort) value.value));
    }
    public static bool IsPowerOfTwo(Char value) {
        return NumericBitOperations.IsPowerOfTwoUInt16((ushort) value.value);
    }
    public static Char Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var chars = ReadOnlySpan.FromStringChars(text);
        return Parse(chars);
    }
    public static bool TryParse(string text, out Char result) {
        if (text == null)
        {
            result = Zero;
            return false;
        }
        var chars = ReadOnlySpan.FromStringChars(text);
        return TryParse(chars, out result);
    }
    public static Char Parse(ReadOnlySpan <char >text) {
        if (TryParse (text, out var value)) {
            return value;
        }
        throw new Std.FormatException("Input string was not in a correct format.");
    }
    public static bool TryParse(ReadOnlySpan <char >text, out Char result) {
        if (text.Length != 1)
        {
            result = Zero;
            return false;
        }
        result = new Char(text[0]);
        return true;
    }
    public static Char Parse(ReadOnlySpan <byte >text) {
        var decoded = Utf8String.FromSpan(text);
        return Parse(decoded);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Char result) {
        try {
            var decoded = Utf8String.FromSpan(text);
            return TryParse(decoded, out result);
        }
        catch(Std.Exception) {
            result = Zero;
            return false;
        }
        result = Zero;
        return false;
    }
    public static bool IsScalar(char value) {
        return CharRuntimeIntrinsics.chic_rt_char_is_scalar(value) == 1;
    }
    public static bool IsDigit(char value) {
        return CharRuntimeIntrinsics.chic_rt_char_is_digit(value) == 1;
    }
    public static bool IsLetter(char value) {
        return CharRuntimeIntrinsics.chic_rt_char_is_letter(value) == 1;
    }
    public static bool IsWhiteSpace(char value) {
        return CharRuntimeIntrinsics.chic_rt_char_is_whitespace(value) == 1;
    }
    public static bool TryToUpperInvariant(char value, out Char result) {
        var raw = CharRuntimeIntrinsics.chic_rt_char_to_upper(value);
        if (CharRuntimeIntrinsics.chic_rt_char_status (raw) == 0)
        {
            result = new Char(CharRuntimeIntrinsics.chic_rt_char_value(raw));
            return true;
        }
        result = new Char(value);
        return false;
    }
    public static bool TryToLowerInvariant(char value, out Char result) {
        var raw = CharRuntimeIntrinsics.chic_rt_char_to_lower(value);
        if (CharRuntimeIntrinsics.chic_rt_char_status (raw) == 0)
        {
            result = new Char(CharRuntimeIntrinsics.chic_rt_char_value(raw));
            return true;
        }
        result = new Char(value);
        return false;
    }
    public static bool TryFromCodePoint(uint codePoint, out Char result) {
        var raw = CharRuntimeIntrinsics.chic_rt_char_from_codepoint(codePoint);
        if (CharRuntimeIntrinsics.chic_rt_char_status (raw) == 0)
        {
            result = new Char(CharRuntimeIntrinsics.chic_rt_char_value(raw));
            return true;
        }
        result = Zero;
        return false;
    }
    public static Char ToUpperInvariant(Char value) {
        return TryToUpperInvariant(value.value, out var result) ?result : value;
    }
    public static Char ToLowerInvariant(Char value) {
        return TryToLowerInvariant(value.value, out var result) ?result : value;
    }
}
