namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "long", kind = "int", bits = 64, signed = true, aliases = ["long",
"Int64", "Std.Int64", "Std.Numeric.Int64", "System.Int64", "int64", "i64"], c_type = "int64_t") public readonly struct Int64 : IComparable, IComparable <long >, IConvertible, IEquatable <long >, IParsable <long >, ISpanParsable <long >, IUtf8SpanParsable <long >, IAdditionOperators <long, long, long >, IAdditiveIdentity <long, long >, IBinaryInteger <long >, IBinaryNumber <long >, IBitwiseOperators <long, long, long >, IComparisonOperators <long, long, bool >, IDecrementOperators <long >, IDivisionOperators <long, long, long >, IEqualityOperators <long, long, bool >, IIncrementOperators <long >, IMinMaxValue <long >, IModulusOperators <long, long, long >, IMultiplicativeIdentity <long, long >, IMultiplyOperators <long, long, long >, INumber <long >, INumberBase <long >, IShiftOperators <long, int, long >, ISignedNumber <long >, ISubtractionOperators <long, long, long >, IUnaryNegationOperators <long, long >, IUnaryPlusOperators <long, long >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly long value;
    public const long MinValue = - 9223372036854775807L - 1L;
    public const long MaxValue = 9223372036854775807L;
    public init(long value) {
        this.value = value;
    }
    public long ToInt64() => value;
    public static Int64 From(long value) => new Int64(value);
    public static Int64 Zero => new Int64(0L);
    public static Int64 One => new Int64(1L);
    public static Int64 NegativeOne() => new Int64(- 1L);
    public static Int64 AdditiveIdentity() => Zero;
    public static Int64 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatInt64(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(value);
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
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Int64",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Int64");
    public string Format(string format, string culture) => NumericFormatting.FormatInt64(value, format, culture);
    public static Int64 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out Int64 result) {
        var parsed = 0L;
        if (!NumericParse.TryParseInt64 (text, out parsed)) {
            result = new Int64(0L);
            return false;
        }
        result = new Int64(parsed);
        return true;
    }
    public static Int64 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseInt64 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Int64");
        }
        return new Int64(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Int64 result) {
        var parsed = 0L;
        if (!NumericParse.TryParseInt64 (text, out parsed)) {
            result = new Int64(0L);
            return false;
        }
        result = new Int64(parsed);
        return true;
    }
    public int CompareTo(Int64 other) {
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
        throw new Std.ArgumentException("object is not an Int64");
    }
    public static int Compare(Int64 left, Int64 right) {
        return left.CompareTo(right);
    }
    public static bool operator == (Int64 left, Int64 right) => left.value == right.value;
    public static bool operator != (Int64 left, Int64 right) => left.value != right.value;
    public static bool operator <(Int64 left, Int64 right) => left.value <right.value;
    public static bool operator <= (Int64 left, Int64 right) => left.value <= right.value;
    public static bool operator >(Int64 left, Int64 right) => left.value >right.value;
    public static bool operator >= (Int64 left, Int64 right) => left.value >= right.value;
    public static Int64 operator + (Int64 left, Int64 right) => new Int64(left.value + right.value);
    public static Int64 operator - (Int64 left, Int64 right) => new Int64(left.value - right.value);
    public static Int64 operator * (Int64 left, Int64 right) => new Int64(left.value * right.value);
    public static Int64 operator / (Int64 left, Int64 right) => new Int64(left.value / right.value);
    public static Int64 operator % (Int64 left, Int64 right) => new Int64(left.value % right.value);
    public static Int64 operator - (Int64 value) => new Int64(- value.value);
    public static Int64 operator + (Int64 value) => value;
    public static Int64 operator ++ (Int64 value) => new Int64(value.value + 1L);
    public static Int64 operator -- (Int64 value) => new Int64(value.value - 1L);
    public static Int64 operator & (Int64 left, Int64 right) => new Int64(left.value & right.value);
    public static Int64 operator | (Int64 left, Int64 right) => new Int64(left.value | right.value);
    public static Int64 operator ^ (Int64 left, Int64 right) => new Int64(left.value ^ right.value);
    public static Int64 operator ~ (Int64 value) => new Int64(value.value ^ - 1L);
    public static Int64 operator << (Int64 value, int offset) => new Int64(value.value << offset);
    public static Int64 operator >> (Int64 value, int offset) => new Int64(value.value >> offset);
    public static Int64 Abs(Int64 value) => value.value <0 ?new Int64(- value.value) : value;
    public static bool Equals(Int64 left, Int64 right) => left.value == right.value;
    public bool Equals(Int64 other) => value == other.value;
    public static Int64 Min(Int64 left, Int64 right) => left.value <= right.value ?left : right;
    public static Int64 Max(Int64 left, Int64 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Int64 left, Int64 right, out Int64 result) {
        var raw = 0L;
        if (!NumericArithmetic.TryAddInt64 (left.value, right.value, out raw)) {
            result = new Int64(0L);
            return false;
        }
        result = new Int64(raw);
        return true;
    }
    public static bool TrySubtract(Int64 left, Int64 right, out Int64 result) {
        var raw = 0L;
        if (!NumericArithmetic.TrySubtractInt64 (left.value, right.value, out raw)) {
            result = new Int64(0L);
            return false;
        }
        result = new Int64(raw);
        return true;
    }
    public static bool TryMultiply(Int64 left, Int64 right, out Int64 result) {
        var raw = 0L;
        if (!NumericArithmetic.TryMultiplyInt64 (left.value, right.value, out raw)) {
            result = new Int64(0L);
            return false;
        }
        result = new Int64(raw);
        return true;
    }
    public static bool TryNegate(Int64 value, out Int64 result) {
        var raw = 0L;
        if (!NumericArithmetic.TryNegateInt64 (value.value, out raw)) {
            result = new Int64(0L);
            return false;
        }
        result = new Int64(raw);
        return true;
    }
    public static int LeadingZeroCount(Int64 value) {
        return NumericBitOperations.LeadingZeroCountInt64(value.value);
    }
    public static int TrailingZeroCount(Int64 value) {
        return NumericBitOperations.TrailingZeroCountInt64(value.value);
    }
    public static int PopCount(Int64 value) {
        return NumericBitOperations.PopCountInt64(value.value);
    }
    public static Int64 RotateLeft(Int64 value, int offset) {
        return new Int64(NumericBitOperations.RotateLeftInt64(value.value, offset));
    }
    public static Int64 RotateRight(Int64 value, int offset) {
        return new Int64(NumericBitOperations.RotateRightInt64(value.value, offset));
    }
    public static Int64 ReverseEndianness(Int64 value) {
        return new Int64(NumericBitOperations.ReverseEndiannessInt64(value.value));
    }
    public static bool IsPowerOfTwo(Int64 value) {
        return NumericBitOperations.IsPowerOfTwoInt64(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatInt64(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatInt64(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatInt64(value, destination, out written, format, culture);
    }
}
