import Std;
import Std.Numeric;
import Std.Span;

namespace Exec;

public static class BitConverterIntegerTests
{
    public static int Main()
    {
        if (!ValidateInt32())
        {
            return 1;
        }
        if (!ValidateInt64())
        {
            return 2;
        }
        if (!ValidateUInt16())
        {
            return 3;
        }
        if (!ValidatePointers())
        {
            return 4;
        }
        if (!ValidateBoolAndChar())
        {
            return 5;
        }
        if (!ValidateHelpers())
        {
            return 6;
        }
        return 0;
    }

    private static bool ValidateInt32()
    {
        var buffer = new byte[4];
        var little = Span<byte>.FromArray(ref buffer);
        if (!BitConverter.TryWriteInt32(little, 0x01020304, Endianness.Little, out var written) || written != 4)
        {
            return false;
        }
        if (!Matches(little.AsReadOnly(), new byte[] { 0x04, 0x03, 0x02, 0x01 }))
        {
            return false;
        }

        var bigBuffer = new byte[4];
        var big = Span<byte>.FromArray(ref bigBuffer);
        if (!BitConverter.TryWriteInt32(big, 0x01020304, Endianness.Big, out var bigWritten) || bigWritten != 4)
        {
            return false;
        }
        if (!Matches(big.AsReadOnly(), new byte[] { 0x01, 0x02, 0x03, 0x04 }))
        {
            return false;
        }

        if (!BitConverter.TryReadInt32(big.AsReadOnly(), Endianness.Big, out var roundTrip, out var consumed) || consumed != 4)
        {
            return false;
        }
        if (roundTrip != 0x01020304)
        {
            return false;
        }

        var convenience = BitConverter.GetBytes(0x0A0B0C0D);
        if (!Matches(Span<byte>.FromArray(ref convenience).AsReadOnly(), new byte[] { 0x0D, 0x0C, 0x0B, 0x0A }))
        {
            return false;
        }
        return true;
    }

    private static bool ValidateInt64()
    {
        var buffer = new byte[8];
        var little = Span<byte>.FromArray(ref buffer);
        let value = 0x0102030405060708L;
        if (!BitConverter.TryWriteInt64(little, value, Endianness.Little, out var written) || written != 8)
        {
            return false;
        }
        if (!Matches(
                little.AsReadOnly(),
                new byte[] { 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01 }
            ))
        {
            return false;
        }

        var bigBuffer = new byte[8];
        var big = Span<byte>.FromArray(ref bigBuffer);
        if (!BitConverter.TryWriteInt64(big, value, Endianness.Big, out _))
        {
            return false;
        }
        if (!Matches(
                big.AsReadOnly(),
                new byte[] { 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08 }
            ))
        {
            return false;
        }

        if (!BitConverter.TryReadInt64(big.AsReadOnly(), Endianness.Big, out var roundTrip, out var consumed) || consumed != 8)
        {
            return false;
        }
        if (roundTrip != value)
        {
            return false;
        }
        return true;
    }

    private static bool ValidateUInt16()
    {
        var buffer = new byte[2];
        var span = Span<byte>.FromArray(ref buffer);
        if (!BitConverter.TryWriteUInt16(span, 0x1122u16, Endianness.Little, out var written) || written != 2)
        {
            return false;
        }
        if (!Matches(span.AsReadOnly(), new byte[] { 0x22, 0x11 }))
        {
            return false;
        }

        if (!BitConverter.TryReadUInt16(span.AsReadOnly(), Endianness.Little, out var value, out var consumed) || consumed != 2)
        {
            return false;
        }
        if (value != 0x1122u16)
        {
            return false;
        }
        return true;
    }

