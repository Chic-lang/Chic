namespace Tests.FfiAbi;

public static int Main()
{
    let s1 = Native.make_s1(0x2au8);
    if (s1.a != 0x2au8)
    {
        return 1;
    }

    let s2 = Native.make_s2(0x1234u16);
    if (s2.a != 0x1234u16)
    {
        return 2;
    }

    let s3 = Native.make_s3(0x34u8, 0x5678u16);
    if (s3.a != 0x34u8 || s3.b != 0x5678u16)
    {
        return 3;
    }

    let s4 = Native.make_s4(0xdeadbeefu32);
    if (s4.a != 0xdeadbeefu32)
    {
        return 4;
    }

    let s8 = Native.make_s8(42ul);
    if (s8.a != 42ul)
    {
        return 5;
    }

    let s16 = Native.make_s16(21ul);
    if (s16.a != 21ul || s16.b != 22ul)
    {
        return 6;
    }

    let s24 = Native.make_s24(9ul);
    if (s24.c != 11ul)
    {
        return 7;
    }

    let s32 = Native.make_s32(11ul);
    if (s32.d != 14ul)
    {
        return 8;
    }

    let s48 = Native.make_s48(7ul);
    if (s48.a != 7ul || s48.f != 12ul)
    {
        return 9;
    }

    let sum48 = Native.sum_s48(s48);
    if (sum48 != (7ul + 8ul + 9ul + 10ul + 11ul + 12ul))
    {
        return 10;
    }

    let bumped = Native.bump_s48(s48);
    if (bumped.a != 17ul || bumped.f != 22ul)
    {
        return 11;
    }

    let s64 = Native.make_s64(3ul);
    if (s64.a != 3ul || s64.h != 10ul)
    {
        return 12;
    }
    let sum64 = Native.sum_s64(s64);
    if (sum64 != (3ul + 4ul + 5ul + 6ul + 7ul + 8ul + 9ul + 10ul))
    {
        return 13;
    }

    let s72 = Native.make_s72(5ul);
    if (s72.a != 5ul || s72.i != 13ul)
    {
        return 14;
    }
    let sum72 = Native.sum_s72(s72);
    if (sum72 != (5ul + 6ul + 7ul + 8ul + 9ul + 10ul + 11ul + 12ul + 13ul))
    {
        return 15;
    }

    let mix = Native.make_mix(0xdecafbad, 1.5, 0x4321u16);
    if (mix.a != 0xdecafbad || mix.b != 1.5 || mix.c != 0x4321u16)
    {
        return 16;
    }

    let outer = Native.make_outer(101ul, 0x9988u32);
    if (outer.inner.a != 101ul || outer.inner.b != 102ul || outer.tail != 0x9988u32)
    {
        return 17;
    }

    let hf = Native.make_hfa4d(1.5);
    if (hf.a != 1.5 || hf.d != 4.5)
    {
        return 18;
    }
    let hf_sum = Native.sum_hfa4d(hf);
    if (hf_sum != (1.5 + 2.5 + 3.5 + 4.5))
    {
        return 19;
    }

    return 0;
}

