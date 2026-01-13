import Std;
import Std.Memory;
import Std.Numeric;
import Std.Runtime.Native;
import Std.Runtime.Collections;
import Std.Span;

namespace Exec;

public static class BitConverterDecimalUuidTests
{
    public static int Main()
    {
        if (!ValidateDecimalLayouts())
        {
            return 1;
        }
        if (!ValidateUuidLayouts())
        {
            return 2;
        }
        return 0;
    }

    private static bool ValidateDecimalLayouts()
    {
        let value = 12345.625m;
        let parts = ExtractParts(value);
        let expectedLittle = ConcatLittle(parts);
        let actualLittle = BitConverter.GetBytes(value, Endianness.Little);
        if (!Matches(actualLittle, expectedLittle))
        {
            return false;
        }

        let expectedBig = ConcatBig(parts);
        let actualBig = BitConverter.GetBytes(value, Endianness.Big);
        if (!Matches(actualBig, expectedBig))
        {
            return false;
        }

        if (!BitConverter.TryReadDecimal(Span<byte>.FromArray(ref actualLittle).AsReadOnly(), Endianness.Little, out var roundTrip, out var consumed) || consumed != NumericUnchecked.ToInt32(__sizeof<decimal>()))
        {
            return false;
        }
        if (roundTrip != value)
        {
            return false;
        }

        var samples = new decimal[] { 0m, 1m, -1m, Decimal.MaxValue };
        var idx = 0usize;
        while (idx < samples.Length)
        {
            var current = samples[idx];
            var bytes = BitConverter.GetBytes(current, Endianness.Big);
            if (!BitConverter.TryReadDecimal(Span<byte>.FromArray(ref bytes).AsReadOnly(), Endianness.Big, out var decoded, out var read) || read != NumericUnchecked.ToInt32(__sizeof<decimal>()))
            {
                return false;
            }
            if (decoded != current)
            {
                return false;
            }
            idx += 1usize;
        }

        return true;
    }

    private static bool ValidateUuidLayouts()
    {
        let value = Uuid.Parse("00112233-4455-6677-8899-aabbccddeeff");
        var bigBuffer = Span<byte>.StackAlloc(16usize);
        if (!BitConverter.TryWriteUuid(bigBuffer, value, Endianness.Big, out var written) || written != 16)
        {
            return false;
        }
        if (!Matches(bigBuffer.AsReadOnly(), new byte[] { 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF }))
        {
            return false;
        }

        if (!BitConverter.TryReadUuid(bigBuffer.AsReadOnly(), Endianness.Big, out var decoded, out var consumed) || consumed != 16)
        {
            return false;
        }
        if (decoded != value)
        {
            return false;
        }

        var littleBuffer = Span<byte>.StackAlloc(16usize);
        if (!BitConverter.TryWriteUuid(littleBuffer, value, Endianness.Little, out var littleWritten) || littleWritten != 16)
        {
            return false;
        }
        if (!Matches(littleBuffer.AsReadOnly(), new byte[] { 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00 }))
        {
            return false;
        }

        var samples = new Uuid[] { Uuid.Empty, Uuid.Parse("fedcba98-7654-3210-fedc-ba9876543210") };
        var idx = 0usize;
        while (idx < samples.Length)
        {
            var current = samples[idx];
            var bytes = BitConverter.GetBytes(current, Endianness.Little);
            if (!BitConverter.TryReadUuid(Span<byte>.FromArray(ref bytes).AsReadOnly(), Endianness.Little, out var decodedUuid, out var read) || read != 16)
            {
                return false;
            }
            if (decodedUuid != current)
            {
                return false;
            }
            idx += 1usize;
        }

        return true;
    }

    private static Decimal128Parts ExtractParts(decimal value)
    {
        var slot = MaybeUninit<Decimal128Parts>.Uninit();
        let dest = slot.AsValueMutPtr();
        let size = __sizeof<decimal>();
        unsafe
        {
            var source = Std.Runtime.Collections.ValuePointer.CreateConst(
                Std.Numeric.PointerIntrinsics.AsByteConst(&value),
                size,
                __alignof<decimal>()
            );
            GlobalAllocator.Copy(dest, source, size);
        }
        return slot.AssumeInit();
    }

    private static byte[] ConcatLittle(Decimal128Parts parts)
    {
        var buffer = new byte[16];
        WriteUInt32Little(buffer, parts.lo, 0usize);
        WriteUInt32Little(buffer, parts.mid, 4usize);
        WriteUInt32Little(buffer, parts.hi, 8usize);
        WriteUInt32Little(buffer, parts.flags, 12usize);
        return buffer;
    }

    private static byte[] ConcatBig(Decimal128Parts parts)
    {
        var buffer = new byte[16];
        WriteUInt32Big(buffer, parts.lo, 0usize);
        WriteUInt32Big(buffer, parts.mid, 4usize);
        WriteUInt32Big(buffer, parts.hi, 8usize);
        WriteUInt32Big(buffer, parts.flags, 12usize);
        return buffer;
    }

    private static void WriteUInt32Little(byte[] buffer, uint value, usize offset)
    {
        unsafe
        {
            buffer[offset] = (byte)(value & 0xFFu);
            buffer[offset + 1usize] = (byte)((value >> 8) & 0xFFu);
            buffer[offset + 2usize] = (byte)((value >> 16) & 0xFFu);
            buffer[offset + 3usize] = (byte)((value >> 24) & 0xFFu);
        }
    }

    private static void WriteUInt32Big(byte[] buffer, uint value, usize offset)
    {
        unsafe
        {
            buffer[offset] = (byte)((value >> 24) & 0xFFu);
            buffer[offset + 1usize] = (byte)((value >> 16) & 0xFFu);
            buffer[offset + 2usize] = (byte)((value >> 8) & 0xFFu);
            buffer[offset + 3usize] = (byte)(value & 0xFFu);
        }
    }

    private static bool Matches(byte[] actual, byte[] expected)
    {
        return Matches(Span<byte>.FromArray(ref actual).AsReadOnly(), expected);
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
