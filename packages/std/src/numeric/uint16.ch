namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "ushort", kind = "int", bits = 16, signed = false,
aliases = ["ushort", "UInt16", "Std.UInt16", "Std.Numeric.UInt16", "System.UInt16", "uint16", "u16"], c_type = "uint16_t") public readonly struct UInt16 : IComparable, IComparable <ushort >, IConvertible, IEquatable <ushort >, IParsable <ushort >, ISpanParsable <ushort >, IUtf8SpanParsable <ushort >, IAdditionOperators <ushort, ushort, ushort >, IAdditiveIdentity <ushort, ushort >, IBinaryInteger <ushort >, IBinaryNumber <ushort >, IBitwiseOperators <ushort, ushort, ushort >, IComparisonOperators <ushort, ushort, bool >, IDecrementOperators <ushort >, IDivisionOperators <ushort, ushort, ushort >, IEqualityOperators <ushort, ushort, bool >, IIncrementOperators <ushort >, IMinMaxValue <ushort >, IModulusOperators <ushort, ushort, ushort >, IMultiplicativeIdentity <ushort, ushort >, IMultiplyOperators <ushort, ushort, ushort >, INumber <ushort >, INumberBase <ushort >, IShiftOperators <ushort, int, ushort >, ISubtractionOperators <ushort, ushort, ushort >, IUnaryPlusOperators <ushort, ushort >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly ushort value;
    public const ushort MinValue = 0u16;
    public const ushort MaxValue = 0xFFFFu16;
    public init(ushort value) {
        this.value = value;
    }
    public ushort ToUInt16() => value;
    public static UInt16 From(ushort value) => new UInt16(value);
    public static UInt16 Zero => new UInt16(0u16);
    public static UInt16 One => new UInt16(1u16);
    public static UInt16 AdditiveIdentity() => Zero;
    public static UInt16 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatUInt16(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromUInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(NumericUnchecked.ToUInt64(value));
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
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromUInt64(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("UInt16",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("UInt16");
    public string Format(string format, string culture) => NumericFormatting.FormatUInt16(value, format, culture);
    public static UInt16 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out UInt16 result) {
        var parsed = 0u16;
        if (! NumericParse.TryParseUInt16 (text, out parsed)) {
            result = new UInt16(0u16);
            return false;
        }
        result = new UInt16(parsed);
        return true;
    }
    public static UInt16 Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseUInt32 (text, out var widened, out status)) {
            NumericParse.ThrowParseException(status, "UInt16");
        }
        if (widened >NumericConstants.UInt16Max)
        {
            NumericParse.ThrowParseException(ParseStatus.Overflow, "UInt16");
        }
        return new UInt16(NumericUnchecked.ToUInt16(widened));
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out UInt16 result) {
        var parsed = 0u16;
        if (! NumericParse.TryParseUInt16 (text, out parsed)) {
            result = new UInt16(0u16);
            return false;
        }
        result = new UInt16(parsed);
        return true;
    }
    public int CompareTo(UInt16 other) {
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
        throw new Std.ArgumentException("object is not a UInt16");
    }
    public static int Compare(UInt16 left, UInt16 right) {
        return left.CompareTo(right);
    }
    public static bool operator == (UInt16 left, UInt16 right) => left.value == right.value;
    public static bool operator != (UInt16 left, UInt16 right) => left.value != right.value;
    public static bool operator <(UInt16 left, UInt16 right) => left.value <right.value;
    public static bool operator <= (UInt16 left, UInt16 right) => left.value <= right.value;
    public static bool operator >(UInt16 left, UInt16 right) => left.value >right.value;
    public static bool operator >= (UInt16 left, UInt16 right) => left.value >= right.value;
    public static UInt16 operator + (UInt16 left, UInt16 right) => new UInt16(NumericUnchecked.ToUInt16(left.value + right.value));
    public static UInt16 operator - (UInt16 left, UInt16 right) => new UInt16(NumericUnchecked.ToUInt16(left.value - right.value));
    public static UInt16 operator * (UInt16 left, UInt16 right) => new UInt16(NumericUnchecked.ToUInt16(left.value * right.value));
    public static UInt16 operator / (UInt16 left, UInt16 right) => new UInt16(NumericUnchecked.ToUInt16(left.value / right.value));
    public static UInt16 operator % (UInt16 left, UInt16 right) => new UInt16(NumericUnchecked.ToUInt16(left.value % right.value));
    public static UInt16 operator + (UInt16 value) => value;
    public static UInt16 operator ++ (UInt16 value) => new UInt16(NumericUnchecked.ToUInt16(value.value + 1u16));
    public static UInt16 operator -- (UInt16 value) => new UInt16(NumericUnchecked.ToUInt16(value.value - 1u16));
    public static UInt16 operator & (UInt16 left, UInt16 right) => new UInt16((ushort)(left.value & right.value));
    public static UInt16 operator | (UInt16 left, UInt16 right) => new UInt16((ushort)(left.value | right.value));
    public static UInt16 operator ^ (UInt16 left, UInt16 right) => new UInt16((ushort)(left.value ^ right.value));
    public static UInt16 operator ~ (UInt16 value) => new UInt16((ushort)(value.value ^ 0xFFFFu16));
    public static UInt16 operator << (UInt16 value, int offset) => new UInt16((ushort)(value.value << offset));
    public static UInt16 operator >> (UInt16 value, int offset) => new UInt16((ushort)(value.value >> offset));
    public static bool Equals(UInt16 left, UInt16 right) => left.value == right.value;
    public bool Equals(UInt16 other) => value == other.value;
    public static UInt16 Min(UInt16 left, UInt16 right) => left.value <= right.value ?left : right;
    public static UInt16 Max(UInt16 left, UInt16 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(UInt16 left, UInt16 right, out UInt16 result) {
        var raw = 0u16;
        if (! NumericArithmetic.TryAddUInt16 (left.value, right.value, out raw)) {
            result = new UInt16(0u16);
            return false;
        }
        result = new UInt16(raw);
        return true;
    }
    public static bool TrySubtract(UInt16 left, UInt16 right, out UInt16 result) {
        var raw = 0u16;
        if (! NumericArithmetic.TrySubtractUInt16 (left.value, right.value, out raw)) {
            result = new UInt16(0u16);
            return false;
        }
        result = new UInt16(raw);
        return true;
    }
    public static bool TryMultiply(UInt16 left, UInt16 right, out UInt16 result) {
        var raw = 0u16;
        if (! NumericArithmetic.TryMultiplyUInt16 (left.value, right.value, out raw)) {
            result = new UInt16(0u16);
            return false;
        }
        result = new UInt16(raw);
        return true;
    }
    public static int LeadingZeroCount(UInt16 value) {
        return NumericBitOperations.LeadingZeroCountUInt16(value.value);
    }
    public static int TrailingZeroCount(UInt16 value) {
        return NumericBitOperations.TrailingZeroCountUInt16(value.value);
    }
    public static int PopCount(UInt16 value) {
        return NumericBitOperations.PopCountUInt16(value.value);
    }
    public static UInt16 RotateLeft(UInt16 value, int offset) {
        return new UInt16(NumericBitOperations.RotateLeftUInt16(value.value, offset));
    }
    public static UInt16 RotateRight(UInt16 value, int offset) {
        return new UInt16(NumericBitOperations.RotateRightUInt16(value.value, offset));
    }
    public static UInt16 ReverseEndianness(UInt16 value) {
        return new UInt16(NumericBitOperations.ReverseEndiannessUInt16(value.value));
    }
    public static bool IsPowerOfTwo(UInt16 value) {
        return NumericBitOperations.IsPowerOfTwoUInt16(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatUInt16(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatUInt16(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatUInt16(value, destination, out written, format, culture);
    }
}
