namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Strings;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "double", kind = "float", bits = 64, aliases = ["double",
"Float64", "Std.Float64", "Std.Numeric.Float64", "System.Double", "float64", "f64"], c_type = "double") public readonly struct Float64 : IComparable, IComparable <double >, IConvertible, IEquatable <double >, IParsable <double >, ISpanParsable <double >, IUtf8SpanParsable <double >, IAdditionOperators <double, double, double >, IAdditiveIdentity <double, double >, IComparisonOperators <double, double, bool >, IDecrementOperators <double >, IDivisionOperators <double, double, double >, IEqualityOperators <double, double, bool >, IIncrementOperators <double >, IMinMaxValue <double >, IModulusOperators <double, double, double >, IMultiplicativeIdentity <double, double >, IMultiplyOperators <double, double, double >, INumber <double >, INumberBase <double >, ISignedNumber <double >, ISubtractionOperators <double, double, double >, IUnaryNegationOperators <double, double >, IUnaryPlusOperators <double, double >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly double value;
    public const double MinValue = - 1.7976931348623157e308;
    public const double MaxValue = 1.7976931348623157e308;
    public const double Epsilon = 4.9406564584124654e-324;
    public const double NegativeZero = - 0.0d;
    public static Float64 PositiveInfinity => CreatePositiveInfinity();
    public static Float64 NegativeInfinity => CreateNegativeInfinity();
    public static Float64 NaN => CreateNaN();
    public init(double value) {
        this.value = value;
    }
    public double ToFloat64() => value;
    public static Float64 From(double value) => new Float64(value);
    public static Float64 Zero => new Float64(0.0d);
    public static Float64 One => new Float64(1.0d);
    public static Float64 NegativeOne() => new Float64(- 1.0d);
    public static Float64 AdditiveIdentity() => Zero;
    public static Float64 MultiplicativeIdentity() => One;
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
    public Float128 ToFloat128(IFormatProvider provider) => new Float128(value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromFloat64(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Float64",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Float64");
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
    public static Float64 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        if (TryParse (text, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float64");
        return new Float64(0.0d);
    }
    public static bool TryParse(string text, out Float64 result) {
        if (text == null)
        {
            result = new Float64(0.0d);
            return false;
        }
        let utf8 = Std.Span.ReadOnlySpan.FromString(text);
        return TryParse(utf8, out result);
    }
    public static Float64 Parse(ReadOnlySpan <byte >text) {
        if (TryParse (text, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float64");
        return new Float64(0.0d);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Float64 result) {
        result = new Float64(0.0d);
        return false;
    }
    public int CompareTo(Float64 other) {
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
        throw new Std.ArgumentException("object is not a Float64");
    }
    public static int Compare(Float64 left, Float64 right) => left.CompareTo(right);
    public static bool operator == (Float64 left, Float64 right) => left.value == right.value;
    public static bool operator != (Float64 left, Float64 right) => left.value != right.value;
    public static bool operator <(Float64 left, Float64 right) => left.value <right.value;
    public static bool operator <= (Float64 left, Float64 right) => left.value <= right.value;
    public static bool operator >(Float64 left, Float64 right) => left.value >right.value;
    public static bool operator >= (Float64 left, Float64 right) => left.value >= right.value;
    public static Float64 operator + (Float64 left, Float64 right) => new Float64(left.value + right.value);
    public static Float64 operator - (Float64 left, Float64 right) => new Float64(left.value - right.value);
    public static Float64 operator * (Float64 left, Float64 right) => new Float64(left.value * right.value);
    public static Float64 operator / (Float64 left, Float64 right) => new Float64(left.value / right.value);
    public static Float64 operator % (Float64 left, Float64 right) => new Float64(left.value % right.value);
    public static Float64 operator - (Float64 value) => new Float64(- value.value);
    public static Float64 operator + (Float64 value) => value;
    public static Float64 operator ++ (Float64 value) => new Float64(value.value + 1.0d);
    public static Float64 operator -- (Float64 value) => new Float64(value.value - 1.0d);
    public static Float64 Abs(Float64 value) {
        if (value.value <0.0d)
        {
            return new Float64(- value.value);
        }
        return value;
    }
    public static bool Equals(Float64 left, Float64 right) => left.value == right.value;
    public bool Equals(Float64 other) => value == other.value;
    public static Float64 Min(Float64 left, Float64 right) => left.value <= right.value ?left : right;
    public static Float64 Max(Float64 left, Float64 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Float64 left, Float64 right, out Float64 result) {
        result = new Float64(left.value + right.value);
        return true;
    }
    public static bool TrySubtract(Float64 left, Float64 right, out Float64 result) {
        result = new Float64(left.value - right.value);
        return true;
    }
    public static bool TryMultiply(Float64 left, Float64 right, out Float64 result) {
        result = new Float64(left.value * right.value);
        return true;
    }
    public static bool TryDivide(Float64 left, Float64 right, out Float64 result) {
        result = new Float64(left.value / right.value);
        return true;
    }
    public static bool TryNegate(Float64 value, out Float64 result) {
        result = new Float64(- value.value);
        return true;
    }
    private static Float64 CreatePositiveInfinity() {
        let zero = 0.0d;
        return new Float64(1.0d / zero);
    }
    private static Float64 CreateNegativeInfinity() {
        let zero = 0.0d;
        return new Float64(- 1.0d / zero);
    }
    private static Float64 CreateNaN() {
        let zero = 0.0d;
        return new Float64(zero / zero);
    }
}
