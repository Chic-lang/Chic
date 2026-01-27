namespace Std.Numeric;
/// Helpers for lossy numeric conversions that have already been range-checked.
public static class NumericUnchecked
{
    public static sbyte ToSByte(sbyte value) => value;
    public static sbyte ToSByte(byte value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(short value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(ushort value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(int value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(uint value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(long value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(ulong value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(isize value) {
        unchecked {
            return (sbyte) value;
        }
    }
    public static sbyte ToSByte(usize value) {
        unchecked {
            return (sbyte) value;
        }
    }

    public static byte ToByte(byte value) => value;
    public static byte ToByte(sbyte value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(char value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(short value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(ushort value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(int value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(uint value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(long value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(ulong value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(isize value) {
        unchecked {
            return (byte) value;
        }
    }
    public static byte ToByte(usize value) {
        unchecked {
            return (byte) value;
        }
    }

    public static char ToChar(char value) => value;
    public static char ToChar(byte value) {
        unchecked {
            return (char) value;
        }
    }
    public static char ToChar(ushort value) {
        unchecked {
            return (char) value;
        }
    }
    public static char ToChar(int value) {
        unchecked {
            return (char) value;
        }
    }
    public static char ToChar(uint value) {
        unchecked {
            return (char) value;
        }
    }
    public static char ToChar(long value) {
        unchecked {
            return (char) value;
        }
    }
    public static char ToChar(ulong value) {
        unchecked {
            return (char) value;
        }
    }

    public static short ToInt16(short value) => value;
    public static short ToInt16(byte value) {
        unchecked {
            return (short) value;
        }
    }
    public static short ToInt16(ushort value) {
        unchecked {
            return (short) value;
        }
    }
    public static short ToInt16(int value) {
        unchecked {
            return (short) value;
        }
    }
    public static short ToInt16(uint value) {
        unchecked {
            return (short) value;
        }
    }
    public static short ToInt16(long value) {
        unchecked {
            return (short) value;
        }
    }
    public static short ToInt16(ulong value) {
        unchecked {
            return (short) value;
        }
    }

    public static ushort ToUInt16(ushort value) => value;
    public static ushort ToUInt16(byte value) {
        unchecked {
            return (ushort) value;
        }
    }
    public static ushort ToUInt16(short value) {
        unchecked {
            return (ushort) value;
        }
    }
    public static ushort ToUInt16(int value) {
        unchecked {
            return (ushort) value;
        }
    }
    public static ushort ToUInt16(uint value) {
        unchecked {
            return (ushort) value;
        }
    }
    public static ushort ToUInt16(long value) {
        unchecked {
            return (ushort) value;
        }
    }
    public static ushort ToUInt16(ulong value) {
        unchecked {
            return (ushort) value;
        }
    }

    public static int ToInt32(int value) => value;
    public static int ToInt32(byte value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(sbyte value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(short value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(ushort value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(uint value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(isize value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(usize value) {
        unchecked {
            return (int) value;
        }
    }
    public static int ToInt32(long value) {
        unchecked {
            return (int) value;
        }
    }

    public static long ToInt64(long value) => value;
    public static long ToInt64(int value) {
        unchecked {
            return (long) value;
        }
    }
    public static long ToInt64(uint value) {
        unchecked {
            return (long) value;
        }
    }
    public static long ToInt64(ulong value) {
        unchecked {
            return (long) value;
        }
    }

    public static uint ToUInt32(char value) {
        unchecked {
            return (uint) value;
        }
    }
    public static uint ToUInt32(ulong value) {
        unchecked {
            return (uint) value;
        }
    }
    public static uint ToUInt32(uint value) => value;
    public static uint ToUInt32(ushort value) {
        unchecked {
            return (uint) value;
        }
    }
    public static uint ToUInt32(short value) {
        unchecked {
            return (uint) value;
        }
    }
    public static uint ToUInt32(byte value) {
        unchecked {
            return (uint) value;
        }
    }
    public static uint ToUInt32(sbyte value) {
        unchecked {
            return (uint) value;
        }
    }
    public static uint ToUInt32(int value) {
        unchecked {
            return (uint) value;
        }
    }

    public static usize ToUSize(usize value) => value;
    public static usize ToUSize(uint value) {
        unchecked {
            return (usize) value;
        }
    }
    public static usize ToUSize(int value) {
        unchecked {
            return (usize) value;
        }
    }
    public static usize ToUSize(isize value) {
        unchecked {
            return (usize) value;
        }
    }
    public static usize ToUSize(long value) {
        unchecked {
            return (usize) value;
        }
    }
    public static usize ToUSize(ulong value) {
        unchecked {
            return (usize) value;
        }
    }

    public static isize ToISize(usize value) {
        unchecked {
            return (isize) value;
        }
    }

    public static ulong ToUInt64(ulong value) => value;
    public static ulong ToUInt64(long value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(int value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(uint value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(ushort value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(short value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(byte value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(sbyte value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong ToUInt64(nuint value) {
        unchecked {
            return (ulong) value;
        }
    }

    public static nint ToNintNarrow(uint value) {
        unchecked {
            return (nint) value;
        }
    }
    public static nint ToNintWiden(ulong value) {
        unchecked {
            return (nint) value;
        }
    }
    public static nint ToNintFromPtr(nuint value) {
        unchecked {
            return (nint) value;
        }
    }
    public static nuint ToNuintNarrow(uint value) {
        unchecked {
            return (nuint) value;
        }
    }
    public static nuint ToNuintWiden(ulong value) {
        unchecked {
            return (nuint) value;
        }
    }
    public static nuint ToNuintFromPtr(nint value) {
        unchecked {
            return (nuint) value;
        }
    }
    public static nint ToNintFromInt32(int value) {
        unchecked {
            return (nint) value;
        }
    }
    public static nint ToNintFromInt64(long value) {
        unchecked {
            return (nint) value;
        }
    }

    public static u128 ToUInt128(long value) {
        unchecked {
            return (u128) value;
        }
    }
    public static u128 ToUInt128(ulong value) {
        unchecked {
            return (u128) value;
        }
    }
    public static u128 ToUInt128(int128 value) {
        unchecked {
            return (u128) value;
        }
    }
    public static int128 ToInt128(u128 value) {
        unchecked {
            return (int128) value;
        }
    }
    public static ulong ToUInt64FromUInt128(u128 value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static ulong HighUInt64FromUInt128(u128 value) {
        unchecked {
            return (ulong) (value >> 64);
        }
    }

    public static double ToFloat64(long value) {
        unchecked {
            return (double) value;
        }
    }
    public static double ToFloat64(int value) {
        unchecked {
            return (double) value;
        }
    }
    public static double ToFloat64(uint value) {
        unchecked {
            return (double) value;
        }
    }
    public static double ToFloat64(ulong value) {
        unchecked {
            return (double) value;
        }
    }
    public static ulong ToUInt64(double value) {
        unchecked {
            return (ulong) value;
        }
    }
    public static double ToFloat64(int128 value) {
        unchecked {
            return (double) value;
        }
    }
    public static double ToFloat64(u128 value) {
        unchecked {
            return (double) value;
        }
    }
    public static float ToFloat32(int128 value) {
        unchecked {
            return (float) value;
        }
    }
    public static float ToFloat32(u128 value) {
        unchecked {
            return (float) value;
        }
    }
    public static int128 ToInt128(double value) {
        unchecked {
            return (int128) value;
        }
    }
    public static int128 ToInt128(float value) {
        unchecked {
            return (int128) value;
        }
    }
    public static u128 ToUInt128(double value) {
        unchecked {
            return (u128) value;
        }
    }
    public static u128 ToUInt128(float value) {
        unchecked {
            return (u128) value;
        }
    }
}

