namespace Std.Numeric;
import Std.Globalization;
import Std.Datetime;
public interface IConvertible
{
    bool ToBoolean(IFormatProvider provider);
    char ToChar(IFormatProvider provider);
    string ToString(IFormatProvider provider);
    sbyte ToSByte(IFormatProvider provider);
    byte ToByte(IFormatProvider provider);
    short ToInt16(IFormatProvider provider);
    ushort ToUInt16(IFormatProvider provider);
    int ToInt32(IFormatProvider provider);
    uint ToUInt32(IFormatProvider provider);
    long ToInt64(IFormatProvider provider);
    ulong ToUInt64(IFormatProvider provider);
    nint ToNInt(IFormatProvider provider);
    nuint ToNUInt(IFormatProvider provider);
    isize ToISize(IFormatProvider provider);
    usize ToUSize(IFormatProvider provider);
    Int128 ToInt128(IFormatProvider provider);
    UInt128 ToUInt128(IFormatProvider provider);
    float ToSingle(IFormatProvider provider);
    double ToDouble(IFormatProvider provider);
    Float128 ToFloat128(IFormatProvider provider);
    Decimal ToDecimal(IFormatProvider provider);
    DateTime ToDateTime(IFormatProvider provider);
    Object ToType(Type targetType, IFormatProvider provider);
}
