namespace Std.Numeric;
internal static class NumericBitOperations
{
    public static int LeadingZeroCountByte(byte value) {
        return LeadingZeroCountUInt32(value) - 24;
    }
    public static int LeadingZeroCountSByte(sbyte value) {
        return LeadingZeroCountByte(NumericUnchecked.ToByte(value));
    }
    public static int LeadingZeroCountUInt16(ushort value) {
        return LeadingZeroCountUInt32((uint) value) - 16;
    }
    public static int LeadingZeroCountInt16(short value) {
        return LeadingZeroCountUInt16((ushort) value);
    }
    public static int LeadingZeroCountInt32(int value) {
        var bits = (uint) value;
        if (bits == 0u)
        {
            return 32;
        }
        var count = 0;
        var mask = 0x8000_0000u;
        while ( (bits & mask) == 0u)
        {
            count += 1;
            mask >>= 1;
        }
        return count;
    }
    public static int LeadingZeroCountInt64(long value) {
        var bits = (ulong) value;
        if (bits == 0ul)
        {
            return 64;
        }
        var count = 0;
        var mask = 0x8000_0000_0000_0000ul;
        while ( (bits & mask) == 0ul)
        {
            count += 1;
            mask >>= 1;
        }
        return count;
    }
    public static int LeadingZeroCountUInt32(uint value) {
        return LeadingZeroCountInt32((int) value);
    }
    public static int LeadingZeroCountUInt64(ulong value) {
        return LeadingZeroCountInt64((long) value);
    }
    public static int TrailingZeroCountByte(byte value) {
        var count = TrailingZeroCountUInt32((uint) value);
        if (count >8)
        {
            return 8;
        }
        return count;
    }
    public static int TrailingZeroCountSByte(sbyte value) {
        return TrailingZeroCountByte((byte) value);
    }
    public static int TrailingZeroCountUInt16(ushort value) {
        var count = TrailingZeroCountUInt32((uint) value);
        if (count >16)
        {
            return 16;
        }
        return count;
    }
    public static int TrailingZeroCountInt16(short value) {
        return TrailingZeroCountUInt16((ushort) value);
    }
    public static int TrailingZeroCountInt32(int value) {
        var count = 0;
        var bits = (uint) value;
        if (bits == 0u)
        {
            return 32;
        }
        while ( (bits & 1u) == 0u)
        {
            count += 1;
            bits >>= 1;
        }
        return count;
    }
    public static int TrailingZeroCountInt64(long value) {
        var count = 0;
        var bits = (ulong) value;
        if (bits == 0ul)
        {
            return 64;
        }
        while ( (bits & 1ul) == 0ul)
        {
            count += 1;
            bits >>= 1;
        }
        return count;
    }
    public static int TrailingZeroCountUInt32(uint value) {
        return TrailingZeroCountInt32((int) value);
    }
    public static int TrailingZeroCountUInt64(ulong value) {
        return TrailingZeroCountInt64((long) value);
    }
    public static int PopCountByte(byte value) {
        return PopCountUInt32(value);
    }
    public static int PopCountSByte(sbyte value) {
        return PopCountByte(NumericUnchecked.ToByte(value));
    }
    public static int PopCountUInt16(ushort value) {
        return PopCountUInt32(value);
    }
    public static int PopCountInt16(short value) {
        return PopCountUInt16(NumericUnchecked.ToUInt16(value));
    }
    public static int PopCountInt32(int value) {
        var bits = NumericUnchecked.ToUInt32(value);
        var count = 0;
        while (bits != 0u)
        {
            bits &= bits - 1u;
            count += 1;
        }
        return count;
    }
    public static int PopCountInt64(long value) {
        var bits = NumericUnchecked.ToUInt64(value);
        var count = 0;
        while (bits != 0ul)
        {
            bits &= bits - 1ul;
            count += 1;
        }
        return count;
    }
    public static int PopCountUInt32(uint value) {
        return PopCountInt32(NumericUnchecked.ToInt32(value));
    }
    public static int PopCountUInt64(ulong value) {
        return PopCountInt64(NumericUnchecked.ToInt64(value));
    }
    public static int RotateLeftInt32(int value, int offset) {
        var shift = NormalizeShift(offset, 32);
        return RotateRightInt32(value, 32 - shift);
    }
    public static int RotateRightInt32(int value, int offset) {
        var shift = NormalizeShift(offset, 32);
        var bits = NumericUnchecked.ToUInt32(value);
        var rotated = (bits >> shift) | (bits << (32 - shift));
        return NumericUnchecked.ToInt32(rotated);
    }
    public static long RotateLeftInt64(long value, int offset) {
        var shift = NormalizeShift(offset, 64);
        return RotateRightInt64(value, 64 - shift);
    }
    public static long RotateRightInt64(long value, int offset) {
        var shift = NormalizeShift(offset, 64);
        var bits = NumericUnchecked.ToUInt64(value);
        var rotated = (bits >> shift) | (bits << (64 - shift));
        return NumericUnchecked.ToInt64(rotated);
    }
    public static uint RotateLeftUInt32(uint value, int offset) {
        var shift = NormalizeShift(offset, 32);
        return RotateRightUInt32(value, 32 - shift);
    }
    public static uint RotateRightUInt32(uint value, int offset) {
        var shift = NormalizeShift(offset, 32);
        return(value >> shift) | (value << (32 - shift));
    }
    public static ulong RotateLeftUInt64(ulong value, int offset) {
        var shift = NormalizeShift(offset, 64);
        return RotateRightUInt64(value, 64 - shift);
    }
    public static ulong RotateRightUInt64(ulong value, int offset) {
        var shift = NormalizeShift(offset, 64);
        return(value >> shift) | (value << (64 - shift));
    }
    public static sbyte RotateLeftSByte(sbyte value, int offset) {
        var shift = NormalizeShift(offset, 8);
        return RotateRightSByte(value, 8 - shift);
    }
    public static sbyte RotateRightSByte(sbyte value, int offset) {
        var shift = NormalizeShift(offset, 8);
        var bits = NumericUnchecked.ToUInt32(NumericUnchecked.ToByte(value));
        var rotated = (bits >> shift) | (bits << (8 - shift));
        return NumericUnchecked.ToSByte(rotated);
    }
    public static byte RotateLeftByte(byte value, int offset) {
        var shift = NormalizeShift(offset, 8);
        return RotateRightByte(value, 8 - shift);
    }
    public static byte RotateRightByte(byte value, int offset) {
        var shift = NormalizeShift(offset, 8);
        var bits = value;
        var rotated = (bits >> shift) | (bits << (8 - shift));
        return NumericUnchecked.ToByte(rotated);
    }
    public static short RotateLeftInt16(short value, int offset) {
        var shift = NormalizeShift(offset, 16);
        return RotateRightInt16(value, 16 - shift);
    }
    public static short RotateRightInt16(short value, int offset) {
        var shift = NormalizeShift(offset, 16);
        var bits = NumericUnchecked.ToUInt32(NumericUnchecked.ToUInt16(value));
        var rotated = (bits >> shift) | (bits << (16 - shift));
        return NumericUnchecked.ToInt16(rotated);
    }
    public static ushort RotateLeftUInt16(ushort value, int offset) {
        var shift = NormalizeShift(offset, 16);
        return RotateRightUInt16(value, 16 - shift);
    }
    public static ushort RotateRightUInt16(ushort value, int offset) {
        var shift = NormalizeShift(offset, 16);
        var bits = value;
        var rotated = (bits >> shift) | (bits << (16 - shift));
        return NumericUnchecked.ToUInt16(rotated);
    }
    public static int ReverseEndiannessInt32(int value) {
        var bits = NumericUnchecked.ToUInt32(value);
        bits = ((bits & 0xFF00FF00u) >> 8) | ((bits & 0x00FF00FFu) << 8);
        bits = (bits >> 16) | (bits << 16);
        return NumericUnchecked.ToInt32(bits);
    }
    public static long ReverseEndiannessInt64(long value) {
        var bits = NumericUnchecked.ToUInt64(value);
        bits = ((bits & 0xFF00FF00FF00FF00ul) >> 8) | ((bits & 0x00FF00FF00FF00FFul) << 8);
        bits = ((bits & 0xFFFF0000FFFF0000ul) >> 16) | ((bits & 0x0000FFFF0000FFFFul) << 16);
        bits = (bits >> 32) | (bits << 32);
        return NumericUnchecked.ToInt64(bits);
    }
    public static uint ReverseEndiannessUInt32(uint value) {
        return NumericUnchecked.ToUInt32(ReverseEndiannessInt32(NumericUnchecked.ToInt32(value)));
    }
    public static ulong ReverseEndiannessUInt64(ulong value) {
        return NumericUnchecked.ToUInt64(ReverseEndiannessInt64(NumericUnchecked.ToInt64(value)));
    }
    public static sbyte ReverseEndiannessSByte(sbyte value) {
        return value;
    }
    public static byte ReverseEndiannessByte(byte value) {
        return value;
    }
    public static short ReverseEndiannessInt16(short value) {
        var bits = NumericUnchecked.ToUInt16(value);
        var swapped = ((bits & 0x00FFu) << 8) | ((bits & 0xFF00u) >> 8);
        return NumericUnchecked.ToInt16(swapped);
    }
    public static ushort ReverseEndiannessUInt16(ushort value) {
        var rotated = ((value & 0x00FFu) << 8) | ((value & 0xFF00u) >> 8);
        return NumericUnchecked.ToUInt16(rotated);
    }
    public static bool IsPowerOfTwoSByte(sbyte value) {
        if (value <= 0)
        {
            return false;
        }
        return(value & (value - 1)) == 0;
    }
    public static bool IsPowerOfTwoByte(byte value) {
        if (value == 0u8)
        {
            return false;
        }
        return(value & (value - 1u8)) == 0u8;
    }
    public static bool IsPowerOfTwoInt16(short value) {
        if (value <= 0)
        {
            return false;
        }
        return(value & (value - 1)) == 0;
    }
    public static bool IsPowerOfTwoUInt16(ushort value) {
        if (value == 0u16)
        {
            return false;
        }
        return(value & (value - 1u16)) == 0u16;
    }
    public static bool IsPowerOfTwoInt32(int value) {
        return value >0 && (value & (value - 1)) == 0;
    }
    public static bool IsPowerOfTwoInt64(long value) {
        return value >0L && (value & (value - 1L)) == 0L;
    }
    public static bool IsPowerOfTwoUInt32(uint value) {
        return value != 0u && (value & (value - 1u)) == 0u;
    }
    public static bool IsPowerOfTwoUInt64(ulong value) {
        return value != 0ul && (value & (value - 1ul)) == 0ul;
    }
    public static int LeadingZeroCountInt128(int128 value) {
        var bits = NumericUnchecked.ToUInt128(value);
        if (bits == 0u128)
        {
            return 128;
        }
        var count = 0;
        var mask = 1u128 << 127;
        while ( (bits & mask) == 0u128)
        {
            count += 1;
            mask >>= 1;
        }
        return count;
    }
    public static int LeadingZeroCountUInt128(u128 value) {
        if (value == 0u128)
        {
            return 128;
        }
        var count = 0;
        var mask = 1u128 << 127;
        while ( (value & mask) == 0u128)
        {
            count += 1;
            mask >>= 1;
        }
        return count;
    }
    public static int TrailingZeroCountInt128(int128 value) {
        if (value == 0)
        {
            return 128;
        }
        var count = 0;
        var bits = NumericUnchecked.ToUInt128(value);
        while ( (bits & 1u128) == 0u128)
        {
            count += 1;
            bits >>= 1;
        }
        return count;
    }
    public static int TrailingZeroCountUInt128(u128 value) {
        if (value == 0u128)
        {
            return 128;
        }
        var count = 0;
        var bits = value;
        while ( (bits & 1u128) == 0u128)
        {
            count += 1;
            bits >>= 1;
        }
        return count;
    }
    public static int PopCountInt128(int128 value) {
        var bits = NumericUnchecked.ToUInt128(value);
        var count = 0;
        while (bits != 0u128)
        {
            bits &= bits - 1u128;
            count += 1;
        }
        return count;
    }
    public static int PopCountUInt128(u128 value) {
        var bits = value;
        var count = 0;
        while (bits != 0u128)
        {
            bits &= bits - 1u128;
            count += 1;
        }
        return count;
    }
    public static int128 RotateLeftInt128(int128 value, int offset) {
        let shift = NormalizeShift(offset, 128);
        if (shift == 0)
        {
            return value;
        }
        return RotateRightInt128(value, 128 - shift);
    }
    public static int128 RotateRightInt128(int128 value, int offset) {
        let shift = NormalizeShift(offset, 128);
        if (shift == 0)
        {
            return value;
        }
        var bits = NumericUnchecked.ToUInt128(value);
        var rotated = (bits >> shift) | (bits << (128 - shift));
        return NumericUnchecked.ToInt128(rotated);
    }
    public static u128 RotateLeftUInt128(u128 value, int offset) {
        let shift = NormalizeShift(offset, 128);
        if (shift == 0)
        {
            return value;
        }
        return RotateRightUInt128(value, 128 - shift);
    }
    public static u128 RotateRightUInt128(u128 value, int offset) {
        let shift = NormalizeShift(offset, 128);
        if (shift == 0)
        {
            return value;
        }
        return(value >> shift) | (value << (128 - shift));
    }
    public static int128 ReverseEndiannessInt128(int128 value) {
        var bits = NumericUnchecked.ToUInt128(value);
        var reversed = ReverseEndiannessUInt128(bits);
        return NumericUnchecked.ToInt128(reversed);
    }
    public static u128 ReverseEndiannessUInt128(u128 value) {
        var lo = NumericUnchecked.ToUInt64FromUInt128(value);
        var hi = NumericUnchecked.HighUInt64FromUInt128(value);
        var loReversed = ReverseEndiannessUInt64(hi);
        var hiReversed = ReverseEndiannessUInt64(lo);
        return(NumericUnchecked.ToUInt128(hiReversed) << 64) | NumericUnchecked.ToUInt128(loReversed);
    }
    public static bool IsPowerOfTwoInt128(int128 value) {
        return value >0 && (value & (value - 1)) == 0;
    }
    public static bool IsPowerOfTwoUInt128(u128 value) {
        return value != 0u128 && (value & (value - 1u128)) == 0u128;
    }
    private static int NormalizeShift(int shift, int bitWidth) {
        if (bitWidth <= 0)
        {
            return 0;
        }
        var remainder = shift % bitWidth;
        if (remainder <0)
        {
            remainder += bitWidth;
        }
        return remainder;
    }
}
