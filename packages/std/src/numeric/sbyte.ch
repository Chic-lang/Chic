namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "sbyte", kind = "int", bits = 8, signed = true, aliases = ["sbyte",
"SByte", "Std.SByte", "Std.Numeric.SByte", "System.SByte", "int8", "i8"], c_type = "int8_t") public readonly struct SByte : IComparable, IComparable <sbyte >, IConvertible, IEquatable <sbyte >, IParsable <sbyte >, ISpanParsable <sbyte >, IUtf8SpanParsable <sbyte >, IAdditionOperators <sbyte, sbyte, sbyte >, IAdditiveIdentity <sbyte, sbyte >, IBinaryInteger <sbyte >, IBinaryNumber <sbyte >, IBitwiseOperators <sbyte, sbyte, sbyte >, IComparisonOperators <sbyte, sbyte, bool >, IDecrementOperators <sbyte >, IDivisionOperators <sbyte, sbyte, sbyte >, IEqualityOperators <sbyte, sbyte, bool >, IIncrementOperators <sbyte >, IMinMaxValue <sbyte >, IModulusOperators <sbyte, sbyte, sbyte >, IMultiplicativeIdentity <sbyte, sbyte >, IMultiplyOperators <sbyte, sbyte, sbyte >, INumber <sbyte >, INumberBase <sbyte >, IShiftOperators <sbyte, int, sbyte >, ISignedNumber <sbyte >, ISubtractionOperators <sbyte, sbyte, sbyte >, IUnaryNegationOperators <sbyte, sbyte >, IUnaryPlusOperators <sbyte, sbyte >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly sbyte value;
    public const sbyte MinValue = - 128;
    public const sbyte MaxValue = 127;
    public init(sbyte value) {
        this.value = value;
    }
    public sbyte ToSByte() => value;
    public static SByte From(sbyte value) => new SByte(value);
    public static SByte Zero => new SByte(0);
    public static SByte One => new SByte(1);
    public static SByte NegativeOne() => new SByte(- 1);
    public static SByte AdditiveIdentity() => Zero;
    public static SByte MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatSByte(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromInt64(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value);
    public sbyte ToSByte(IFormatProvider provider) => value;
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
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("SByte",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("SByte");
    public string Format(string format, string culture) => NumericFormatting.FormatSByte(value, format, culture);
    public static SByte Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out SByte result) {
        var parsed = NumericUnchecked.ToSByte(0);
        if (!NumericParse.TryParseSByte (text, out parsed)) {
            result = new SByte(0);
            return false;
        }
        result = new SByte(parsed);
        return true;
    }
    public static SByte Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseInt32 (text, out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "SByte");
        }
        if (parsed <NumericConstants.SByteMin || parsed >NumericConstants.SByteMax)
        {
            NumericParse.ThrowParseException(ParseStatus.Overflow, "SByte");
        }
        return new SByte(NumericUnchecked.ToSByte(parsed));
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out SByte result) {
        var parsed = NumericUnchecked.ToSByte(0);
        if (!NumericParse.TryParseSByte (text, out parsed)) {
            result = new SByte(0);
            return false;
        }
        result = new SByte(parsed);
        return true;
    }
    public int CompareTo(SByte other) {
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
        throw new Std.ArgumentException("object is not an SByte");
    }
    public static int Compare(SByte left, SByte right) {
        return left.CompareTo(right);
    }
    public static bool operator == (SByte left, SByte right) => left.value == right.value;
    public static bool operator != (SByte left, SByte right) => left.value != right.value;
    public static bool operator <(SByte left, SByte right) => left.value <right.value;
    public static bool operator <= (SByte left, SByte right) => left.value <= right.value;
    public static bool operator >(SByte left, SByte right) => left.value >right.value;
    public static bool operator >= (SByte left, SByte right) => left.value >= right.value;
    public static SByte operator + (SByte left, SByte right) => new SByte(NumericUnchecked.ToSByte(left.value + right.value));
    public static SByte operator - (SByte left, SByte right) => new SByte(NumericUnchecked.ToSByte(left.value - right.value));
    public static SByte operator * (SByte left, SByte right) => new SByte(NumericUnchecked.ToSByte(left.value * right.value));
    public static SByte operator / (SByte left, SByte right) => new SByte(NumericUnchecked.ToSByte(left.value / right.value));
    public static SByte operator % (SByte left, SByte right) => new SByte(NumericUnchecked.ToSByte(left.value % right.value));
    public static SByte operator - (SByte value) => new SByte(NumericUnchecked.ToSByte(- value.value));
    public static SByte operator + (SByte value) => value;
    public static SByte operator ++ (SByte value) => new SByte(NumericUnchecked.ToSByte(value.value + 1));
    public static SByte operator -- (SByte value) => new SByte(NumericUnchecked.ToSByte(value.value - 1));
    public static SByte operator & (SByte left, SByte right) => new SByte((sbyte)(left.value & right.value));
    public static SByte operator | (SByte left, SByte right) => new SByte((sbyte)(left.value | right.value));
    public static SByte operator ^ (SByte left, SByte right) => new SByte((sbyte)(left.value ^ right.value));
    public static SByte operator ~ (SByte value) => new SByte((sbyte)(value.value ^ NumericUnchecked.ToSByte(- 1)));
    public static SByte operator << (SByte value, int offset) => new SByte((sbyte)(value.value << offset));
    public static SByte operator >> (SByte value, int offset) => new SByte((sbyte)(value.value >> offset));
    public static SByte Abs(SByte value) => value.value <0 ?new SByte(NumericUnchecked.ToSByte(- value.value)) : value;
    public static bool Equals(SByte left, SByte right) => left.value == right.value;
    public bool Equals(SByte other) => value == other.value;
    public static SByte Min(SByte left, SByte right) => left.value <= right.value ?left : right;
    public static SByte Max(SByte left, SByte right) => left.value >= right.value ?left : right;
    public static bool TryAdd(SByte left, SByte right, out SByte result) {
        var raw = NumericUnchecked.ToSByte(0);
        if (!NumericArithmetic.TryAddSByte (left.value, right.value, out raw)) {
            result = new SByte(0);
            return false;
        }
        result = new SByte(raw);
        return true;
    }
    public static bool TrySubtract(SByte left, SByte right, out SByte result) {
        var raw = NumericUnchecked.ToSByte(0);
        if (!NumericArithmetic.TrySubtractSByte (left.value, right.value, out raw)) {
            result = new SByte(0);
            return false;
        }
        result = new SByte(raw);
        return true;
    }
    public static bool TryMultiply(SByte left, SByte right, out SByte result) {
        var raw = NumericUnchecked.ToSByte(0);
        if (!NumericArithmetic.TryMultiplySByte (left.value, right.value, out raw)) {
            result = new SByte(0);
            return false;
        }
        result = new SByte(raw);
        return true;
    }
    public static bool TryNegate(SByte value, out SByte result) {
        var raw = NumericUnchecked.ToSByte(0);
        if (!NumericArithmetic.TryNegateSByte (value.value, out raw)) {
            result = new SByte(0);
            return false;
        }
        result = new SByte(raw);
        return true;
    }
    public static int LeadingZeroCount(SByte value) {
        return NumericBitOperations.LeadingZeroCountSByte(value.value);
    }
    public static int TrailingZeroCount(SByte value) {
        return NumericBitOperations.TrailingZeroCountSByte(value.value);
    }
    public static int PopCount(SByte value) {
        return NumericBitOperations.PopCountSByte(value.value);
    }
    public static SByte RotateLeft(SByte value, int offset) {
        return new SByte(NumericBitOperations.RotateLeftSByte(value.value, offset));
    }
    public static SByte RotateRight(SByte value, int offset) {
        return new SByte(NumericBitOperations.RotateRightSByte(value.value, offset));
    }
    public static SByte ReverseEndianness(SByte value) {
        return new SByte(NumericBitOperations.ReverseEndiannessSByte(value.value));
    }
    public static bool IsPowerOfTwo(SByte value) {
        return NumericBitOperations.IsPowerOfTwoSByte(value.value);
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatSByte(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatSByte(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatSByte(value, destination, out written, format, culture);
    }
}
