namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Strings;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "float128", kind = "float", bits = 128, aliases = ["float128",
"Float128", "Std.Float128", "Std.Numeric.Float128", "quad", "f128"], c_type = "__float128") public readonly struct Float128 : IComparable, IComparable <Float128 >, IConvertible, IEquatable <Float128 >, IParsable <Float128 >, ISpanParsable <Float128 >, IUtf8SpanParsable <Float128 >, IAdditionOperators <Float128, Float128, Float128 >, IAdditiveIdentity <Float128, Float128 >, IComparisonOperators <Float128, Float128, bool >, IDecrementOperators <Float128 >, IDivisionOperators <Float128, Float128, Float128 >, IEqualityOperators <Float128, Float128, bool >, IIncrementOperators <Float128 >, IMinMaxValue <Float128 >, IModulusOperators <Float128, Float128, Float128 >, IMultiplicativeIdentity <Float128, Float128 >, IMultiplyOperators <Float128, Float128, Float128 >, INumber <Float128 >, INumberBase <Float128 >, ISignedNumber <Float128 >, ISubtractionOperators <Float128, Float128, Float128 >, IUnaryNegationOperators <Float128, Float128 >, IUnaryPlusOperators <Float128, Float128 >, IFormattable, ISpanFormattable, IUtf8SpanFormattable
{
    private readonly double value;
    public const double MinValue = - 1.7976931348623157e308;
    public const double MaxValue = 1.7976931348623157e308;
    public const double Epsilon = 4.9406564584124654e-324;
    public const double NegativeZero = - 0.0d;
    public static Float128 PositiveInfinity => CreatePositiveInfinity();
    public static Float128 NegativeInfinity => CreateNegativeInfinity();
    public static Float128 NaN => CreateNaN();
    public init(double value) {
        this.value = value;
    }
    public double ToFloat128() => value;
    public static Float128 From(double value) => new Float128(value);
    public static Float128 Zero => new Float128(0.0d);
    public static Float128 One => new Float128(1.0d);
    public static Float128 NegativeOne() => new Float128(- 1.0d);
    public static Float128 AdditiveIdentity() => Zero;
    public static Float128 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatFloat64(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromDouble(value);
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
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(value);
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(value);
    public float ToSingle(IFormatProvider provider) => (float) value;
    public double ToDouble(IFormatProvider provider) => value;
    public Float128 ToFloat128(IFormatProvider provider) => this;
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromFloat128(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Float128",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Float128");
    public string Format(string format, string culture) => NumericFormatting.FormatFloat64(value, format, culture);
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatFloat64(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format = null) => NumericFormatting.TryFormatFloat64(value,
    destination, out written, format, null);
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) => NumericFormatting.TryFormatFloat64(value,
    destination, out written, format, culture);
    public static Float128 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        if (TryParse (text, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float128");
        return new Float128(0.0d);
    }
    public static bool TryParse(string text, out Float128 result) {
        if (text == null)
        {
            result = new Float128(0.0d);
            return false;
        }
        let utf8 = Std.Span.ReadOnlySpan.FromString(text);
        return TryParse(utf8, out result);
    }
    public static Float128 Parse(ReadOnlySpan <byte >text) {
        if (TryParse (text, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float128");
        return new Float128(0.0d);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Float128 result) {
        result = new Float128(0.0d);
        return false;
    }
    public int CompareTo(Float128 other) {
        if (value <other.value)
        {
            return - 1;
        }
        if (value >other.value)
        {
            return 1;
        }
        if (value == other.value)
        {
            return 0;
        }
        return 1;
    }
    public int CompareTo(Object other) {
        throw new Std.ArgumentException("object is not a Float128");
    }
    public static int Compare(Float128 left, Float128 right) => left.CompareTo(right);
    public static bool operator == (Float128 left, Float128 right) => left.value == right.value;
    public static bool operator != (Float128 left, Float128 right) => left.value != right.value;
    public static bool operator <(Float128 left, Float128 right) => left.value <right.value;
    public static bool operator <= (Float128 left, Float128 right) => left.value <= right.value;
    public static bool operator >(Float128 left, Float128 right) => left.value >right.value;
    public static bool operator >= (Float128 left, Float128 right) => left.value >= right.value;
    public static Float128 operator + (Float128 left, Float128 right) => new Float128(left.value + right.value);
    public static Float128 operator - (Float128 left, Float128 right) => new Float128(left.value - right.value);
    public static Float128 operator * (Float128 left, Float128 right) => new Float128(left.value * right.value);
    public static Float128 operator / (Float128 left, Float128 right) => new Float128(left.value / right.value);
    public static Float128 operator % (Float128 left, Float128 right) => new Float128(left.value % right.value);
    public static Float128 operator - (Float128 value) => new Float128(- value.value);
    public static Float128 operator + (Float128 value) => value;
    public static Float128 operator ++ (Float128 value) => new Float128(value.value + 1.0d);
    public static Float128 operator -- (Float128 value) => new Float128(value.value - 1.0d);
    public static Float128 Abs(Float128 value) {
        if (value.value <0.0d)
        {
            return new Float128(- value.value);
        }
        return value;
    }
    public static bool Equals(Float128 left, Float128 right) => left.value == right.value;
    public bool Equals(Float128 other) => value == other.value;
    public static Float128 Min(Float128 left, Float128 right) => left.value <= right.value ?left : right;
    public static Float128 Max(Float128 left, Float128 right) => left.value >= right.value ?left : right;
    private static Float128 CreatePositiveInfinity() {
        let zero = 0.0d;
        return new Float128(1.0d / zero);
    }
    private static Float128 CreateNegativeInfinity() {
        let zero = 0.0d;
        return new Float128(- 1.0d / zero);
    }
    private static Float128 CreateNaN() {
        let zero = 0.0d;
        return new Float128(zero / zero);
    }
}
