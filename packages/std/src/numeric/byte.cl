namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "byte", kind = "int", bits = 8, signed = false, aliases = ["byte",
"Byte", "Std.Byte", "Std.Numeric.Byte", "System.Byte", "uint8", "u8"], c_type = "uint8_t") public readonly struct Byte : IComparable, IComparable <byte >, IConvertible, IEquatable <byte >, IParsable <byte >, ISpanParsable <byte >, IUtf8SpanParsable <byte >, IAdditionOperators <byte, byte, byte >, IAdditiveIdentity <byte, byte >, IBinaryInteger <byte >, IBinaryNumber <byte >, IBitwiseOperators <byte, byte, byte >, IComparisonOperators <byte, byte, bool >, IDecrementOperators <byte >, IDivisionOperators <byte, byte, byte >, IEqualityOperators <byte, byte, bool >, IIncrementOperators <byte >, IMinMaxValue <byte >, IModulusOperators <byte, byte, byte >, IMultiplicativeIdentity <byte, byte >, IMultiplyOperators <byte, byte, byte >, INumber <byte >, INumberBase <byte >, IShiftOperators <byte, int, byte >, ISubtractionOperators <byte, byte, byte >, IUnaryPlusOperators <byte, byte >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly byte value;
    public const byte MinValue = 0u8;
    public const byte MaxValue = 0xFFu8;
    public init(byte value) {
        this.value = value;
    }
    public byte ToByte() => value;
    public static Byte From(byte value) => new Byte(value);
    public static Byte Zero => new Byte(0u8);
    public static Byte One => new Byte(1u8);
    public static Byte AdditiveIdentity() => Zero;
    public static Byte MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatByte(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromUInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(NumericUnchecked.ToUInt64(value));
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(NumericUnchecked.ToUInt64(value));
    public byte ToByte(IFormatProvider provider) => value;
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(NumericUnchecked.ToUInt64(value));
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(NumericUnchecked.ToUInt64(value));
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
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Byte",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Byte");
    public string Format(string format, string culture) => NumericFormatting.FormatByte(value, format, culture);
    public static Byte Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out Byte result) {
        var parsed = 0u8;
        if (! NumericParse.TryParseByte (text, out parsed)) {
            result = new Byte(0u8);
            return false;
        }
        result = new Byte(parsed);
        return true;
    }
    public static Byte Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseUInt32 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Byte");
        }
        if (parsed >NumericConstants.UInt8Max)
        {
            NumericParse.ThrowParseException(ParseStatus.Overflow, "Byte");
        }
        return new Byte(NumericUnchecked.ToByte(parsed));
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Byte result) {
        var parsed = 0u8;
        if (! NumericParse.TryParseByte (text, out parsed)) {
            result = new Byte(0u8);
            return false;
        }
        result = new Byte(parsed);
        return true;
    }
    public int CompareTo(Byte other) {
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
        throw new Std.ArgumentException("object is not a Byte");
    }
    public static int Compare(Byte left, Byte right) {
        return left.CompareTo(right);
    }
    public static bool operator == (Byte left, Byte right) => left.value == right.value;
    public static bool operator != (Byte left, Byte right) => left.value != right.value;
    public static bool operator <(Byte left, Byte right) => left.value <right.value;
    public static bool operator <= (Byte left, Byte right) => left.value <= right.value;
    public static bool operator >(Byte left, Byte right) => left.value >right.value;
    public static bool operator >= (Byte left, Byte right) => left.value >= right.value;
    public static Byte operator + (Byte left, Byte right) => new Byte(NumericUnchecked.ToByte(left.value + right.value));
    public static Byte operator - (Byte left, Byte right) => new Byte(NumericUnchecked.ToByte(left.value - right.value));
    public static Byte operator * (Byte left, Byte right) => new Byte(NumericUnchecked.ToByte(left.value * right.value));
    public static Byte operator / (Byte left, Byte right) => new Byte(NumericUnchecked.ToByte(left.value / right.value));
    public static Byte operator % (Byte left, Byte right) => new Byte(NumericUnchecked.ToByte(left.value % right.value));
    public static Byte operator + (Byte value) => value;
    public static Byte operator ++ (Byte value) => new Byte(NumericUnchecked.ToByte(value.value + 1u8));
    public static Byte operator -- (Byte value) => new Byte(NumericUnchecked.ToByte(value.value - 1u8));
    public static Byte operator & (Byte left, Byte right) => new Byte((byte)(left.value & right.value));
    public static Byte operator | (Byte left, Byte right) => new Byte((byte)(left.value | right.value));
    public static Byte operator ^ (Byte left, Byte right) => new Byte((byte)(left.value ^ right.value));
    public static Byte operator ~ (Byte value) => new Byte((byte)(value.value ^ 0xFFu8));
    public static Byte operator << (Byte value, int offset) => new Byte((byte)(value.value << offset));
    public static Byte operator >> (Byte value, int offset) => new Byte((byte)(value.value >> offset));
    public static bool Equals(Byte left, Byte right) => left.value == right.value;
    public bool Equals(Byte other) => value == other.value;
    public static Byte Min(Byte left, Byte right) => left.value <= right.value ?left : right;
    public static Byte Max(Byte left, Byte right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Byte left, Byte right, out Byte result) {
        var raw = 0u8;
        if (! NumericArithmetic.TryAddByte (left.value, right.value, out raw)) {
            result = new Byte(0u8);
            return false;
        }
        result = new Byte(raw);
        return true;
    }
    public static bool TrySubtract(Byte left, Byte right, out Byte result) {
        var raw = 0u8;
        if (! NumericArithmetic.TrySubtractByte (left.value, right.value, out raw)) {
            result = new Byte(0u8);
            return false;
        }
        result = new Byte(raw);
        return true;
    }
    public static bool TryMultiply(Byte left, Byte right, out Byte result) {
        var raw = 0u8;
        if (! NumericArithmetic.TryMultiplyByte (left.value, right.value, out raw)) {
            result = new Byte(0u8);
            return false;
        }
        result = new Byte(raw);
        return true;
    }
    public static int LeadingZeroCount(Byte value) {
        return NumericBitOperations.LeadingZeroCountByte(value.value);
    }
    public static int TrailingZeroCount(Byte value) {
        return NumericBitOperations.TrailingZeroCountByte(value.value);
    }
    public static int PopCount(Byte value) {
        return NumericBitOperations.PopCountByte(value.value);
    }
    public static Byte RotateLeft(Byte value, int offset) {
        return new Byte(NumericBitOperations.RotateLeftByte(value.value, offset));
    }
    public static Byte RotateRight(Byte value, int offset) {
        return new Byte(NumericBitOperations.RotateRightByte(value.value, offset));
    }
    public static Byte ReverseEndianness(Byte value) {
        return new Byte(NumericBitOperations.ReverseEndiannessByte(value.value));
    }
    public static bool IsPowerOfTwo(Byte value) {
        return NumericBitOperations.IsPowerOfTwoByte(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatByte(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatByte(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatByte(value, destination, out written, format, culture);
    }
}
