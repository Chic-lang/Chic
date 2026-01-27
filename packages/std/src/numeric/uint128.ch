namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "uint128", kind = "int", bits = 128, signed = false,
aliases = ["uint128", "UInt128", "Std.UInt128", "Std.Numeric.UInt128", "System.UInt128", "u128"], c_type = "uint128_t") public readonly struct UInt128 : IComparable, IComparable <UInt128 >, IConvertible, IEquatable <UInt128 >, IParsable <UInt128 >, ISpanParsable <UInt128 >, IUtf8SpanParsable <UInt128 >, IAdditionOperators <UInt128, UInt128, UInt128 >, IAdditiveIdentity <UInt128, UInt128 >, IBinaryInteger <UInt128 >, IBinaryNumber <UInt128 >, IBitwiseOperators <UInt128, UInt128, UInt128 >, IComparisonOperators <UInt128, UInt128, bool >, IDecrementOperators <UInt128 >, IDivisionOperators <UInt128, UInt128, UInt128 >, IEqualityOperators <UInt128, UInt128, bool >, IIncrementOperators <UInt128 >, IMinMaxValue <UInt128 >, IModulusOperators <UInt128, UInt128, UInt128 >, IMultiplicativeIdentity <UInt128, UInt128 >, IMultiplyOperators <UInt128, UInt128, UInt128 >, INumber <UInt128 >, INumberBase <UInt128 >, IShiftOperators <UInt128, int, UInt128 >, ISubtractionOperators <UInt128, UInt128, UInt128 >, IUnaryPlusOperators <UInt128, UInt128 >, IFormattable, ISpanFormattable, IUtf8SpanFormattable
{
    private readonly u128 value;
    public const u128 MinValue = 0u128;
    public const u128 MaxValue = Std.Numeric.NumericConstants.UInt128Max;
    public init(u128 value) {
        this.value = value;
    }
    public init(float value) {
        this.value = ConvertibleHelpers.ToUInt128Checked((double) value);
    }
    public init(double value) {
        this.value = ConvertibleHelpers.ToUInt128Checked(value);
    }
    public init(Decimal value) {
        this.value = ConvertibleHelpers.ToUInt128Checked(value);
    }
    public init(Int128 value) {
        this.value = ConvertibleHelpers.ToUInt128Checked(value.ToInt128());
    }
    public u128 ToUInt128() => value;
    public static UInt128 From(u128 value) => new UInt128(value);
    public static UInt128 From(float value) => new UInt128(value);
    public static UInt128 From(double value) => new UInt128(value);
    public static UInt128 From(Decimal value) => new UInt128(value);
    public static UInt128 From(Int128 value) => new UInt128(value);
    public static UInt128 Zero => new UInt128(0u128);
    public static UInt128 One => new UInt128(1u128);
    public static UInt128 AdditiveIdentity() => Zero;
    public static UInt128 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatUInt128(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromUInt128(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(value);
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(value);
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(value);
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(value);
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked(value);
    public ulong ToUInt64(IFormatProvider provider) => ConvertibleHelpers.ToUInt64Checked(value);
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(value);
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(value);
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(value);
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(value);
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(value);
    public UInt128 ToUInt128(IFormatProvider provider) => value;
    public float ToSingle(IFormatProvider provider) => (float) value;
    public double ToDouble(IFormatProvider provider) => (double) value;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromUInt128(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("UInt128",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("UInt128");
    public string Format(string format, string culture) => NumericFormatting.FormatUInt128(value, format, culture);
    public static UInt128 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out UInt128 result) {
        var parsed = 0u128;
        if (! NumericParse.TryParseUInt128 (text, out parsed)) {
            result = new UInt128(0u128);
            return false;
        }
        result = new UInt128(parsed);
        return true;
    }
    public static UInt128 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseUInt128 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "UInt128");
        }
        return new UInt128(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out UInt128 result) {
        var parsed = 0u128;
        if (! NumericParse.TryParseUInt128 (text, out parsed)) {
            result = new UInt128(0u128);
            return false;
        }
        result = new UInt128(parsed);
        return true;
    }
    public int CompareTo(UInt128 other) {
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
        throw new Std.ArgumentException("object is not a UInt128");
    }
    public static bool operator == (UInt128 left, UInt128 right) => left.value == right.value;
    public static bool operator != (UInt128 left, UInt128 right) => left.value != right.value;
    public static bool operator <(UInt128 left, UInt128 right) => left.value <right.value;
    public static bool operator <= (UInt128 left, UInt128 right) => left.value <= right.value;
    public static bool operator >(UInt128 left, UInt128 right) => left.value >right.value;
    public static bool operator >= (UInt128 left, UInt128 right) => left.value >= right.value;
    public static UInt128 operator + (UInt128 left, UInt128 right) => new UInt128(left.value + right.value);
    public static UInt128 operator - (UInt128 left, UInt128 right) => new UInt128(left.value - right.value);
    public static UInt128 operator * (UInt128 left, UInt128 right) => new UInt128(left.value * right.value);
    public static UInt128 operator / (UInt128 left, UInt128 right) => new UInt128(left.value / right.value);
    public static UInt128 operator % (UInt128 left, UInt128 right) => new UInt128(left.value % right.value);
    public static UInt128 operator + (UInt128 value) => value;
    public static UInt128 operator ++ (UInt128 value) => new UInt128(value.value + 1ul);
    public static UInt128 operator -- (UInt128 value) => new UInt128(value.value - 1ul);
    public static UInt128 operator & (UInt128 left, UInt128 right) => new UInt128(left.value & right.value);
    public static UInt128 operator | (UInt128 left, UInt128 right) => new UInt128(left.value | right.value);
    public static UInt128 operator ^ (UInt128 left, UInt128 right) => new UInt128(left.value ^ right.value);
    public static UInt128 operator ~ (UInt128 value) => new UInt128(value.value ^ Std.Numeric.NumericConstants.UInt128Max);
    public static UInt128 operator << (UInt128 value, int offset) => new UInt128(value.value << offset);
    public static UInt128 operator >> (UInt128 value, int offset) => new UInt128(value.value >> offset);
    public static bool Equals(UInt128 left, UInt128 right) => left.value == right.value;
    public bool Equals(UInt128 other) => value == other.value;
    public static UInt128 Min(UInt128 left, UInt128 right) => left.value <= right.value ?left : right;
    public static UInt128 Max(UInt128 left, UInt128 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(UInt128 left, UInt128 right, out UInt128 result) {
        var raw = 0u128;
        if (! NumericArithmetic.TryAddUInt128 (left.value, right.value, out raw)) {
            result = new UInt128(0u128);
            return false;
        }
        result = new UInt128(raw);
        return true;
    }
    public static bool TrySubtract(UInt128 left, UInt128 right, out UInt128 result) {
        var raw = 0u128;
        if (! NumericArithmetic.TrySubtractUInt128 (left.value, right.value, out raw)) {
            result = new UInt128(0u128);
            return false;
        }
        result = new UInt128(raw);
        return true;
    }
    public static bool TryMultiply(UInt128 left, UInt128 right, out UInt128 result) {
        var raw = 0u128;
        if (! NumericArithmetic.TryMultiplyUInt128 (left.value, right.value, out raw)) {
            result = new UInt128(0u128);
            return false;
        }
        result = new UInt128(raw);
        return true;
    }
    public static int LeadingZeroCount(UInt128 value) {
        return NumericBitOperations.LeadingZeroCountUInt128(value.value);
    }
    public static int TrailingZeroCount(UInt128 value) {
        return NumericBitOperations.TrailingZeroCountUInt128(value.value);
    }
    public static int PopCount(UInt128 value) {
        return NumericBitOperations.PopCountUInt128(value.value);
    }
    public static UInt128 RotateLeft(UInt128 value, int offset) {
        return new UInt128(NumericBitOperations.RotateLeftUInt128(value.value, offset));
    }
    public static UInt128 RotateRight(UInt128 value, int offset) {
        return new UInt128(NumericBitOperations.RotateRightUInt128(value.value, offset));
    }
    public static UInt128 ReverseEndianness(UInt128 value) {
        return new UInt128(NumericBitOperations.ReverseEndiannessUInt128(value.value));
    }
    public static bool IsPowerOfTwo(UInt128 value) {
        return NumericBitOperations.IsPowerOfTwoUInt128(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatUInt128(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatUInt128(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatUInt128(value, destination, out written, format, culture);
    }
}
