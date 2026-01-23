namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "int", kind = "int", bits = 32, signed = true, aliases = ["int",
"Int32", "Std.Int32", "Std.Numeric.Int32", "System.Int32", "int32", "i32"], c_type = "int32_t") public readonly struct Int32 : IComparable, IComparable <int >, IConvertible, IEquatable <int >, IParsable <int >, ISpanParsable <int >, IUtf8SpanParsable <int >, IAdditionOperators <int, int, int >, IAdditiveIdentity <int, int >, IBinaryInteger <int >, IBinaryNumber <int >, IBitwiseOperators <int, int, int >, IComparisonOperators <int, int, bool >, IDecrementOperators <int >, IDivisionOperators <int, int, int >, IEqualityOperators <int, int, bool >, IIncrementOperators <int >, IMinMaxValue <int >, IModulusOperators <int, int, int >, IMultiplicativeIdentity <int, int >, IMultiplyOperators <int, int, int >, INumber <int >, INumberBase <int >, IShiftOperators <int, int, int >, ISignedNumber <int >, ISubtractionOperators <int, int, int >, IUnaryNegationOperators <int, int >, IUnaryPlusOperators <int, int >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly int value;
    public const int MinValue = - 2147483647 - 1;
    public const int MaxValue = 2147483647;
    public init(int value) {
        this.value = value;
    }
    public int ToInt32() => value;
    public static Int32 From(int value) => new Int32(value);
    public static Int32 Zero => new Int32(0);
    public static Int32 One => new Int32(1);
    public static Int32 NegativeOne() => new Int32(- 1);
    public static Int32 AdditiveIdentity() => Zero;
    public static Int32 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatInt32(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(value);
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(value);
    public int ToInt32(IFormatProvider provider) => value;
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
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Int32",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Int32");
    public string Format(string format, string culture) => NumericFormatting.FormatInt32(value, format, culture);
    public static Int32 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out Int32 result) {
        var parsed = 0;
        if (!NumericParse.TryParseInt32 (text, out parsed)) {
            result = new Int32(0);
            return false;
        }
        result = new Int32(parsed);
        return true;
    }
    public static Int32 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseInt32 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Int32");
        }
        return new Int32(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Int32 result) {
        var parsed = 0;
        if (!NumericParse.TryParseInt32 (text, out parsed)) {
            result = new Int32(0);
            return false;
        }
        result = new Int32(parsed);
        return true;
    }
    public int CompareTo(Int32 other) {
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
        throw new Std.ArgumentException("object is not an Int32");
    }
    public static bool operator == (Int32 left, Int32 right) => left.value == right.value;
    public static bool operator != (Int32 left, Int32 right) => left.value != right.value;
    public static bool operator <(Int32 left, Int32 right) => left.value <right.value;
    public static bool operator <= (Int32 left, Int32 right) => left.value <= right.value;
    public static bool operator >(Int32 left, Int32 right) => left.value >right.value;
    public static bool operator >= (Int32 left, Int32 right) => left.value >= right.value;
    public static Int32 operator + (Int32 left, Int32 right) => new Int32(left.value + right.value);
    public static Int32 operator - (Int32 left, Int32 right) => new Int32(left.value - right.value);
    public static Int32 operator * (Int32 left, Int32 right) => new Int32(left.value * right.value);
    public static Int32 operator / (Int32 left, Int32 right) => new Int32(left.value / right.value);
    public static Int32 operator % (Int32 left, Int32 right) => new Int32(left.value % right.value);
    public static Int32 operator - (Int32 value) => new Int32(- value.value);
    public static Int32 operator + (Int32 value) => value;
    public static Int32 operator ++ (Int32 value) => new Int32(value.value + 1);
    public static Int32 operator -- (Int32 value) => new Int32(value.value - 1);
    public static Int32 operator & (Int32 left, Int32 right) => new Int32(left.value & right.value);
    public static Int32 operator | (Int32 left, Int32 right) => new Int32(left.value | right.value);
    public static Int32 operator ^ (Int32 left, Int32 right) => new Int32(left.value ^ right.value);
    public static Int32 operator ~ (Int32 value) => new Int32(value.value ^ - 1);
    public static Int32 operator << (Int32 value, int offset) => new Int32(value.value << offset);
    public static Int32 operator >> (Int32 value, int offset) => new Int32(value.value >> offset);
    public static Int32 Abs(Int32 value) => value.value <0 ?new Int32(- value.value) : value;
    public static int Compare(Int32 left, Int32 right) {
        return left.CompareTo(right);
    }
    public static bool Equals(Int32 left, Int32 right) => left.value == right.value;
    public bool Equals(Int32 other) => value == other.value;
    public static Int32 Min(Int32 left, Int32 right) => left.value <= right.value ?left : right;
    public static Int32 Max(Int32 left, Int32 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Int32 left, Int32 right, out Int32 result) {
        var raw = 0;
        if (!NumericArithmetic.TryAddInt32 (left.value, right.value, out raw)) {
            result = new Int32(0);
            return false;
        }
        result = new Int32(raw);
        return true;
    }
    public static bool TrySubtract(Int32 left, Int32 right, out Int32 result) {
        var raw = 0;
        if (!NumericArithmetic.TrySubtractInt32 (left.value, right.value, out raw)) {
            result = new Int32(0);
            return false;
        }
        result = new Int32(raw);
        return true;
    }
    public static bool TryMultiply(Int32 left, Int32 right, out Int32 result) {
        var raw = 0;
        if (!NumericArithmetic.TryMultiplyInt32 (left.value, right.value, out raw)) {
            result = new Int32(0);
            return false;
        }
        result = new Int32(raw);
        return true;
    }
    public static bool TryNegate(Int32 value, out Int32 result) {
        var raw = 0;
        if (!NumericArithmetic.TryNegateInt32 (value.value, out raw)) {
            result = new Int32(0);
            return false;
        }
        result = new Int32(raw);
        return true;
    }
    public static int LeadingZeroCount(Int32 value) {
        return NumericBitOperations.LeadingZeroCountInt32(value.value);
    }
    public static int TrailingZeroCount(Int32 value) {
        return NumericBitOperations.TrailingZeroCountInt32(value.value);
    }
    public static int PopCount(Int32 value) {
        return NumericBitOperations.PopCountInt32(value.value);
    }
    public static Int32 RotateLeft(Int32 value, int offset) {
        return new Int32(NumericBitOperations.RotateLeftInt32(value.value, offset));
    }
    public static Int32 RotateRight(Int32 value, int offset) {
        return new Int32(NumericBitOperations.RotateRightInt32(value.value, offset));
    }
    public static Int32 ReverseEndianness(Int32 value) {
        return new Int32(NumericBitOperations.ReverseEndiannessInt32(value.value));
    }
    public static bool IsPowerOfTwo(Int32 value) {
        return NumericBitOperations.IsPowerOfTwoInt32(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatInt32(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatInt32(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatInt32(value, destination, out written, format, culture);
    }
}
