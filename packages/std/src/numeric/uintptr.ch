namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
/// <summary>
/// Pointer-sized unsigned integer facade. Mirrors the core helper but lives in the stdlib so
/// numeric operator support and generic math integration can use the full language surface.
/// </summary>
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "nuint", kind = "int", bits = 64, signed = false,
pointer_sized = true, aliases = ["nuint", "UIntPtr", "Std.UIntPtr", "Std.Numeric.UIntPtr", "System.UIntPtr", "usize", "uintptr"],
c_type = "uintptr_t") public readonly struct UIntPtr : IComparable, IComparable <UIntPtr >, IConvertible, IEquatable <UIntPtr >, IParsable <UIntPtr >, ISpanParsable <UIntPtr >, IUtf8SpanParsable <UIntPtr >, IAdditionOperators <UIntPtr, UIntPtr, UIntPtr >, IAdditiveIdentity <UIntPtr, UIntPtr >, IBinaryInteger <UIntPtr >, IBinaryNumber <UIntPtr >, IBitwiseOperators <UIntPtr, UIntPtr, UIntPtr >, IComparisonOperators <UIntPtr, UIntPtr, bool >, IDecrementOperators <UIntPtr >, IDivisionOperators <UIntPtr, UIntPtr, UIntPtr >, IEqualityOperators <UIntPtr, UIntPtr, bool >, IIncrementOperators <UIntPtr >, IMinMaxValue <UIntPtr >, IModulusOperators <UIntPtr, UIntPtr, UIntPtr >, IMultiplicativeIdentity <UIntPtr, UIntPtr >, IMultiplyOperators <UIntPtr, UIntPtr, UIntPtr >, INumber <UIntPtr >, INumberBase <UIntPtr >, IShiftOperators <UIntPtr, int, UIntPtr >, ISubtractionOperators <UIntPtr, UIntPtr, UIntPtr >, IUnaryPlusOperators <UIntPtr, UIntPtr >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly nuint value;
    public const nuint MinValue = 0usize;
    public static nuint MaxValue => NumericPlatform.UIntPtrMaxValue;
    public init(nuint value) {
        this.value = value;
    }
    public nuint ToUIntPtr() => value;
    public static UIntPtr From(nuint value) => new UIntPtr(value);
    public static UIntPtr Zero {
        get {
            return new UIntPtr(0usize);
        }
    }
    public static UIntPtr One {
        get {
            return new UIntPtr(FromUInt32(1));
        }
    }
    public static UIntPtr AdditiveIdentity() => Zero;
    public static UIntPtr MultiplicativeIdentity() => One;
    private static uint AsUInt32(nuint ptr) => NumericUnchecked.ToUInt32(ptr);
    private static ulong AsUInt64(nuint ptr) => NumericUnchecked.ToUInt64(ptr);
    private static nuint FromUInt32(uint value) => NumericUnchecked.ToNuintNarrow(value);
    private static nuint FromUInt64(ulong value) => NumericUnchecked.ToNuintWiden(value);
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatUIntPtr(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromUInt64(AsUInt64(value));
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(AsUInt64(value));
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(AsUInt64(value));
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(AsUInt64(value));
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(AsUInt64(value));
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(AsUInt64(value));
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(AsUInt64(value));
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(AsUInt64(value));
    public long ToInt64(IFormatProvider provider) => ConvertibleHelpers.ToInt64Checked(AsUInt64(value));
    public ulong ToUInt64(IFormatProvider provider) => AsUInt64(value);
    public nint ToNInt(IFormatProvider provider) => NumericUnchecked.ToNintFromPtr(value);
    public nuint ToNUInt(IFormatProvider provider) => value;
    public isize ToISize(IFormatProvider provider) => (isize) NumericUnchecked.ToNintFromPtr(value);
    public usize ToUSize(IFormatProvider provider) => (usize) value;
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(AsUInt64(value));
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(AsUInt64(value));
    public float ToSingle(IFormatProvider provider) => (float) AsUInt64(value);
    public double ToDouble(IFormatProvider provider) => (double) AsUInt64(value);
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) AsUInt64(value));
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromNUInt(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("UIntPtr",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("UIntPtr");
    public string Format(string format, string culture) => NumericFormatting.FormatUIntPtr(value, format, culture);
    public static UIntPtr Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out UIntPtr result) {
        var parsed = 0u;
        if (!NumericParse.TryParseUIntPtr (text, out parsed)) {
            result = new UIntPtr(0u);
            return false;
        }
        result = new UIntPtr(parsed);
        return true;
    }
    public static UIntPtr Parse(ReadOnlySpan <byte >text) {
        var parsed = 0u;
        var status = ParseStatus.Invalid;
        if (!NumericParse.TryParseUIntPtr (text, out parsed, out status)) {
            NumericParse.ThrowParseException(status, "UIntPtr");
        }
        return new UIntPtr(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out UIntPtr result) {
        var parsed = 0u;
        if (!NumericParse.TryParseUIntPtr (text, out parsed)) {
            result = new UIntPtr(0u);
            return false;
        }
        result = new UIntPtr(parsed);
        return true;
    }
    public int CompareTo(UIntPtr other) {
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
        throw new Std.ArgumentException("object is not a UIntPtr");
    }
    public static int Compare(UIntPtr left, UIntPtr right) => left.CompareTo(right);
    public static bool operator == (UIntPtr left, UIntPtr right) => left.value == right.value;
    public static bool operator != (UIntPtr left, UIntPtr right) => left.value != right.value;
    public static bool operator <(UIntPtr left, UIntPtr right) => left.value <right.value;
    public static bool operator <= (UIntPtr left, UIntPtr right) => left.value <= right.value;
    public static bool operator >(UIntPtr left, UIntPtr right) => left.value >right.value;
    public static bool operator >= (UIntPtr left, UIntPtr right) => left.value >= right.value;
    public static UIntPtr operator + (UIntPtr left, UIntPtr right) => new UIntPtr(left.value + right.value);
    public static UIntPtr operator - (UIntPtr left, UIntPtr right) => new UIntPtr(left.value - right.value);
    public static UIntPtr operator * (UIntPtr left, UIntPtr right) => new UIntPtr(left.value * right.value);
    public static UIntPtr operator / (UIntPtr left, UIntPtr right) => new UIntPtr(left.value / right.value);
    public static UIntPtr operator % (UIntPtr left, UIntPtr right) => new UIntPtr(left.value % right.value);
    public static UIntPtr operator + (UIntPtr value) => value;
    public static UIntPtr operator ++ (UIntPtr value) => new UIntPtr(value.value + 1usize);
    public static UIntPtr operator -- (UIntPtr value) => new UIntPtr(value.value - 1usize);
    public static UIntPtr operator & (UIntPtr left, UIntPtr right) => new UIntPtr(left.value & right.value);
    public static UIntPtr operator | (UIntPtr left, UIntPtr right) => new UIntPtr(left.value | right.value);
    public static UIntPtr operator ^ (UIntPtr left, UIntPtr right) => new UIntPtr(left.value ^ right.value);
    public static UIntPtr operator ~ (UIntPtr value) => new UIntPtr(value.value ^ NumericPlatform.UIntPtrMaxValue);
    public static UIntPtr operator << (UIntPtr value, int offset) => new UIntPtr(value.value << offset);
    public static UIntPtr operator >> (UIntPtr value, int offset) => new UIntPtr(value.value >> offset);
    public static UIntPtr Abs(UIntPtr value) => value;
    public static bool Equals(UIntPtr left, UIntPtr right) => left.value == right.value;
    public bool Equals(UIntPtr other) => value == other.value;
    public static UIntPtr Min(UIntPtr left, UIntPtr right) => left.value <= right.value ?left : right;
    public static UIntPtr Max(UIntPtr left, UIntPtr right) => left.value >= right.value ?left : right;
    public static bool TryAdd(UIntPtr left, UIntPtr right, out UIntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0u;
            if (!NumericArithmetic.TryAddUInt32 (AsUInt32 (left.value), AsUInt32 (right.value), out raw32)) {
                result = new UIntPtr(0u);
                return false;
            }
            result = new UIntPtr(FromUInt32(raw32));
            return true;
        }
        var raw64 = 0ul;
        if (!NumericArithmetic.TryAddUInt64 (AsUInt64 (left.value), AsUInt64 (right.value), out raw64)) {
            result = new UIntPtr(0u);
            return false;
        }
        result = new UIntPtr(FromUInt64(raw64));
        return true;
    }
    public static bool TrySubtract(UIntPtr left, UIntPtr right, out UIntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0u;
            if (!NumericArithmetic.TrySubtractUInt32 (AsUInt32 (left.value), AsUInt32 (right.value), out raw32)) {
                result = new UIntPtr(0u);
                return false;
            }
            result = new UIntPtr(FromUInt32(raw32));
            return true;
        }
        var raw64 = 0ul;
        if (!NumericArithmetic.TrySubtractUInt64 (AsUInt64 (left.value), AsUInt64 (right.value), out raw64)) {
            result = new UIntPtr(0u);
            return false;
        }
        result = new UIntPtr(FromUInt64(raw64));
        return true;
    }
    public static bool TryMultiply(UIntPtr left, UIntPtr right, out UIntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0u;
            if (!NumericArithmetic.TryMultiplyUInt32 (AsUInt32 (left.value), AsUInt32 (right.value), out raw32)) {
                result = new UIntPtr(0u);
                return false;
            }
            result = new UIntPtr(FromUInt32(raw32));
            return true;
        }
        var raw64 = 0ul;
        if (!NumericArithmetic.TryMultiplyUInt64 (AsUInt64 (left.value), AsUInt64 (right.value), out raw64)) {
            result = new UIntPtr(0u);
            return false;
        }
        result = new UIntPtr(FromUInt64(raw64));
        return true;
    }
    public static int LeadingZeroCount(UIntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.LeadingZeroCountUInt32(AsUInt32(value.value));
        }
        return NumericBitOperations.LeadingZeroCountUInt64(AsUInt64(value.value));
    }
    public static int TrailingZeroCount(UIntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.TrailingZeroCountUInt32(AsUInt32(value.value));
        }
        return NumericBitOperations.TrailingZeroCountUInt64(AsUInt64(value.value));
    }
    public static int PopCount(UIntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.PopCountUInt32(AsUInt32(value.value));
        }
        return NumericBitOperations.PopCountUInt64(AsUInt64(value.value));
    }
    public static UIntPtr RotateLeft(UIntPtr value, int offset) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var rotated = NumericBitOperations.RotateLeftUInt32(AsUInt32(value.value), offset);
            return new UIntPtr(FromUInt32(rotated));
        }
        var rotated64 = NumericBitOperations.RotateLeftUInt64(AsUInt64(value.value), offset);
        return new UIntPtr(FromUInt64(rotated64));
    }
    public static UIntPtr RotateRight(UIntPtr value, int offset) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var rotated = NumericBitOperations.RotateRightUInt32(AsUInt32(value.value), offset);
            return new UIntPtr(FromUInt32(rotated));
        }
        var rotated64 = NumericBitOperations.RotateRightUInt64(AsUInt64(value.value), offset);
        return new UIntPtr(FromUInt64(rotated64));
    }
    public static UIntPtr ReverseEndianness(UIntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return new UIntPtr(FromUInt32(NumericBitOperations.ReverseEndiannessUInt32(AsUInt32(value.value))));
        }
        return new UIntPtr(FromUInt64(NumericBitOperations.ReverseEndiannessUInt64(AsUInt64(value.value))));
    }
    public static bool IsPowerOfTwo(UIntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.IsPowerOfTwoUInt32(AsUInt32(value.value));
        }
        return NumericBitOperations.IsPowerOfTwoUInt64(AsUInt64(value.value));
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatUIntPtr(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatUIntPtr(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatUIntPtr(value, destination, out written, format, culture);
    }
}
