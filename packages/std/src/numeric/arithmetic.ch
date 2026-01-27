namespace Std.Numeric;
internal static class NumericArithmetic
{
    public static bool TryAddSByte(sbyte left, sbyte right, out sbyte value) {
        var sum = left + right;
        if (sum <NumericConstants.SByteMin || sum >NumericConstants.SByteMax)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToSByte(sum);
        return true;
    }
    public static bool TryAddByte(byte left, byte right, out byte value) {
        var sum = left + right;
        if (sum >0xFF)
        {
            value = 0u8;
            return false;
        }
        value = NumericUnchecked.ToByte(sum);
        return true;
    }
    public static bool TryAddInt16(short left, short right, out short value) {
        var sum = left + right;
        if (sum <NumericConstants.Int16Min || sum >NumericConstants.Int16Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt16(sum);
        return true;
    }
    public static bool TryAddUInt16(ushort left, ushort right, out ushort value) {
        var sum = (uint) left + (uint) right;
        if (sum >NumericConstants.UInt16Max)
        {
            value = 0u16;
            return false;
        }
        value = NumericUnchecked.ToUInt16(sum);
        return true;
    }
    public static bool TryAddInt32(int left, int right, out int value) {
        var sum = (long) left + (long) right;
        if (sum <NumericConstants.Int32Min || sum >NumericConstants.Int32Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt32(sum);
        return true;
    }
    public static bool TryAddInt64(long left, long right, out long value) {
        if ( (right >0 && left >NumericConstants.Int64Max - right) || (right <0 && left <NumericConstants.Int64Min - right))
        {
            value = 0L;
            return false;
        }
        value = left + right;
        return true;
    }
    public static bool TryAddUInt32(uint left, uint right, out uint value) {
        var sum = left + right;
        if (sum <left)
        {
            value = 0u;
            return false;
        }
        value = sum;
        return true;
    }
    public static bool TryAddUInt64(ulong left, ulong right, out ulong value) {
        var sum = left + right;
        if (sum <left)
        {
            value = 0ul;
            return false;
        }
        value = sum;
        return true;
    }
    public static bool TrySubtractSByte(sbyte left, sbyte right, out sbyte value) {
        var diff = left - right;
        if (diff <NumericConstants.SByteMin || diff >NumericConstants.SByteMax)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToSByte(diff);
        return true;
    }
    public static bool TrySubtractByte(byte left, byte right, out byte value) {
        if (left <right)
        {
            value = 0u8;
            return false;
        }
        value = NumericUnchecked.ToByte(left - right);
        return true;
    }
    public static bool TrySubtractInt16(short left, short right, out short value) {
        var diff = left - right;
        if (diff <NumericConstants.Int16Min || diff >NumericConstants.Int16Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt16(diff);
        return true;
    }
    public static bool TrySubtractUInt16(ushort left, ushort right, out ushort value) {
        if (left <right)
        {
            value = 0u16;
            return false;
        }
        value = NumericUnchecked.ToUInt16(left - right);
        return true;
    }
    public static bool TrySubtractInt32(int left, int right, out int value) {
        var diff = (long) left - (long) right;
        if (diff <NumericConstants.Int32Min || diff >NumericConstants.Int32Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt32(diff);
        return true;
    }
    public static bool TrySubtractInt64(long left, long right, out long value) {
        if ( (right >0 && left <NumericConstants.Int64Min + right) || (right <0 && left >NumericConstants.Int64Max + (- right)))
        {
            value = 0L;
            return false;
        }
        value = left - right;
        return true;
    }
    public static bool TrySubtractUInt32(uint left, uint right, out uint value) {
        if (left <right)
        {
            value = 0u;
            return false;
        }
        value = left - right;
        return true;
    }
    public static bool TrySubtractUInt64(ulong left, ulong right, out ulong value) {
        if (left <right)
        {
            value = 0ul;
            return false;
        }
        value = left - right;
        return true;
    }
    public static bool TryMultiplySByte(sbyte left, sbyte right, out sbyte value) {
        var product = left * right;
        if (product <NumericConstants.SByteMin || product >NumericConstants.SByteMax)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToSByte(product);
        return true;
    }
    public static bool TryMultiplyByte(byte left, byte right, out byte value) {
        var product = left * right;
        if (product >0xFF)
        {
            value = 0u8;
            return false;
        }
        value = NumericUnchecked.ToByte(product);
        return true;
    }
    public static bool TryMultiplyInt16(short left, short right, out short value) {
        var product = left * right;
        if (product <NumericConstants.Int16Min || product >NumericConstants.Int16Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt16(product);
        return true;
    }
    public static bool TryMultiplyUInt16(ushort left, ushort right, out ushort value) {
        var product = (uint) left * (uint) right;
        if (product >NumericConstants.UInt16Max)
        {
            value = 0u16;
            return false;
        }
        value = NumericUnchecked.ToUInt16(product);
        return true;
    }
    public static bool TryMultiplyInt32(int left, int right, out int value) {
        var product = (long) left * (long) right;
        if (product <NumericConstants.Int32Min || product >NumericConstants.Int32Max)
        {
            value = 0;
            return false;
        }
        value = NumericUnchecked.ToInt32(product);
        return true;
    }
    public static bool TryMultiplyInt64(long left, long right, out long value) {
        if (left == 0 || right == 0)
        {
            value = 0L;
            return true;
        }
        if (left == NumericConstants.Int64Min && right == - 1)
        {
            value = 0L;
            return false;
        }
        if (right == NumericConstants.Int64Min && left == - 1)
        {
            value = 0L;
            return false;
        }
        var product = left * right;
        if (product / left != right)
        {
            value = 0L;
            return false;
        }
        value = product;
        return true;
    }
    public static bool TryMultiplyUInt32(uint left, uint right, out uint value) {
        var product = (ulong) left * (ulong) right;
        if (product >NumericConstants.UInt32Max)
        {
            value = 0u;
            return false;
        }
        value = NumericUnchecked.ToUInt32(product);
        return true;
    }
    public static bool TryMultiplyUInt64(ulong left, ulong right, out ulong value) {
        if (right != 0ul && left >NumericConstants.UInt64Max / right)
        {
            value = 0ul;
            return false;
        }
        value = left * right;
        return true;
    }
    public static bool TryNegateSByte(sbyte value, out sbyte result) {
        if (value == NumericConstants.SByteMin)
        {
            result = 0;
            return false;
        }
        result = NumericUnchecked.ToSByte(- value);
        return true;
    }
    public static bool TryNegateInt16(short value, out short result) {
        if (value == NumericConstants.Int16Min)
        {
            result = 0;
            return false;
        }
        result = NumericUnchecked.ToInt16(- value);
        return true;
    }
    public static bool TryNegateInt32(int value, out int result) {
        if (value == NumericConstants.Int32Min)
        {
            result = 0;
            return false;
        }
        result = - value;
        return true;
    }
    public static bool TryNegateInt64(long value, out long result) {
        if (value == NumericConstants.Int64Min)
        {
            result = 0L;
            return false;
        }
        result = - value;
        return true;
    }
    public static bool TryAddInt128(int128 left, int128 right, out int128 value) {
        let sum = left + right;
        if ( (right >0 && sum <left) || (right <0 && sum >left))
        {
            value = 0;
            return false;
        }
        value = sum;
        return true;
    }
    public static bool TryAddUInt128(u128 left, u128 right, out u128 value) {
        let sum = left + right;
        if (sum <left)
        {
            value = 0u128;
            return false;
        }
        value = sum;
        return true;
    }
    public static bool TrySubtractInt128(int128 left, int128 right, out int128 value) {
        if ( (right >0 && left <NumericConstants.Int128Min + right) || (right <0 && left >NumericConstants.Int128Max + (- right)))
        {
            value = 0;
            return false;
        }
        value = left - right;
        return true;
    }
    public static bool TrySubtractUInt128(u128 left, u128 right, out u128 value) {
        if (left <right)
        {
            value = 0u128;
            return false;
        }
        value = left - right;
        return true;
    }
    public static bool TryMultiplyInt128(int128 left, int128 right, out int128 value) {
        if (left == 0 || right == 0)
        {
            value = 0;
            return true;
        }
        if (left == NumericConstants.Int128Min && right == - 1)
        {
            value = 0;
            return false;
        }
        if (right == NumericConstants.Int128Min && left == - 1)
        {
            value = 0;
            return false;
        }
        let product = left * right;
        if (product / left != right)
        {
            value = 0;
            return false;
        }
        value = product;
        return true;
    }
    public static bool TryMultiplyUInt128(u128 left, u128 right, out u128 value) {
        if (right != 0u128 && left >NumericConstants.UInt128Max / right)
        {
            value = 0u128;
            return false;
        }
        value = left * right;
        return true;
    }
    public static bool TryNegateInt128(int128 value, out int128 result) {
        if (value == NumericConstants.Int128Min)
        {
            result = 0;
            return false;
        }
        result = - value;
        return true;
    }
}
