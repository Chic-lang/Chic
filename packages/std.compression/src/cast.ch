namespace Std.IO.Compression;
/// <summary>Unchecked numeric casts used within std.compression to avoid depending on internal Std helpers.</summary>
internal static class CompressionCast
{
    public static byte ToByte(int value) {
        unchecked {
            return(byte) value;
        }
    }
    public static byte ToByte(uint value) {
        unchecked {
            return(byte) value;
        }
    }
    public static byte ToByte(long value) {
        unchecked {
            return(byte) value;
        }
    }
    public static ushort ToUInt16(int value) {
        unchecked {
            return(ushort) value;
        }
    }
    public static ushort ToUInt16(uint value) {
        unchecked {
            return(ushort) value;
        }
    }
    public static uint ToUInt32(int value) {
        unchecked {
            return(uint) value;
        }
    }
    public static uint ToUInt32(uint value) {
        unchecked {
            return value;
        }
    }
    public static uint ToUInt32(byte value) {
        unchecked {
            return(uint) value;
        }
    }
    public static uint ToUInt32(long value) {
        unchecked {
            return(uint) value;
        }
    }
    public static uint ToUInt32(ulong value) {
        unchecked {
            return(uint) value;
        }
    }
    public static int ToInt32(long value) {
        unchecked {
            return(int) value;
        }
    }
    public static int ToInt32(uint value) {
        unchecked {
            return(int) value;
        }
    }
    public static int ToInt32(usize value) {
        unchecked {
            return(int) value;
        }
    }
    public static usize ToUSize(int value) {
        unchecked {
            return(usize) value;
        }
    }
    public static usize ToUSize(uint value) {
        unchecked {
            return(usize) value;
        }
    }
    public static usize ToUSize(long value) {
        unchecked {
            return(usize) value;
        }
    }
    public static long ToInt64(ulong value) {
        unchecked {
            return(long) value;
        }
    }
    public static long ToInt64(int value) {
        unchecked {
            return(long) value;
        }
    }
}
