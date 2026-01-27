namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "uint", kind = "int", bits = 32, signed = false, aliases = ["uint",
"UInt32", "Std.UInt32", "Std.Numeric.UInt32", "System.UInt32", "uint32", "u32"], c_type = "uint32_t") public readonly struct UInt32 : IComparable, IComparable <uint >, IConvertible, IEquatable <uint >, IParsable <uint >, ISpanParsable <uint >, IUtf8SpanParsable <uint >, IAdditionOperators <uint, uint, uint >, IAdditiveIdentity <uint, uint >, IBinaryInteger <uint >, IBinaryNumber <uint >, IBitwiseOperators <uint, uint, uint >, IComparisonOperators <uint, uint, bool >, IDecrementOperators <uint >, IDivisionOperators <uint, uint, uint >, IEqualityOperators <uint, uint, bool >, IIncrementOperators <uint >, IMinMaxValue <uint >, IModulusOperators <uint, uint, uint >, IMultiplicativeIdentity <uint, uint >, IMultiplyOperators <uint, uint, uint >, INumber <uint >, INumberBase <uint >, IShiftOperators <uint, int, uint >, ISubtractionOperators <uint, uint, uint >, IUnaryPlusOperators <uint, uint >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly uint value;
    public const uint MinValue = 0u;
    public const uint MaxValue = 0xFFFF_FFFFu;
    public init(uint value) {
        this.value = value;
    }
    public uint ToUInt32() => value;
    public static UInt32 From(uint value) => new UInt32(value);
    public static UInt32 Zero => new UInt32(0u);
    public static UInt32 One => new UInt32(1u);
    public static UInt32 AdditiveIdentity() => Zero;
    public static UInt32 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatUInt32(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromUInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(NumericUnchecked.ToUInt64(value));
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(NumericUnchecked.ToUInt64(value));
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(NumericUnchecked.ToUInt64(value));
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(NumericUnchecked.ToUInt64(value));
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(NumericUnchecked.ToUInt64(value));
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(NumericUnchecked.ToUInt64(value));
    public uint ToUInt32(IFormatProvider provider) => value;
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
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromUInt64(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("UInt32",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("UInt32");
    public string Format(string format, string culture) => NumericFormatting.FormatUInt32(value, format, culture);
    public static UInt32 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out UInt32 result) {
        var parsed = 0u;
        if (!NumericParse.TryParseUInt32 (text, out parsed)) {
            result = new UInt32(0u);
            return false;
        }
        result = new UInt32(parsed);
        return true;
    }
    public static UInt32 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseUInt32 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "UInt32");
        }
        return new UInt32(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out UInt32 result) {
        var parsed = 0u;
        if (!NumericParse.TryParseUInt32 (text, out parsed)) {
            result = new UInt32(0u);
            return false;
        }
        result = new UInt32(parsed);
        return true;
    }
    public int CompareTo(UInt32 other) {
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
        throw new Std.ArgumentException("object is not a UInt32");
    }
    public static int Compare(UInt32 left, UInt32 right) {
        return left.CompareTo(right);
    }
    public static bool operator == (UInt32 left, UInt32 right) => left.value == right.value;
    public static bool operator != (UInt32 left, UInt32 right) => left.value != right.value;
    public static bool operator <(UInt32 left, UInt32 right) => left.value <right.value;
    public static bool operator <= (UInt32 left, UInt32 right) => left.value <= right.value;
    public static bool operator >(UInt32 left, UInt32 right) => left.value >right.value;
    public static bool operator >= (UInt32 left, UInt32 right) => left.value >= right.value;
    public static UInt32 operator + (UInt32 left, UInt32 right) => new UInt32(left.value + right.value);
    public static UInt32 operator - (UInt32 left, UInt32 right) => new UInt32(left.value - right.value);
    public static UInt32 operator * (UInt32 left, UInt32 right) => new UInt32(left.value * right.value);
    public static UInt32 operator / (UInt32 left, UInt32 right) => new UInt32(left.value / right.value);
    public static UInt32 operator % (UInt32 left, UInt32 right) => new UInt32(left.value % right.value);
    public static UInt32 operator + (UInt32 value) => value;
    public static UInt32 operator ++ (UInt32 value) => new UInt32(value.value + 1u);
    public static UInt32 operator -- (UInt32 value) => new UInt32(value.value - 1u);
    public static UInt32 operator & (UInt32 left, UInt32 right) => new UInt32(left.value & right.value);
    public static UInt32 operator | (UInt32 left, UInt32 right) => new UInt32(left.value | right.value);
    public static UInt32 operator ^ (UInt32 left, UInt32 right) => new UInt32(left.value ^ right.value);
    public static UInt32 operator ~ (UInt32 value) => new UInt32(value.value ^ 0xFFFFFFFFu);
    public static UInt32 operator << (UInt32 value, int offset) => new UInt32(value.value << offset);
    public static UInt32 operator >> (UInt32 value, int offset) => new UInt32(value.value >> offset);
    public static bool Equals(UInt32 left, UInt32 right) => left.value == right.value;
    public bool Equals(UInt32 other) => value == other.value;
    public static UInt32 Min(UInt32 left, UInt32 right) => left.value <= right.value ?left : right;
    public static UInt32 Max(UInt32 left, UInt32 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(UInt32 left, UInt32 right, out UInt32 result) {
        var raw = 0u;
        if (!NumericArithmetic.TryAddUInt32 (left.value, right.value, out raw)) {
            result = new UInt32(0u);
            return false;
        }
        result = new UInt32(raw);
        return true;
    }
    public static bool TrySubtract(UInt32 left, UInt32 right, out UInt32 result) {
        var raw = 0u;
        if (!NumericArithmetic.TrySubtractUInt32 (left.value, right.value, out raw)) {
            result = new UInt32(0u);
            return false;
        }
        result = new UInt32(raw);
        return true;
    }
    public static bool TryMultiply(UInt32 left, UInt32 right, out UInt32 result) {
        var raw = 0u;
        if (!NumericArithmetic.TryMultiplyUInt32 (left.value, right.value, out raw)) {
            result = new UInt32(0u);
            return false;
        }
        result = new UInt32(raw);
        return true;
    }
    public static int LeadingZeroCount(UInt32 value) {
        return NumericBitOperations.LeadingZeroCountUInt32(value.value);
    }
    public static int TrailingZeroCount(UInt32 value) {
        return NumericBitOperations.TrailingZeroCountUInt32(value.value);
    }
    public static int PopCount(UInt32 value) {
        return NumericBitOperations.PopCountUInt32(value.value);
    }
    public static UInt32 RotateLeft(UInt32 value, int offset) {
        return new UInt32(NumericBitOperations.RotateLeftUInt32(value.value, offset));
    }
    public static UInt32 RotateRight(UInt32 value, int offset) {
        return new UInt32(NumericBitOperations.RotateRightUInt32(value.value, offset));
    }
    public static UInt32 ReverseEndianness(UInt32 value) {
        return new UInt32(NumericBitOperations.ReverseEndiannessUInt32(value.value));
    }
    public static bool IsPowerOfTwo(UInt32 value) {
        return NumericBitOperations.IsPowerOfTwoUInt32(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatUInt32(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatUInt32(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatUInt32(value, destination, out written, format, culture);
    }
}
