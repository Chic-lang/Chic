import Std;
import Std.Memory;
import Std.Numeric;
import Std.Span;

namespace Exec;

public static class BitConverterFloatTests
{
    public static int Main()
    {
        if (!ValidateFloat16Vectors())
        {
            return 1;
        }
        if (!ValidateFloat32Vectors())
        {
            return 2;
        }
        if (!ValidateFloat64Vectors())
        {
            return 3;
        }
        if (!ValidateFloat128RoundTrip())
        {
            return 4;
        }
        return 0;
    }

    private static bool ValidateFloat16Vectors()
    {
        let oneBits = 0x3C00u16;
        let one = FromUIntToFloat16(oneBits);
        var bytes = BitConverter.GetBytes(one, Endianness.Little);
        if (!Matches(bytes, new byte[] { 0x00, 0x3C }))
        {
            return false;
        }

        var bigBuffer = new byte[2];
        var bigSpan = Span<byte>.FromArray(ref bigBuffer);
        if (!BitConverter.TryWriteFloat16(bigSpan, one, Endianness.Big, out var written) || written != 2)
        {
            return false;
        }
        if (!Matches(bigBuffer, new byte[] { 0x3C, 0x00 }))
        {
            return false;
        }

        let negZero = FromUIntToFloat16(0x8000u16);
        var negZeroBytes = BitConverter.GetBytes(negZero, Endianness.Little);
        if (!Matches(negZeroBytes, new byte[] { 0x00, 0x80 }))
        {
            return false;
        }

        let nanBits = 0x7E01u16;
        let nan = FromUIntToFloat16(nanBits);
        var nanBytes = BitConverter.GetBytes(nan, Endianness.Little);
        if (!Matches(nanBytes, new byte[] { 0x01, 0x7E }))
        {
            return false;
        }
        if (!BitConverter.TryReadFloat16(Span<byte>.FromArray(ref nanBytes).AsReadOnly(), Endianness.Little, out var nanRoundTrip, out var consumed) || consumed != 2)
        {
            return false;
        }
        var nanRoundTripBytes = BitConverter.GetBytes(nanRoundTrip, Endianness.Little);
        if (!Matches(nanRoundTripBytes, nanBytes))
        {
            return false;
        }

        let infinity = FromUIntToFloat16(0x7C00u16);
        var infBytes = BitConverter.GetBytes(infinity, Endianness.Big);
        if (!Matches(infBytes, new byte[] { 0x7C, 0x00 }))
        {
            return false;
        }

        var seeds = new ushort[] { 0x0000u16, 0x3C00u16, 0xBC00u16, 0x3555u16, 0x7E01u16 };
        var idx = 0usize;
        while (idx < seeds.Length)
        {
            let bits = seeds[idx];
            let value = FromUIntToFloat16(bits);
            var buffer = Span<byte>.StackAlloc(2usize);
            if (!BitConverter.TryWriteFloat16(buffer, value, Endianness.Little, out var count) || count != 2)
            {
                return false;
            }
            if (!BitConverter.TryReadFloat16(buffer.AsReadOnly(), Endianness.Little, out var decoded, out var readCount) || readCount != 2)
            {
                return false;
            }
            var decodedBytes = BitConverter.GetBytes(decoded, Endianness.Little);
            if (!Matches(buffer.AsReadOnly(), decodedBytes))
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }

    private static bool ValidateFloat32Vectors()
    {
        let oneBits = 0x3F800000u;
        let one = FromUIntToFloat(oneBits);
        var bytes = BitConverter.GetBytes(one, Endianness.Little);
        if (!Matches(bytes, new byte[] { 0x00, 0x00, 0x80, 0x3F }))
        {
            return false;
        }

        var bigBuffer = new byte[4];
        var bigSpan = Span<byte>.FromArray(ref bigBuffer);
        if (!BitConverter.TryWriteSingle(bigSpan, one, Endianness.Big, out var written) || written != 4)
        {
            return false;
        }
        if (!Matches(bigBuffer, new byte[] { 0x3F, 0x80, 0x00, 0x00 }))
        {
            return false;
        }

        let negZero = FromUIntToFloat(0x80000000u);
        var negZeroBytes = BitConverter.GetBytes(negZero, Endianness.Little);
        if (!Matches(negZeroBytes, new byte[] { 0x00, 0x00, 0x00, 0x80 }))
        {
            return false;
        }

        let nanBits = 0x7FC00001u;
        let nan = FromUIntToFloat(nanBits);
        var nanBytes = BitConverter.GetBytes(nan, Endianness.Little);
        if (!Matches(nanBytes, new byte[] { 0x01, 0x00, 0xC0, 0x7F }))
        {
            return false;
        }
        if (!BitConverter.TryReadSingle(Span<byte>.FromArray(ref nanBytes).AsReadOnly(), Endianness.Little, out var nanRoundTrip, out var consumed) || consumed != 4)
        {
            return false;
        }
        var nanRoundTripBytes = BitConverter.GetBytes(nanRoundTrip, Endianness.Little);
        if (!Matches(nanRoundTripBytes, nanBytes))
        {
            return false;
        }

        let infinity = FromUIntToFloat(0x7F800000u);
        var infBytes = BitConverter.GetBytes(infinity, Endianness.Big);
        if (!Matches(infBytes, new byte[] { 0x7F, 0x80, 0x00, 0x00 }))
        {
            return false;
        }

        var seeds = new uint[] { 0x00000000u, 0x3F000000u, 0xBF800000u, 0x41200000u, 0x7FC00001u };
        var idx = 0usize;
        while (idx < seeds.Length)
        {
            let originalBits = seeds[idx];
            let value = FromUIntToFloat(originalBits);
            var buffer = Span<byte>.StackAlloc(4usize);
            if (!BitConverter.TryWriteSingle(buffer, value, Endianness.Little, out var count) || count != 4)
            {
                return false;
            }
            if (!BitConverter.TryReadSingle(buffer.AsReadOnly(), Endianness.Little, out var decoded, out var readCount) || readCount != 4)
            {
                return false;
            }
            var decodedBytes = BitConverter.GetBytes(decoded, Endianness.Little);
            if (!Matches(buffer.AsReadOnly(), decodedBytes))
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }

    private static bool ValidateFloat64Vectors()
    {
        let oneBits = 0x3FF0000000000000ul;
        let one = FromUIntToDouble(oneBits);
        var bytes = BitConverter.GetBytes(one, Endianness.Little);
        if (!Matches(bytes, new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F }))
        {
            return false;
        }

        let negZero = FromUIntToDouble(0x8000000000000000ul);
        var negZeroBytes = BitConverter.GetBytes(negZero, Endianness.Little);
        if (!Matches(negZeroBytes, new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80 }))
        {
            return false;
        }

        let nanBits = 0x7FF8000000000001ul;
        let nan = FromUIntToDouble(nanBits);
        var nanBytes = BitConverter.GetBytes(nan, Endianness.Big);
        if (!Matches(nanBytes, new byte[] { 0x7F, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01 }))
        {
            return false;
        }
        if (!BitConverter.TryReadDouble(Span<byte>.FromArray(ref nanBytes).AsReadOnly(), Endianness.Big, out var nanRoundTrip, out var consumed) || consumed != 8)
        {
            return false;
        }
        var nanRoundTripBytes = BitConverter.GetBytes(nanRoundTrip, Endianness.Big);
        if (!Matches(nanRoundTripBytes, nanBytes))
        {
            return false;
        }

        var seeds = new ulong[] { 0x0000000000000000ul, 0x3FF0000000000000ul, 0xBFF0000000000000ul, 0x4008000000000000ul, 0x7FF8000000000001ul };
        var idx = 0usize;
        while (idx < seeds.Length)
        {
            let bits = seeds[idx];
            let value = FromUIntToDouble(bits);
            var buffer = Span<byte>.StackAlloc(8usize);
            if (!BitConverter.TryWriteDouble(buffer, value, Endianness.Little, out var count) || count != 8)
            {
                return false;
            }
            if (!BitConverter.TryReadDouble(buffer.AsReadOnly(), Endianness.Little, out var decoded, out var readCount) || readCount != 8)
            {
                return false;
            }
            var decodedBytes = BitConverter.GetBytes(decoded, Endianness.Little);
            if (!Matches(buffer.AsReadOnly(), decodedBytes))
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }

    private static bool ValidateFloat128RoundTrip()
    {
        let size = __sizeof<Float128>();
        var buffer = Span<byte>.StackAlloc(size);
        var value = new Float128(1234.5d);
        if (!BitConverter.TryWriteFloat128(buffer, value, Endianness.Little, out var written) || written != NumericUnchecked.ToInt32(size))
        {
            return false;
        }
        if (!BitConverter.TryReadFloat128(buffer.AsReadOnly(), Endianness.Little, out var decoded, out var consumed) || consumed != NumericUnchecked.ToInt32(size))
        {
            return false;
        }
        var originalBytes = BitConverter.GetBytes(value, Endianness.Little);
        var decodedBytes = BitConverter.GetBytes(decoded, Endianness.Little);
        if (!Matches(Span<byte>.FromArray(ref originalBytes).AsReadOnly(), decodedBytes))
        {
            return false;
        }
        return true;
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

    private static float FromUIntToFloat(uint bits)
    {
        var slot = MaybeUninit<float>.Uninit();
        let dest = slot.AsValueMutPtr();
        let sourceSpan = BitConverter.GetBytes(bits, Endianness.Little);
        GlobalAllocator.Copy(dest, Span<byte>.FromArray(ref sourceSpan).Raw.Data, __sizeof<uint>());
        return slot.AssumeInit();
    }

    private static double FromUIntToDouble(ulong bits)
    {
        var slot = MaybeUninit<double>.Uninit();
        let dest = slot.AsValueMutPtr();
        let sourceSpan = BitConverter.GetBytes(bits, Endianness.Little);
        GlobalAllocator.Copy(dest, Span<byte>.FromArray(ref sourceSpan).Raw.Data, __sizeof<ulong>());
        return slot.AssumeInit();
    }

    private static float16 FromUIntToFloat16(ushort bits)
    {
        var slot = MaybeUninit<float16>.Uninit();
        let dest = slot.AsValueMutPtr();
        let sourceSpan = BitConverter.GetBytes(bits, Endianness.Little);
        GlobalAllocator.Copy(dest, Span<byte>.FromArray(ref sourceSpan).Raw.Data, __sizeof<ushort>());
        return slot.AssumeInit();
    }
}
