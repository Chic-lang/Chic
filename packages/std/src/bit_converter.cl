namespace Std;
/// <summary>
/// Minimal stub for BitConverter to satisfy compilation while native implementation is rebuilt.
/// </summary>
public static class BitConverter
{
    public static Endianness NativeEndianness => Endianness.Little;
    public static bool TryWriteSByte(Std.Span.Span <byte >destination, sbyte value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    Numeric.NumericUnchecked.ToByte(value), out bytesWritten);
    public static bool TryWriteByte(Std.Span.Span <byte >destination, byte value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    value, out bytesWritten);
    public static bool TryWriteInt16(Std.Span.Span <byte >destination, short value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    (ushort) value, out bytesWritten);
    public static bool TryWriteUInt16(Std.Span.Span <byte >destination, ushort value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    value, out bytesWritten);
    public static bool TryWriteInt32(Std.Span.Span <byte >destination, int value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    (uint) value, out bytesWritten);
    public static bool TryWriteUInt32(Std.Span.Span <byte >destination, uint value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    value, out bytesWritten);
    public static bool TryWriteInt64(Std.Span.Span <byte >destination, long value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    (ulong) value, out bytesWritten);
    public static bool TryWriteUInt64(Std.Span.Span <byte >destination, ulong value, Endianness order, out int bytesWritten) => TryWriteScalar(destination,
    value, out bytesWritten);
    public static byte[] GetBytes(sbyte value, Endianness order = Endianness.Little) => WriteBytes(Numeric.NumericUnchecked.ToByte(value));
    public static byte[] GetBytes(byte value, Endianness order = Endianness.Little) => WriteBytes(value);
    public static byte[] GetBytes(short value, Endianness order = Endianness.Little) => WriteBytes((ushort) value);
    public static byte[] GetBytes(ushort value, Endianness order = Endianness.Little) => WriteBytes(value);
    public static byte[] GetBytes(int value, Endianness order = Endianness.Little) => WriteBytes((uint) value);
    public static byte[] GetBytes(uint value, Endianness order = Endianness.Little) => WriteBytes(value);
    public static byte[] GetBytes(long value, Endianness order = Endianness.Little) => WriteBytes((ulong) value);
    public static byte[] GetBytes(ulong value, Endianness order = Endianness.Little) => WriteBytes(value);
    public static short ToInt16(Std.Span.ReadOnlySpan <byte >bytes, Endianness order = Endianness.Little) => (short) ToUInt16(bytes);
    public static ushort ToUInt16(Std.Span.ReadOnlySpan <byte >bytes, Endianness order = Endianness.Little) => bytes.Length >= 2 ?(ushort)(bytes[0] | ((ushort) bytes[1] << 8)) : (ushort) 0;
    public static int ToInt32(Std.Span.ReadOnlySpan <byte >bytes, Endianness order = Endianness.Little) => (int) ToUInt32(bytes);
    public static uint ToUInt32(Std.Span.ReadOnlySpan <byte >bytes, Endianness order = Endianness.Little) {
        if (bytes.Length <4)
        {
            return 0u;
        }
        return bytes[0] | ((uint) bytes[1] << 8) | ((uint) bytes[2] << 16) | ((uint) bytes[3] << 24);
    }
    public static long ToInt64(Std.Span.ReadOnlySpan <byte >bytes, Endianness order = Endianness.Little) => (long) ToUInt64(bytes);
    public static ulong ToUInt64(Std.Span.ReadOnlySpan <byte >bytes, Endianness order = Endianness.Little) {
        if (bytes.Length <8)
        {
            return 0ul;
        }
        return bytes[0] | ((ulong) bytes[1] << 8) | ((ulong) bytes[2] << 16) | ((ulong) bytes[3] << 24) | ((ulong) bytes[4] << 32) | ((ulong) bytes[5] << 40) | ((ulong) bytes[6] << 48) | ((ulong) bytes[7] << 56);
    }
    private static bool TryWriteScalar(Std.Span.Span <byte >destination, ulong value, out int bytesWritten) {
        if (destination.Length <8)
        {
            bytesWritten = 0;
            return false;
        }
        var idx = 0;
        while (idx <8)
        {
            destination[idx] = Numeric.NumericUnchecked.ToByte((value >> (idx * 8)) & 0xFF);
            idx += 1;
        }
        bytesWritten = 8;
        return true;
    }
    private static byte[] WriteBytes(ulong value) {
        var buffer = new byte[8];
        var idx = 0;
        while (idx <8)
        {
            buffer[idx] = Numeric.NumericUnchecked.ToByte((value >> (idx * 8)) & 0xFF);
            idx += 1;
        }
        return buffer;
    }
}
