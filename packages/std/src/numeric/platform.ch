namespace Std.Numeric;
internal static class NumericPlatform
{
    public const uint PointerBits = (uint)(sizeof(nint) * 8u);
    public static nint IntPtrMinValue {
        get {
            if (PointerBits == 32u)
            {
                return NumericUnchecked.ToNintFromInt32(- 2147483648);
            }
            return NumericUnchecked.ToNintFromInt64(- 9223372036854775808L);
        }
    }
    public static nint IntPtrMaxValue {
        get {
            if (PointerBits == 32u)
            {
                return NumericUnchecked.ToNintFromInt32(2147483647);
            }
            return NumericUnchecked.ToNintFromInt64(9223372036854775807L);
        }
    }
    public static nuint UIntPtrMaxValue {
        get {
            if (PointerBits == 32u)
            {
                return(nuint) 0xFFFF_FFFFu;
            }
            return(nuint) 0xFFFF_FFFF_FFFF_FFFFul;
        }
    }
}
