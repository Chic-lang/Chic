namespace Tests.FfiFnPtr;

import Std.Runtime.InteropServices;

@StructLayout(LayoutKind.Sequential)
public struct Big
{
    public long a;
    public long b;
    public long c;
}

public static class Native
{
    @extern("C") @link("ffi_fnptr")
    public static extern fn @extern("C")(long) -> Big c_provide_big_cb();

    @extern("C") @link("ffi_fnptr")
    public static extern fn @extern("C")(Big) -> long c_provide_sum_cb();

    @extern("C") @link("ffi_fnptr")
    public static extern long c_call_chic_make(fn @extern("C")(long) -> Big cb);

    @extern("C") @link("ffi_fnptr")
    public static extern long c_call_chic_sum(fn @extern("C")(Big) -> long cb);
}

@extern("C") @export("chic_make_big")
public static Big ChicMakeBig(long base)
{
    return new Big { a = base, b = base + 1, c = base + 2 };
}

@extern("C") @export("chic_sum_big")
public static long ChicSumBig(Big value)
{
    return value.a + value.b + value.c;
}

public static int Main()
{
    unsafe
    {
        let c_big = Native.c_provide_big_cb();
        let via_c = c_big(10);
        if (via_c.a != 10 || via_c.b != 11 || via_c.c != 12)
        {
            return 1;
        }

        let sum_cb = Native.c_provide_sum_cb();
        let summed = sum_cb(new Big { a = 5, b = 6, c = 7 });
        if (summed != (5 + 6 + 7))
        {
            return 2;
        }

        let roundtrip = Native.c_call_chic_make(ChicMakeBig);
        if (roundtrip != (50 + 51 + 52))
        {
            return 3;
        }

        let sum_roundtrip = Native.c_call_chic_sum(ChicSumBig);
        if (sum_roundtrip != (7 + 8 + 9))
        {
            return 4;
        }
    }

    return 0;
}