    private static bool ValidatePointers()
    {
        var nintBuffer = new byte[__sizeof<nint>()];
        var nintSpan = Span<byte>.FromArray(ref nintBuffer);
        let nintValue = (nint)0x0102030405060708L;
        if (!BitConverter.TryWriteNInt(nintSpan, nintValue, Endianness.Big, out var written) || written != NumericUnchecked.ToInt32(__sizeof<nint>()))
        {
            return false;
        }
        if (!BitConverter.TryReadNInt(nintSpan.AsReadOnly(), Endianness.Big, out var roundTrip, out var consumed) || consumed != NumericUnchecked.ToInt32(__sizeof<nint>()))
        {
            return false;
        }
        if (roundTrip != nintValue)
        {
            return false;
        }

        var usizeBuffer = new byte[__sizeof<usize>()];
        var usizeSpan = Span<byte>.FromArray(ref usizeBuffer);
        let usizeValue = 0x0A0B0C0D0E0F1011usize;
        if (!BitConverter.TryWriteUSize(usizeSpan, usizeValue, Endianness.Little, out var usizeWritten) || usizeWritten != NumericUnchecked.ToInt32(__sizeof<usize>()))
        {
            return false;
        }
        if (!BitConverter.TryReadUSize(usizeSpan.AsReadOnly(), Endianness.Little, out var usizeRoundTrip, out var usizeConsumed) || usizeConsumed != NumericUnchecked.ToInt32(__sizeof<usize>()))
        {
            return false;
        }
        if (usizeRoundTrip != usizeValue)
        {
            return false;
        }

        return true;
    }

    private static bool ValidateBoolAndChar()
    {
        var buffer = new byte[1];
        var span = Span<byte>.FromArray(ref buffer);
        if (!BitConverter.TryWriteBoolean(span, true, Endianness.Little, out var written) || written != 1)
        {
            return false;
        }
        if (buffer[0] != 1u8)
        {
            return false;
        }
        if (!BitConverter.TryReadBoolean(span.AsReadOnly(), Endianness.Big, out var value, out var consumed) || !value || consumed != 1)
        {
            return false;
        }

        buffer[0] = 2u8;
        if (BitConverter.TryReadBoolean(span.AsReadOnly(), Endianness.Little, out var _, out var invalidConsumed) || invalidConsumed != 0)
        {
            return false;
        }

        var charBytes = new byte[2];
        var charSpan = Span<byte>.FromArray(ref charBytes);
        if (!BitConverter.TryWriteChar(charSpan, 'A', Endianness.Little, out var charWritten) || charWritten != 2)
        {
            return false;
        }
        if (!Matches(charSpan.AsReadOnly(), new byte[] { 0x41, 0x00 }))
        {
            return false;
        }
        if (!BitConverter.TryReadChar(charSpan.AsReadOnly(), Endianness.Little, out var character, out var charConsumed) || charConsumed != 2)
        {
            return false;
        }
        if (character != 'A')
        {
            return false;
        }
        return true;
    }

    private static bool ValidateHelpers()
    {
        var bytes = new byte[] { 0xAA, 0xBB, 0xCC, 0xDD };
        var span = Span<byte>.FromArray(ref bytes);
        BitConverter.ReverseEndiannessInPlace(span);
        if (!Matches(span.AsReadOnly(), new byte[] { 0xDD, 0xCC, 0xBB, 0xAA }))
        {
            return false;
        }

        if (BitConverter.ReverseEndianness((ushort)0x1122) != 0x2211u16)
        {
            return false;
        }
        if (BitConverter.ReverseEndianness((uint)0x01020304u) != 0x04030201u)
        {
            return false;
        }
        if (BitConverter.ReverseEndianness(0x0102030405060708ul) != 0x0807060504030201ul)
        {
            return false;
        }
        return true;
    }

    private static bool Matches(ReadOnlySpan<byte> actual, byte[] expected)
    {
        if (actual.Length != expected.Length)
        {
            return false;
        }

        var idx = 0usize;
        while (idx < actual.Length)
        {
            unsafe
            {
                if (actual.Raw.Data.Pointer[idx] != expected[idx])
                {
                    return false;
                }
            }
            idx += 1usize;
        }
        return true;
    }
}
