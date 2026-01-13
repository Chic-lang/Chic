namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "short", kind = "int", bits = 16, signed = true, aliases = ["short",
"Int16", "Std.Int16", "Std.Numeric.Int16", "System.Int16", "int16", "i16"], c_type = "int16_t") public readonly struct Int16 : IComparable, IComparable <short >, IConvertible, IEquatable <short >, IParsable <short >, ISpanParsable <short >, IUtf8SpanParsable <short >, IAdditionOperators <short, short, short >, IAdditiveIdentity <short, short >, IBinaryInteger <short >, IBinaryNumber <short >, IBitwiseOperators <short, short, short >, IComparisonOperators <short, short, bool >, IDecrementOperators <short >, IDivisionOperators <short, short, short >, IEqualityOperators <short, short, bool >, IIncrementOperators <short >, IMinMaxValue <short >, IModulusOperators <short, short, short >, IMultiplicativeIdentity <short, short >, IMultiplyOperators <short, short, short >, INumber <short >, INumberBase <short >, IShiftOperators <short, int, short >, ISignedNumber <short >, ISubtractionOperators <short, short, short >, IUnaryNegationOperators <short, short >, IUnaryPlusOperators <short, short >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly short value;
    public const short MinValue = - 32768;
    public const short MaxValue = 32767;
    public init(short value) {
        this.value = value;
    }
    public short ToInt16() => value;
    public static Int16 From(short value) => new Int16(value);
    public static Int16 Zero => new Int16(0);
    public static Int16 One => new Int16(1);
    public static Int16 NegativeOne() => new Int16(- 1);
    public static Int16 AdditiveIdentity() => Zero;
    public static Int16 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatInt16(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value);
    public short ToInt16(IFormatProvider provider) => value;
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(value);
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(value);
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(value);
    public long ToInt64(IFormatProvider provider) => value;
    public ulong ToUInt64(IFormatProvider provider) => ConvertibleHelpers.ToUInt64Checked(value);
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(value);
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(value);
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(value);
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(value);
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(value);
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(value);
    public float ToSingle(IFormatProvider provider) => (float) value;
    public double ToDouble(IFormatProvider provider) => (double) value;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromInt64(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Int16",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Int16");
    public string Format(string format, string culture) => NumericFormatting.FormatInt16(value, format, culture);
    public static Int16 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out Int16 result) {
        var parsed = 0;
        if (! NumericParse.TryParseInt16 (text, out parsed)) {
            result = new Int16(0);
            return false;
        }
        result = new Int16(parsed);
        return true;
    }
    public static Int16 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseInt32 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Int16");
        }
        if (parsed <NumericConstants.Int16Min || parsed >NumericConstants.Int16Max)
        {
            NumericParse.ThrowParseException(ParseStatus.Overflow, "Int16");
        }
        return new Int16(NumericUnchecked.ToInt16(parsed));
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Int16 result) {
        var parsed = 0;
        if (! NumericParse.TryParseInt16 (text, out parsed)) {
            result = new Int16(0);
            return false;
        }
        result = new Int16(parsed);
        return true;
    }
    public int CompareTo(Int16 other) {
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
    public int CompareTo(Object other) {
        throw new Std.ArgumentException("object is not an Int16");
    }
    public static int Compare(Int16 left, Int16 right) {
        return left.CompareTo(right);
    }
    public static bool operator == (Int16 left, Int16 right) => left.value == right.value;
    public static bool operator != (Int16 left, Int16 right) => left.value != right.value;
    public static bool operator <(Int16 left, Int16 right) => left.value <right.value;
    public static bool operator <= (Int16 left, Int16 right) => left.value <= right.value;
    public static bool operator >(Int16 left, Int16 right) => left.value >right.value;
    public static bool operator >= (Int16 left, Int16 right) => left.value >= right.value;
    public static Int16 operator + (Int16 left, Int16 right) => new Int16(NumericUnchecked.ToInt16(left.value + right.value));
    public static Int16 operator - (Int16 left, Int16 right) => new Int16(NumericUnchecked.ToInt16(left.value - right.value));
    public static Int16 operator * (Int16 left, Int16 right) => new Int16(NumericUnchecked.ToInt16(left.value * right.value));
    public static Int16 operator / (Int16 left, Int16 right) => new Int16(NumericUnchecked.ToInt16(left.value / right.value));
    public static Int16 operator % (Int16 left, Int16 right) => new Int16(NumericUnchecked.ToInt16(left.value % right.value));
    public static Int16 operator - (Int16 value) => new Int16(NumericUnchecked.ToInt16(- value.value));
    public static Int16 operator + (Int16 value) => value;
    public static Int16 operator ++ (Int16 value) => new Int16(NumericUnchecked.ToInt16(value.value + 1));
    public static Int16 operator -- (Int16 value) => new Int16(NumericUnchecked.ToInt16(value.value - 1));
    public static Int16 operator & (Int16 left, Int16 right) => new Int16((short)(left.value & right.value));
    public static Int16 operator | (Int16 left, Int16 right) => new Int16((short)(left.value | right.value));
    public static Int16 operator ^ (Int16 left, Int16 right) => new Int16((short)(left.value ^ right.value));
    public static Int16 operator ~ (Int16 value) => new Int16((short)(value.value ^ - 1));
    public static Int16 operator << (Int16 value, int offset) => new Int16((short)(value.value << offset));
    public static Int16 operator >> (Int16 value, int offset) => new Int16((short)(value.value >> offset));
    public static Int16 Abs(Int16 value) => value.value <0 ?new Int16(NumericUnchecked.ToInt16(- value.value)) : value;
    public static bool Equals(Int16 left, Int16 right) => left.value == right.value;
    public bool Equals(Int16 other) => value == other.value;
    public static Int16 Min(Int16 left, Int16 right) => left.value <= right.value ?left : right;
    public static Int16 Max(Int16 left, Int16 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Int16 left, Int16 right, out Int16 result) {
        var raw = 0;
        if (! NumericArithmetic.TryAddInt16 (left.value, right.value, out raw)) {
            result = new Int16(0);
            return false;
        }
        result = new Int16(raw);
        return true;
    }
    public static bool TrySubtract(Int16 left, Int16 right, out Int16 result) {
        var raw = 0;
        if (! NumericArithmetic.TrySubtractInt16 (left.value, right.value, out raw)) {
            result = new Int16(0);
            return false;
        }
        result = new Int16(raw);
        return true;
    }
    public static bool TryMultiply(Int16 left, Int16 right, out Int16 result) {
        var raw = 0;
        if (! NumericArithmetic.TryMultiplyInt16 (left.value, right.value, out raw)) {
            result = new Int16(0);
            return false;
        }
        result = new Int16(raw);
        return true;
    }
    public static bool TryNegate(Int16 value, out Int16 result) {
        var raw = 0;
        if (! NumericArithmetic.TryNegateInt16 (value.value, out raw)) {
            result = new Int16(0);
            return false;
        }
        result = new Int16(raw);
        return true;
    }
    public static int LeadingZeroCount(Int16 value) {
        return NumericBitOperations.LeadingZeroCountInt16(value.value);
    }
    public static int TrailingZeroCount(Int16 value) {
        return NumericBitOperations.TrailingZeroCountInt16(value.value);
    }
    public static int PopCount(Int16 value) {
        return NumericBitOperations.PopCountInt16(value.value);
    }
    public static Int16 RotateLeft(Int16 value, int offset) {
        return new Int16(NumericBitOperations.RotateLeftInt16(value.value, offset));
    }
    public static Int16 RotateRight(Int16 value, int offset) {
        return new Int16(NumericBitOperations.RotateRightInt16(value.value, offset));
    }
    public static Int16 ReverseEndianness(Int16 value) {
        return new Int16(NumericBitOperations.ReverseEndiannessInt16(value.value));
    }
    public static bool IsPowerOfTwo(Int16 value) {
        return NumericBitOperations.IsPowerOfTwoInt16(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatInt16(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatInt16(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatInt16(value, destination, out written, format, culture);
    }
}
