namespace Std;
import Std.Numeric;
import Std.Runtime.InteropServices;
import Std.Span;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "nint", kind = "int", bits = 64, signed = true, pointer_sized = true,
aliases = ["nint", "IntPtr", "Std.IntPtr", "Std.Numeric.IntPtr", "System.IntPtr", "isize", "intptr"], c_type = "intptr_t") public readonly struct IntPtr : IComparable, IComparable <IntPtr >, IConvertible, IEquatable <IntPtr >, IParsable <IntPtr >, ISpanParsable <IntPtr >, IUtf8SpanParsable <IntPtr >, IAdditionOperators <IntPtr, IntPtr, IntPtr >, IAdditiveIdentity <IntPtr, IntPtr >, IBinaryInteger <IntPtr >, IBinaryNumber <IntPtr >, IBitwiseOperators <IntPtr, IntPtr, IntPtr >, IComparisonOperators <IntPtr, IntPtr, bool >, IDecrementOperators <IntPtr >, IDivisionOperators <IntPtr, IntPtr, IntPtr >, IEqualityOperators <IntPtr, IntPtr, bool >, IIncrementOperators <IntPtr >, IMinMaxValue <IntPtr >, IModulusOperators <IntPtr, IntPtr, IntPtr >, IMultiplicativeIdentity <IntPtr, IntPtr >, IMultiplyOperators <IntPtr, IntPtr, IntPtr >, INumber <IntPtr >, INumberBase <IntPtr >, IShiftOperators <IntPtr, int, IntPtr >, ISignedNumber <IntPtr >, ISubtractionOperators <IntPtr, IntPtr, IntPtr >, IUnaryNegationOperators <IntPtr, IntPtr >, IUnaryPlusOperators <IntPtr, IntPtr >, IFormattable, ISpanFormattable, IUtf8SpanFormattable, Clone, Copy
{
    private readonly nint value;
    public static nint MinValue => NumericPlatform.IntPtrMinValue;
    public static nint MaxValue => NumericPlatform.IntPtrMaxValue;
    public init(nint value) {
        this.value = value;
    }
    public nint ToIntPtr() => value;
    public static IntPtr From(nint value) => new IntPtr(value);
    public static IntPtr Zero => new IntPtr(NumericPlatform.IntPtrMaxValue - NumericPlatform.IntPtrMaxValue);
    public static IntPtr One => new IntPtr(FromInt32(1));
    public static IntPtr NegativeOne() => new IntPtr(FromInt32(- 1));
    public static IntPtr AdditiveIdentity() => Zero;
    public static IntPtr MultiplicativeIdentity() => One;
    private static int AsInt32(nint ptr) => NumericUnchecked.ToInt32(ptr);
    private static long AsInt64(nint ptr) => NumericUnchecked.ToInt64(ptr);
    private static nint FromInt32(int value) => NumericUnchecked.ToNintFromInt32(value);
    private static nint FromInt64(long value) => NumericUnchecked.ToNintFromInt64(value);
    public string ToString() => Format(null, null);
    public string ToString(IFormatProvider provider) => NumericFormatting.FormatIntPtr(value, "G", ConvertibleHelpers.ResolveCulture(provider));
    public bool ToBoolean(IFormatProvider provider) => ConvertibleHelpers.FromInt64(AsInt64(value));
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(AsInt64(value));
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(AsInt64(value));
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(AsInt64(value));
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(AsInt64(value));
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(AsInt64(value));
    public int ToInt32(IFormatProvider provider) => ConvertibleHelpers.ToInt32Checked(AsInt64(value));
    public uint ToUInt32(IFormatProvider provider) => ConvertibleHelpers.ToUInt32Checked(AsInt64(value));
    public long ToInt64(IFormatProvider provider) => AsInt64(value);
    public ulong ToUInt64(IFormatProvider provider) => ConvertibleHelpers.ToUInt64Checked(AsInt64(value));
    public nint ToNInt(IFormatProvider provider) => value;
    public nuint ToNUInt(IFormatProvider provider) => ConvertibleHelpers.ToNUIntChecked(AsInt64(value));
    public isize ToISize(IFormatProvider provider) => (isize) value;
    public usize ToUSize(IFormatProvider provider) => ConvertibleHelpers.ToUSizeChecked(AsInt64(value));
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(AsInt64(value));
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(AsInt64(value));
    public float ToSingle(IFormatProvider provider) => (float) AsInt64(value);
    public double ToDouble(IFormatProvider provider) => (double) AsInt64(value);
    public Float128 ToFloat128(IFormatProvider provider) => new Float128((double) AsInt64(value));
    public Decimal ToDecimal(IFormatProvider provider) => ConvertibleHelpers.ToDecimalFromNInt(value);
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("IntPtr",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("IntPtr");
    public string Format(string format, string culture) => NumericFormatting.FormatIntPtr(value, format, culture);
    public static IntPtr Parse(string text) {
        if (text == null)
        {
            throw new Std.ArgumentNullException(nameof(text));
        }
        var span = Std.Span.ReadOnlySpan.FromString(text);
        return Parse(span);
    }
    public static bool TryParse(string text, out IntPtr result) {
        var parsed = NumericPlatform.IntPtrMaxValue - NumericPlatform.IntPtrMaxValue;
        if (! NumericParse.TryParseIntPtr (text, out parsed)) {
            result = new IntPtr(FromInt32(0));
            return false;
        }
        result = new IntPtr(parsed);
        return true;
    }
    public static IntPtr Parse(ReadOnlySpan <byte >text) {
        var parsed = NumericPlatform.IntPtrMaxValue - NumericPlatform.IntPtrMaxValue;
        var status = ParseStatus.Invalid;
        if (! NumericParse.TryParseIntPtr (text, out parsed, out status)) {
            NumericParse.ThrowParseException(status, "IntPtr");
        }
        return new IntPtr(parsed);
    }
    public static bool TryParse(ReadOnlySpan <byte >text, out IntPtr result) {
        var parsed = NumericPlatform.IntPtrMaxValue - NumericPlatform.IntPtrMaxValue;
        if (! NumericParse.TryParseIntPtr (text, out parsed)) {
            result = new IntPtr(FromInt32(0));
            return false;
        }
        result = new IntPtr(parsed);
        return true;
    }
    public int CompareTo(IntPtr other) {
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
        throw new Std.ArgumentException("object is not an IntPtr");
    }
    public static int Compare(IntPtr left, IntPtr right) {
        return left.CompareTo(right);
    }
    public static bool operator == (IntPtr left, IntPtr right) => left.value == right.value;
    public static bool operator != (IntPtr left, IntPtr right) => left.value != right.value;
    public static bool operator <(IntPtr left, IntPtr right) => left.value <right.value;
    public static bool operator <= (IntPtr left, IntPtr right) => left.value <= right.value;
    public static bool operator >(IntPtr left, IntPtr right) => left.value >right.value;
    public static bool operator >= (IntPtr left, IntPtr right) => left.value >= right.value;
    public static IntPtr operator + (IntPtr left, IntPtr right) => new IntPtr(left.value + right.value);
    public static IntPtr operator - (IntPtr left, IntPtr right) => new IntPtr(left.value - right.value);
    public static IntPtr operator * (IntPtr left, IntPtr right) => new IntPtr(left.value * right.value);
    public static IntPtr operator / (IntPtr left, IntPtr right) => new IntPtr(left.value / right.value);
    public static IntPtr operator % (IntPtr left, IntPtr right) => new IntPtr(left.value % right.value);
    public static IntPtr operator - (IntPtr value) => new IntPtr(- value.value);
    public static IntPtr operator + (IntPtr value) => value;
    public static IntPtr operator ++ (IntPtr value) => new IntPtr(value.value + FromInt32(1));
    public static IntPtr operator -- (IntPtr value) => new IntPtr(value.value - FromInt32(1));
    public static IntPtr operator & (IntPtr left, IntPtr right) => new IntPtr(left.value & right.value);
    public static IntPtr operator | (IntPtr left, IntPtr right) => new IntPtr(left.value | right.value);
    public static IntPtr operator ^ (IntPtr left, IntPtr right) => new IntPtr(left.value ^ right.value);
    public static IntPtr operator ~ (IntPtr value) => new IntPtr(value.value ^ (nint)(- 1));
    public static IntPtr operator << (IntPtr value, int offset) => new IntPtr(value.value << offset);
    public static IntPtr operator >> (IntPtr value, int offset) => new IntPtr(value.value >> offset);
    public static IntPtr Abs(IntPtr value) => value.value <0 ?new IntPtr(- value.value) : value;
    public static bool Equals(IntPtr left, IntPtr right) => left.value == right.value;
    public bool Equals(IntPtr other) => value == other.value;
    public static IntPtr Min(IntPtr left, IntPtr right) => left.value <= right.value ?left : right;
    public static IntPtr Max(IntPtr left, IntPtr right) => left.value >= right.value ?left : right;
    public static bool TryAdd(IntPtr left, IntPtr right, out IntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0;
            if (! NumericArithmetic.TryAddInt32 (AsInt32 (left.value), AsInt32 (right.value), out raw32)) {
                result = new IntPtr(FromInt32(0));
                return false;
            }
            result = new IntPtr(FromInt32(raw32));
            return true;
        }
        var raw64 = 0L;
        if (! NumericArithmetic.TryAddInt64 (AsInt64 (left.value), AsInt64 (right.value), out raw64)) {
            result = new IntPtr(FromInt32(0));
            return false;
        }
        result = new IntPtr(FromInt64(raw64));
        return true;
    }
    public static bool TrySubtract(IntPtr left, IntPtr right, out IntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0;
            if (! NumericArithmetic.TrySubtractInt32 (AsInt32 (left.value), AsInt32 (right.value), out raw32)) {
                result = new IntPtr(FromInt32(0));
                return false;
            }
            result = new IntPtr(FromInt32(raw32));
            return true;
        }
        var raw64 = 0L;
        if (! NumericArithmetic.TrySubtractInt64 (AsInt64 (left.value), AsInt64 (right.value), out raw64)) {
            result = new IntPtr(FromInt32(0));
            return false;
        }
        result = new IntPtr(FromInt64(raw64));
        return true;
    }
    public static bool TryMultiply(IntPtr left, IntPtr right, out IntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0;
            if (! NumericArithmetic.TryMultiplyInt32 (AsInt32 (left.value), AsInt32 (right.value), out raw32)) {
                result = new IntPtr(FromInt32(0));
                return false;
            }
            result = new IntPtr(FromInt32(raw32));
            return true;
        }
        var raw64 = 0L;
        if (! NumericArithmetic.TryMultiplyInt64 (AsInt64 (left.value), AsInt64 (right.value), out raw64)) {
            result = new IntPtr(FromInt32(0));
            return false;
        }
        result = new IntPtr(FromInt64(raw64));
        return true;
    }
    public static IntPtr ReverseEndianness(IntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return new IntPtr(FromInt32(NumericBitOperations.ReverseEndiannessInt32(AsInt32(value.value))));
        }
        return new IntPtr(FromInt64(NumericBitOperations.ReverseEndiannessInt64(AsInt64(value.value))));
    }
    public static bool IsPowerOfTwo(IntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.IsPowerOfTwoInt32(AsInt32(value.value));
        }
        return NumericBitOperations.IsPowerOfTwoInt64(AsInt64(value.value));
    }
    public string ToString(string format) => Format(format, null);
    public string ToString(string format, string culture) => Format(format, culture);
    public bool TryFormat(Span <byte >destination, out usize written) {
        return NumericFormatting.TryFormatIntPtr(value, destination, out written, null, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format) {
        return NumericFormatting.TryFormatIntPtr(value, destination, out written, format, null);
    }
    public bool TryFormat(Span <byte >destination, out usize written, string format, string culture) {
        return NumericFormatting.TryFormatIntPtr(value, destination, out written, format, culture);
    }
    public static bool TryNegate(IntPtr value, out IntPtr result) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var raw32 = 0;
            if (! NumericArithmetic.TryNegateInt32 (AsInt32 (value.value), out raw32)) {
                result = new IntPtr(FromInt32(0));
                return false;
            }
            result = new IntPtr(FromInt32(raw32));
            return true;
        }
        var raw64 = 0L;
        if (! NumericArithmetic.TryNegateInt64 (AsInt64 (value.value), out raw64)) {
            result = new IntPtr(FromInt32(0));
            return false;
        }
        result = new IntPtr(FromInt64(raw64));
        return true;
    }
    public static int LeadingZeroCount(IntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.LeadingZeroCountInt32(AsInt32(value.value));
        }
        return NumericBitOperations.LeadingZeroCountInt64(AsInt64(value.value));
    }
    public static int TrailingZeroCount(IntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.TrailingZeroCountInt32(AsInt32(value.value));
        }
        return NumericBitOperations.TrailingZeroCountInt64(AsInt64(value.value));
    }
    public static int PopCount(IntPtr value) {
        if (NumericPlatform.PointerBits == 32u)
        {
            return NumericBitOperations.PopCountInt32(AsInt32(value.value));
        }
        return NumericBitOperations.PopCountInt64(AsInt64(value.value));
    }
    public static IntPtr RotateLeft(IntPtr value, int offset) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var rotated = NumericBitOperations.RotateLeftInt32(AsInt32(value.value), offset);
            return new IntPtr(FromInt32(rotated));
        }
        var rotated64 = NumericBitOperations.RotateLeftInt64(AsInt64(value.value), offset);
        return new IntPtr(FromInt64(rotated64));
    }
    public static IntPtr RotateRight(IntPtr value, int offset) {
        if (NumericPlatform.PointerBits == 32u)
        {
            var rotated = NumericBitOperations.RotateRightInt32(AsInt32(value.value), offset);
            return new IntPtr(FromInt32(rotated));
        }
        var rotated64 = NumericBitOperations.RotateRightInt64(AsInt64(value.value), offset);
        return new IntPtr(FromInt64(rotated64));
    }
}
