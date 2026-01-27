namespace Std;
import Std.Runtime.InteropServices;
import Std.Globalization;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "bool", kind = "bool", aliases = ["bool", "Boolean",
"Std.Boolean", "System.Boolean"], c_type = "bool") public readonly struct Boolean : Clone, Copy, IConvertible, IEquatable <bool >
{
    private readonly bool value;
    public init(bool value) {
        this.value = value;
    }
    public bool Equals(bool other) => value == other.value;
    public bool ToBool() => value;
    public string ToString(IFormatProvider provider) => value ?"True" : "False";
    public bool ToBoolean(IFormatProvider provider) => value;
    public char ToChar(IFormatProvider provider) => ConvertibleHelpers.ToCharChecked(value ?1L : 0L);
    public sbyte ToSByte(IFormatProvider provider) => ConvertibleHelpers.ToSByteChecked(value ?1L : 0L);
    public byte ToByte(IFormatProvider provider) => ConvertibleHelpers.ToByteChecked(value ?1L : 0L);
    public short ToInt16(IFormatProvider provider) => ConvertibleHelpers.ToInt16Checked(value ?1L : 0L);
    public ushort ToUInt16(IFormatProvider provider) => ConvertibleHelpers.ToUInt16Checked(value ?1L : 0L);
    public int ToInt32(IFormatProvider provider) => value ?1 : 0;
    public uint ToUInt32(IFormatProvider provider) => value ?1u : 0u;
    public long ToInt64(IFormatProvider provider) => value ?1L : 0L;
    public ulong ToUInt64(IFormatProvider provider) => value ?1ul : 0ul;
    public nint ToNInt(IFormatProvider provider) => value ?NumericUnchecked.ToNintFromInt32(1) : NumericUnchecked.ToNintFromInt32(0);
    public nuint ToNUInt(IFormatProvider provider) => value ?NumericUnchecked.ToNuintNarrow(1u) : NumericUnchecked.ToNuintNarrow(0u);
    public isize ToISize(IFormatProvider provider) => (isize) ToNInt(provider);
    public usize ToUSize(IFormatProvider provider) => (usize) ToNUInt(provider);
    public Int128 ToInt128(IFormatProvider provider) => ConvertibleHelpers.ToInt128Checked(value ?1L : 0L);
    public UInt128 ToUInt128(IFormatProvider provider) => ConvertibleHelpers.ToUInt128Checked(value ?1L : 0L);
    public float ToSingle(IFormatProvider provider) => value ?1.0f : 0.0f;
    public double ToDouble(IFormatProvider provider) => value ?1.0d : 0.0d;
    public Float128 ToFloat128(IFormatProvider provider) => new Float128(value ?1.0d : 0.0d);
    public Decimal ToDecimal(IFormatProvider provider) => value ?1m : 0m;
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) => throw ConvertibleHelpers.InvalidConversion("Boolean",
    "DateTime");
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("Boolean");
    public Self Clone() => this;
}
