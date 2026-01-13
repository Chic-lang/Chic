namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "int128", kind = "int", bits = 128, signed = true,
aliases = ["int128", "Int128", "Std.Int128", "Std.Numeric.Int128", "System.Int128", "i128"], c_type = "int128_t") public readonly struct Int128 : IComparable, IComparable <Int128 >, IConvertible, IEquatable <Int128 >, IParsable <Int128 >, ISpanParsable <Int128 >, IUtf8SpanParsable <Int128 >, IAdditionOperators <Int128, Int128, Int128 >, IAdditiveIdentity <Int128, Int128 >, IBinaryInteger <Int128 >, IBinaryNumber <Int128 >, IBitwiseOperators <Int128, Int128, Int128 >, IComparisonOperators <Int128, Int128, bool >, IDecrementOperators <Int128 >, IDivisionOperators <Int128, Int128, Int128 >, IEqualityOperators <Int128, Int128, bool >, IIncrementOperators <Int128 >, IMinMaxValue <Int128 >, IModulusOperators <Int128, Int128, Int128 >, IMultiplicativeIdentity <Int128, Int128 >, IMultiplyOperators <Int128, Int128, Int128 >, INumber <Int128 >, INumberBase <Int128 >, IShiftOperators <Int128, int, Int128 >, ISignedNumber <Int128 >, ISubtractionOperators <Int128, Int128, Int128 >, IUnaryNegationOperators <Int128, Int128 >, IUnaryPlusOperators <Int128, Int128 >, IFormattable, ISpanFormattable, IUtf8SpanFormattable
{
    private readonly int128 value;
    public const int128 MinValue = Std.Numeric.NumericConstants.Int128Min;
    public const int128 MaxValue = Std.Numeric.NumericConstants.Int128Max;
    public init(int128 value) {
        this.value = value;
    }
    public init(u128 value) {
        this.value = ConvertibleHelpers.ToInt128Checked(value);
    }
    public init(float value) {
        this.value = ConvertibleHelpers.ToInt128Checked((double) value);
    }
    public init(double value) {
        this.value = ConvertibleHelpers.ToInt128Checked(value);
    }
    public init(Decimal value) {
        this.value = ConvertibleHelpers.ToInt128Checked(value);
    }
    public int128 ToInt128() => value;
    public static Int128 From(int128 value) => new Int128(value);
    public static Int128 From(u128 value) => new Int128(value);
    public static Int128 From(float value) => new Int128(value);
    public static Int128 From(double value) => new Int128(value);
    public static Int128 From(Decimal value) => new Int128(value);
    public static Int128 Zero => new Int128(0);
    public static Int128 One => new Int128(1);
    public static Int128 NegativeOne() => new Int128(- 1);
    public static Int128 AdditiveIdentity() => Zero;
    public static Int128 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatInt128(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromInt128(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(value);
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(value);
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(value);
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(value);
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked(value);
    public ulong ToUInt64(IFormatProvider provider) => ConvertibleHelpers.ToUInt64Checked(value);
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(ConvertibleHelpers.ToInt64Checked(value));
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(ConvertibleHelpers.ToUInt64Checked(value));
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(ConvertibleHelpers.ToInt64Checked(value));
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(ConvertibleHelpers.ToUInt64Checked(value));
    public Int128 ToInt128(IFormatProvider provider) => value;
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(value);
    public float ToSingle(IFormatProvider provider) => (float) value;
    public double ToDouble(IFormatProvider provider) => (double) value;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromInt128(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Int128",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Int128");
    public string Format(string format, string culture) => NumericFormatting.FormatInt128(value, format, culture);
    public static Int128 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out Int128 result) {
        var parsed = 0i128;
        if (! NumericParse.TryParseInt128 (text, out parsed)) {
            result = new Int128(0);
            return false;
        }
        result = new Int128(parsed);
        return true;
    }
    public static Int128 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseInt128 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Int128");
        }
        return new Int128(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Int128 result) {
        var parsed = 0i128;
        if (! NumericParse.TryParseInt128 (text, out parsed)) {
            result = new Int128(0);
            return false;
        }
        result = new Int128(parsed);
        return true;
    }
    public int CompareTo(Int128 other) {
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
        throw new Std.ArgumentException("object is not an Int128");
    }
    public static int Compare(Int128 left, Int128 right) {
        return left.CompareTo(right);
    }
    public static int Sign(Int128 value) {
        if (value.value <0)
        {
            return - 1;
        }
        if (value.value >0)
        {
            return 1;
        }
        return 0;
    }
    public static bool operator == (Int128 left, Int128 right) => left.value == right.value;
    public static bool operator != (Int128 left, Int128 right) => left.value != right.value;
    public static bool operator <(Int128 left, Int128 right) => left.value <right.value;
    public static bool operator <= (Int128 left, Int128 right) => left.value <= right.value;
    public static bool operator >(Int128 left, Int128 right) => left.value >right.value;
    public static bool operator >= (Int128 left, Int128 right) => left.value >= right.value;
    public static Int128 operator + (Int128 left, Int128 right) => new Int128(left.value + right.value);
    public static Int128 operator - (Int128 left, Int128 right) => new Int128(left.value - right.value);
    public static Int128 operator * (Int128 left, Int128 right) => new Int128(left.value * right.value);
    public static Int128 operator / (Int128 left, Int128 right) => new Int128(left.value / right.value);
    public static Int128 operator % (Int128 left, Int128 right) => new Int128(left.value % right.value);
    public static Int128 operator - (Int128 value) => new Int128(- value.value);
    public static Int128 operator + (Int128 value) => value;
    public static Int128 operator ++ (Int128 value) => new Int128(value.value + 1L);
    public static Int128 operator -- (Int128 value) => new Int128(value.value - 1L);
    public static Int128 operator & (Int128 left, Int128 right) => new Int128(left.value & right.value);
    public static Int128 operator | (Int128 left, Int128 right) => new Int128(left.value | right.value);
    public static Int128 operator ^ (Int128 left, Int128 right) => new Int128(left.value ^ right.value);
    public static Int128 operator ~ (Int128 value) => new Int128(value.value ^ - 1L);
    public static Int128 operator << (Int128 value, int offset) => new Int128(value.value << offset);
    public static Int128 operator >> (Int128 value, int offset) => new Int128(value.value >> offset);
    public static Int128 Abs(Int128 value) {
        if (value.value >= 0)
        {
            return value;
        }
        if (value.value == MinValue)
        {
            return value;
        }
        return new Int128(- value.value);
    }
    public static bool Equals(Int128 left, Int128 right) => left.value == right.value;
    public bool Equals(Int128 other) => value == other.value;
    public static Int128 Min(Int128 left, Int128 right) => left.value <= right.value ?left : right;
    public static Int128 Max(Int128 left, Int128 right) => left.value >= right.value ?left : right;
    public static Int128 MaxMagnitude(Int128 left, Int128 right) {
        let absLeft = AbsMagnitude(left);
        let absRight = AbsMagnitude(right);
        return absLeft.value >absRight.value ?left : right;
    }
    public static Int128 MinMagnitude(Int128 left, Int128 right) {
        let absLeft = AbsMagnitude(left);
        let absRight = AbsMagnitude(right);
        return absLeft.value <absRight.value ?left : right;
    }
    public static Int128 Clamp(Int128 value, Int128 min, Int128 max) {
        if (min.value >max.value)
        {
            throw new Std.ArgumentException("min cannot be greater than max");
        }
        if (value.value <min.value)
        {
            return min;
        }
        if (value.value >max.value)
        {
            return max;
        }
        return value;
    }
    public static Int128 CopySign(Int128 value, Int128 sign) {
        if ( (value.value <0 && sign.value <0) || (value.value >= 0 && sign.value >= 0))
        {
            return value;
        }
        if (value.value == MinValue)
        {
            return value;
        }
        return new Int128(- value.value);
    }
    public static Int128 DivRem(Int128 left, Int128 right, out Int128 remainder) {
        remainder = new Int128(left.value % right.value);
        return new Int128(left.value / right.value);
    }
    private static Int128 AbsMagnitude(Int128 value) {
        if (value.value >= 0)
        {
            return value;
        }
        if (value.value == MinValue)
        {
            return new Int128(MinValue);
        }
        return new Int128(- value.value);
    }
    public static bool TryAdd(Int128 left, Int128 right, out Int128 result) {
        var raw = 0i128;
        if (! NumericArithmetic.TryAddInt128 (left.value, right.value, out raw)) {
            result = new Int128(0);
            return false;
        }
        result = new Int128(raw);
        return true;
    }
    public static bool TrySubtract(Int128 left, Int128 right, out Int128 result) {
        var raw = 0i128;
        if (! NumericArithmetic.TrySubtractInt128 (left.value, right.value, out raw)) {
            result = new Int128(0);
            return false;
        }
        result = new Int128(raw);
        return true;
    }
    public static bool TryMultiply(Int128 left, Int128 right, out Int128 result) {
        var raw = 0i128;
        if (! NumericArithmetic.TryMultiplyInt128 (left.value, right.value, out raw)) {
            result = new Int128(0);
            return false;
        }
        result = new Int128(raw);
        return true;
    }
    public static Int128 ReverseEndianness(Int128 value) {
        return new Int128(NumericBitOperations.ReverseEndiannessInt128(value.value));
    }
    public static bool IsPowerOfTwo(Int128 value) {
        return NumericBitOperations.IsPowerOfTwoInt128(value.value);
    }
    public static int LeadingZeroCount(Int128 value) {
        return NumericBitOperations.LeadingZeroCountInt128(value.value);
    }
    public static int TrailingZeroCount(Int128 value) {
        return NumericBitOperations.TrailingZeroCountInt128(value.value);
    }
    public static int PopCount(Int128 value) {
        return NumericBitOperations.PopCountInt128(value.value);
    }
    public static Int128 RotateLeft(Int128 value, int offset) {
        return new Int128(NumericBitOperations.RotateLeftInt128(value.value, offset));
    }
    public static Int128 RotateRight(Int128 value, int offset) {
        return new Int128(NumericBitOperations.RotateRightInt128(value.value, offset));
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatInt128(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatInt128(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatInt128(value, destination, out written, format, culture);
    }
}
