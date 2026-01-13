namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Strings;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "float", kind = "float", bits = 32, aliases = ["float",
"Float32", "Std.Float32", "Std.Numeric.Float32", "System.Single", "float32", "f32"], c_type = "float") public readonly struct Float32 : IComparable, IComparable <float >, IConvertible, IEquatable <float >, IParsable <float >, ISpanParsable <float >, IUtf8SpanParsable <float >, IAdditionOperators <float, float, float >, IAdditiveIdentity <float, float >, IComparisonOperators <float, float, bool >, IDecrementOperators <float >, IDivisionOperators <float, float, float >, IEqualityOperators <float, float, bool >, IIncrementOperators <float >, IMinMaxValue <float >, IModulusOperators <float, float, float >, IMultiplicativeIdentity <float, float >, IMultiplyOperators <float, float, float >, INumber <float >, INumberBase <float >, ISignedNumber <float >, ISubtractionOperators <float, float, float >, IUnaryNegationOperators <float, float >, IUnaryPlusOperators <float, float >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly float value;
    public const float MinValue = - 3.40282347e38f;
    public const float MaxValue = 3.40282347e38f;
    public const float Epsilon = 1.40129846e-45f;
    public const float NegativeZero = - 0.0f;
    public static Float32 PositiveInfinity => CreatePositiveInfinity();
    public static Float32 NegativeInfinity => CreateNegativeInfinity();
    public static Float32 NaN => CreateNaN();
    public init(float value) {
        this.value = value;
    }
    public float ToFloat32() => value;
    public static Float32 From(float value) => new Float32(value);
    public static Float32 Zero => new Float32(0.0f);
    public static Float32 One => new Float32(1.0f);
    public static Float32 NegativeOne() => new Float32(- 1.0f);
    public static Float32 AdditiveIdentity() => Zero;
    public static Float32 MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatFloat32(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromDouble(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked((double) value);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked((double) value);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked((double) value);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked((double) value);
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked((double) value);
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked((double) value);
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked((double) value);
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked((double) value);
    public ulong ToUInt64(IFormatProvider provider) => ConvertibleHelpers.ToUInt64Checked((double) value);
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(ConvertibleHelpers.ToInt64Checked((double) value));
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(ConvertibleHelpers.ToUInt64Checked((double) value));
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(ConvertibleHelpers.ToInt64Checked((double) value));
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(ConvertibleHelpers.ToUInt64Checked((double) value));
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked((double) value);
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked((double) value);
    public float ToSingle(IFormatProvider provider) => value;
    public double ToDouble(IFormatProvider provider) => (double) value;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) value);
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromFloat32(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Float32",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Float32");
    public string Format(string format, string culture) => NumericFormatting.FormatFloat32(value, format, culture);
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatFloat32(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format = null) => NumericFormatting.TryFormatFloat32(value,
    destination, out written, format, null);
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) => NumericFormatting.TryFormatFloat32(value,
    destination, out written, format, culture);
    public static Float32 Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        if (TryParse (text, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float32");
        return new Float32(0.0f);
    }
    public static bool TryParse(string text, out Float32 result) {
        if (text == null)
        {
            result = new Float32(0.0f);
            return false;
        }
        let utf8 = Std.Span.ReadOnlySpan.FromString(text);
        return TryParse(utf8, out result);
    }
    public static Float32 Parse(ReadOnlySpan <byte >text) {
        if (TryParse (text, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float32");
        return new Float32(0.0f);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out Float32 result) {
        result = new Float32(0.0f);
        return false;
    }
    public int CompareTo(Float32 other) {
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
        // NaN comparisons mirror .NET: NaN compares unordered, treat as not equal
        return 1;
    }
    public int CompareTo(Object other) {
        throw new Std.ArgumentException("object is not a Float32");
    }
    public static int Compare(Float32 left, Float32 right) => left.CompareTo(right);
    public static bool operator == (Float32 left, Float32 right) => left.value == right.value;
    public static bool operator != (Float32 left, Float32 right) => left.value != right.value;
    public static bool operator <(Float32 left, Float32 right) => left.value <right.value;
    public static bool operator <= (Float32 left, Float32 right) => left.value <= right.value;
    public static bool operator >(Float32 left, Float32 right) => left.value >right.value;
    public static bool operator >= (Float32 left, Float32 right) => left.value >= right.value;
    public static Float32 operator + (Float32 left, Float32 right) => new Float32(left.value + right.value);
    public static Float32 operator - (Float32 left, Float32 right) => new Float32(left.value - right.value);
    public static Float32 operator * (Float32 left, Float32 right) => new Float32(left.value * right.value);
    public static Float32 operator / (Float32 left, Float32 right) => new Float32(left.value / right.value);
    public static Float32 operator % (Float32 left, Float32 right) => new Float32(left.value % right.value);
    public static Float32 operator - (Float32 value) => new Float32(- value.value);
    public static Float32 operator + (Float32 value) => value;
    public static Float32 operator ++ (Float32 value) => new Float32(value.value + 1.0f);
    public static Float32 operator -- (Float32 value) => new Float32(value.value - 1.0f);
    public static Float32 Abs(Float32 value) {
        if (value.value <0.0f)
        {
            return new Float32(- value.value);
        }
        return value;
    }
    public static bool Equals(Float32 left, Float32 right) => left.value == right.value;
    public bool Equals(Float32 other) => value == other.value;
    public static Float32 Min(Float32 left, Float32 right) => left.value <= right.value ?left : right;
    public static Float32 Max(Float32 left, Float32 right) => left.value >= right.value ?left : right;
    public static bool TryAdd(Float32 left, Float32 right, out Float32 result) {
        result = new Float32(left.value + right.value);
        return true;
    }
    public static bool TrySubtract(Float32 left, Float32 right, out Float32 result) {
        result = new Float32(left.value - right.value);
        return true;
    }
    public static bool TryMultiply(Float32 left, Float32 right, out Float32 result) {
        result = new Float32(left.value * right.value);
        return true;
    }
    public static bool TryDivide(Float32 left, Float32 right, out Float32 result) {
        result = new Float32(left.value / right.value);
        return true;
    }
    public static bool TryNegate(Float32 value, out Float32 result) {
        result = new Float32(- value.value);
        return true;
    }
    private static Float32 CreatePositiveInfinity() {
        let zero = 0.0f;
        return new Float32(1.0f / zero);
    }
    private static Float32 CreateNegativeInfinity() {
        let zero = 0.0f;
        return new Float32(- 1.0f / zero);
    }
    private static Float32 CreateNaN() {
        let zero = 0.0f;
        return new Float32(zero / zero);
    }
}
