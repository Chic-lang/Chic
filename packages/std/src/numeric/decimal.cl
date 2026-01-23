namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "decimal", kind = "decimal", aliases = ["decimal",
"Decimal", "Std.Decimal", "Std.Numeric.Decimal", "System.Decimal"], c_type = "decimal128_t") public readonly struct Decimal : IComparable, IComparable <decimal >, IEquatable <decimal >, IParsable <decimal >, ISpanParsable <decimal >, IUtf8SpanParsable <decimal >, IAdditionOperators <decimal, decimal, decimal >, ISubtractionOperators <decimal, decimal, decimal >, IMultiplyOperators <decimal, decimal, decimal >, IDivisionOperators <decimal, decimal, decimal >, IModulusOperators <decimal, decimal, decimal >, IUnaryPlusOperators <decimal, decimal >, IUnaryNegationOperators <decimal, decimal >, IIncrementOperators <decimal >, IDecrementOperators <decimal >, IComparisonOperators <decimal, decimal, bool >, IEqualityOperators <decimal, decimal, bool >, INumber <decimal >, INumberBase <decimal >, ISignedNumber <decimal >, IMinMaxValue <decimal >, IAdditiveIdentity <decimal, decimal >, IMultiplicativeIdentity <decimal, decimal >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, IConvertible
{
    private readonly decimal value;
    public const decimal MinValue = - 79228162514264337593543950335m;
    public const decimal MaxValue = 79228162514264337593543950335m;
    public static decimal Zero => new Decimal(0m);
    public static decimal One => new Decimal(1m);
    public init(decimal value) {
        this.value = value;
    }
    public decimal ToDecimal() => value;
    public static decimal From(decimal value) => new Decimal(value);
    public static decimal NegativeOne() => new Decimal(- 1m);
    public static decimal AdditiveIdentity() => Zero;
    public static decimal MultiplicativeIdentity() => One;
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatDecimal(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromDecimal(value);
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(ConvertibleHelpers.ToInt64Checked(value));
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(ConvertibleHelpers.ToInt64Checked(value));
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(ConvertibleHelpers.ToInt64Checked(value));
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(ConvertibleHelpers.ToInt64Checked(value));
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(ConvertibleHelpers.ToUInt64Checked(value));
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(ConvertibleHelpers.ToInt64Checked(value));
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(ConvertibleHelpers.ToUInt64Checked(value));
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked(value);
    public ulong ToUInt64(IFormatProvider provider) => ConvertibleHelpers.ToUInt64Checked(value);
    public nint ToNInt(IFormatProvider provider) => ConvertibleHelpers.ToNIntChecked(ConvertibleHelpers.ToInt64Checked(value));
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(ConvertibleHelpers.ToUInt64Checked(value));
    public isize ToISize(IFormatProvider provider) => ConvertibleHelpers.ToISizeChecked(ConvertibleHelpers.ToInt64Checked(value));
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(ConvertibleHelpers.ToUInt64Checked(value));
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(value);
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(value);
    public float ToSingle(IFormatProvider provider) => ConvertibleHelpers.ToFloat32FromDecimal(value);
    public double ToDouble(IFormatProvider provider) => ConvertibleHelpers.ToFloat64FromDecimal(value);
    public Float128 ToFloat128(IFormatProvider provider) => new Float128(ConvertibleHelpers.ToFloat64FromDecimal(value));
    public Decimal ToDecimal(IFormatProvider provider) => value;
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Decimal",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Decimal");
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public string Format(string format, string culture) => NumericFormatting.FormatDecimal(value, format, culture);
    public static decimal Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out decimal result) {
        if (!NumericParse.TryParseDecimal (text, out var parsed)) {
            result = new Decimal(0m);
            return false;
        }
        result = new Decimal(parsed);
        return true;
    }
    public static decimal Parse(ReadOnlySpan <byte >text) {
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseDecimal (text, NumericUnchecked.ToByte ('.'), out var parsed, out status)) {
            NumericParse.ThrowParseException(status, "Decimal");
        }
        return new Decimal(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out decimal result) {
        if (!NumericParse.TryParseDecimal (text, out var parsed)) {
            result = new Decimal(0m);
            return false;
        }
        result = new Decimal(parsed);
        return true;
    }
    public int CompareTo(decimal other) {
        if (value <other)
        {
            return - 1;
        }
        if (value >other)
        {
            return 1;
        }
        return 0;
    }
    public int CompareTo(Object other) {
        throw new Std.ArgumentException("object is not a Decimal");
    }
    public static bool operator == (decimal left, decimal right) => left.value == right.value;
    public static bool operator != (decimal left, decimal right) => left.value != right.value;
    public static bool operator <(decimal left, decimal right) => left.value <right.value;
    public static bool operator <= (decimal left, decimal right) => left.value <= right.value;
    public static bool operator >(decimal left, decimal right) => left.value >right.value;
    public static bool operator >= (decimal left, decimal right) => left.value >= right.value;
    public static decimal operator + (decimal left, decimal right) => new Decimal(left.value + right.value);
    public static decimal operator - (decimal left, decimal right) => new Decimal(left.value - right.value);
    public static decimal operator * (decimal left, decimal right) => new Decimal(left.value * right.value);
    public static decimal operator / (decimal left, decimal right) {
        if (right.value == 0m)
        {
            throw new Std.DivideByZeroException("Division by zero");
        }
        return new Decimal(left.value / right.value);
    }
    public static decimal operator % (decimal left, decimal right) {
        if (right.value == 0m)
        {
            throw new Std.DivideByZeroException("Division by zero");
        }
        return new Decimal(left.value % right.value);
    }
    public static decimal operator - (decimal value) => new Decimal(- value.value);
    public static decimal operator + (decimal value) => value;
    public static decimal operator ++ (decimal value) => new Decimal(value.value + 1m);
    public static decimal operator -- (decimal value) => new Decimal(value.value - 1m);
    public static bool Equals(decimal left, decimal right) => left.value == right.value;
    public bool Equals(decimal other) => value == other.value;
    public static decimal Min(decimal left, decimal right) => left.value <= right.value ?left : right;
    public static decimal Max(decimal left, decimal right) => left.value >= right.value ?left : right;
    public static decimal Abs(decimal value) {
        if (value.value <0m)
        {
            return new Decimal(- value.value);
        }
        return value;
    }
    public bool TryFormat(Span <byte >destination, out usize written) => NumericFormatting.TryFormatDecimal(value, destination,
    out written, null, null);
    public bool TryFormat(Span <byte >destination, out usize written, string format) => NumericFormatting.TryFormatDecimal(value,
    destination, out written, format, null);
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) => NumericFormatting.TryFormatDecimal(value,
    destination, out written, format, culture);
}
