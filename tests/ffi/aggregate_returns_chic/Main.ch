namespace Tests.FfiAggregates;

import Std.Runtime.InteropServices;

@StructLayout(LayoutKind.Sequential)
public struct S1 { public byte a; }

@StructLayout(LayoutKind.Sequential)
public struct S2 { public byte a; public byte b; }

@StructLayout(LayoutKind.Sequential)
public struct S3 { public byte a; public byte b; public byte c; }

@StructLayout(LayoutKind.Sequential)
public struct S4 { public int a; }

@StructLayout(LayoutKind.Sequential)
public struct S8 { public int a; public int b; }

@StructLayout(LayoutKind.Sequential)
public struct S16 { public long a; public long b; }

@StructLayout(LayoutKind.Sequential)
public struct S24 { public long a; public long b; public long c; }

@StructLayout(LayoutKind.Sequential)
public struct S32 { public long a; public long b; public long c; public long d; }

@StructLayout(LayoutKind.Sequential)
public struct S64
{
    public long a;
    public long b;
    public long c;
    public long d;
    public long e;
    public long f;
    public long g;
    public long h;
}

@StructLayout(LayoutKind.Sequential)
public struct S72
{
    public long a;
    public long b;
    public long c;
    public long d;
    public long e;
    public long f;
    public long g;
    public long h;
    public long i;
}

@StructLayout(LayoutKind.Sequential, Pack=1)
public struct Packed
{
    public ushort a;
    public uint b;
    public byte c;
}

@StructLayout(LayoutKind.Sequential)
public struct Hfa4 { public float a; public float b; public float c; public float d; }

@StructLayout(LayoutKind.Sequential)
public struct Mixed16 { public double a; public float b; }

public static class Native
{
    @extern("C") @link("ffi_aggregates") public static extern S1 make_s1(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s1(S1 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s1(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s1(S1 value);

    @extern("C") @link("ffi_aggregates") public static extern S2 make_s2(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s2(S2 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s2(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s2(S2 value);

    @extern("C") @link("ffi_aggregates") public static extern S3 make_s3(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s3(S3 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s3(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s3(S3 value);

    @extern("C") @link("ffi_aggregates") public static extern S4 make_s4(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s4(S4 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s4(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s4(S4 value);

    @extern("C") @link("ffi_aggregates") public static extern S8 make_s8(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s8(S8 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s8(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s8(S8 value);

    @extern("C") @link("ffi_aggregates") public static extern S16 make_s16(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s16(S16 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s16(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s16(S16 value);

    @extern("C") @link("ffi_aggregates") public static extern S24 make_s24(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s24(S24 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s24(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s24(S24 value);

    @extern("C") @link("ffi_aggregates") public static extern S32 make_s32(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s32(S32 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s32(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s32(S32 value);

    @extern("C") @link("ffi_aggregates") public static extern S64 make_s64(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s64(S64 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s64(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s64(S64 value);

    @extern("C") @link("ffi_aggregates") public static extern S72 make_s72(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_s72(S72 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_s72(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_s72(S72 value);

    @extern("C") @link("ffi_aggregates") public static extern Packed make_packed(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_packed(Packed value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_packed(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_packed(Packed value);

    @extern("C") @link("ffi_aggregates") public static extern Hfa4 make_hfa4(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_hfa4(Hfa4 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_hfa4(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_hfa4(Hfa4 value);

    @extern("C") @link("ffi_aggregates") public static extern Mixed16 make_mixed16(long base);
    @extern("C") @link("ffi_aggregates") public static extern long sum_mixed16(Mixed16 value);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_make_mixed16(long base);
    @extern("C") @link("ffi_aggregates") public static extern long call_chic_take_mixed16(Mixed16 value);
}

@extern("C") @export("chic_make_s1")
public static S1 ChicMakeS1(long base) { return new S1 { a = (byte)(base + 7) }; }
@extern("C") @export("chic_take_s1")
public static long ChicTakeS1(S1 value) { return (long)value.a + 100; }

@extern("C") @export("chic_make_s2")
public static S2 ChicMakeS2(long base)
{
    return new S2 { a = (byte)(base + 5), b = (byte)(base + 6) };
}
@extern("C") @export("chic_take_s2")
public static long ChicTakeS2(S2 value) { return (long)value.a + value.b + 10; }

@extern("C") @export("chic_make_s3")
public static S3 ChicMakeS3(long base)
{
    return new S3 { a = (byte)(base), b = (byte)(base + 1), c = (byte)(base + 2) };
}
@extern("C") @export("chic_take_s3")
public static long ChicTakeS3(S3 value) { return (long)value.a + value.b + value.c + 3; }

@extern("C") @export("chic_make_s4")
public static S4 ChicMakeS4(long base) { return new S4 { a = (int)(base * 3) }; }
@extern("C") @export("chic_take_s4")
public static long ChicTakeS4(S4 value) { return value.a + 33; }

@extern("C") @export("chic_make_s8")
public static S8 ChicMakeS8(long base)
{
    return new S8 { a = (int)(base + 1), b = (int)(base + 2) };
}
@extern("C") @export("chic_take_s8")
public static long ChicTakeS8(S8 value) { return (long)value.a + value.b + 8; }

@extern("C") @export("chic_make_s16")
public static S16 ChicMakeS16(long base) { return new S16 { a = base + 11, b = base + 12 }; }
@extern("C") @export("chic_take_s16")
public static long ChicTakeS16(S16 value) { return value.a + value.b + 16; }

@extern("C") @export("chic_make_s24")
public static S24 ChicMakeS24(long base)
{
    return new S24 { a = base + 3, b = base + 4, c = base + 5 };
}
@extern("C") @export("chic_take_s24")
public static long ChicTakeS24(S24 value) { return value.a + value.b + value.c + 24; }

@extern("C") @export("chic_make_s32")
public static S32 ChicMakeS32(long base)
{
    return new S32 { a = base + 1, b = base + 2, c = base + 3, d = base + 4 };
}
@extern("C") @export("chic_take_s32")
public static long ChicTakeS32(S32 value) { return value.a + value.b + value.c + value.d + 32; }

@extern("C") @export("chic_make_s64")
public static S64 ChicMakeS64(long base)
{
    return new S64
    {
        a = base + 1,
        b = base + 2,
        c = base + 3,
        d = base + 4,
        e = base + 5,
        f = base + 6,
        g = base + 7,
        h = base + 8
    };
}
@extern("C") @export("chic_take_s64")
public static long ChicTakeS64(S64 value)
{
    return value.a + value.b + value.c + value.d + value.e + value.f + value.g + value.h + 64;
}

@extern("C") @export("chic_make_s72")
public static S72 ChicMakeS72(long base)
{
    return new S72
    {
        a = base + 1,
        b = base + 2,
        c = base + 3,
        d = base + 4,
        e = base + 5,
        f = base + 6,
        g = base + 7,
        h = base + 8,
        i = base + 9
    };
}
@extern("C") @export("chic_take_s72")
public static long ChicTakeS72(S72 value)
{
    return value.a + value.b + value.c + value.d + value.e + value.f + value.g + value.h + value.i + 72;
}

@extern("C") @export("chic_make_packed")
public static Packed ChicMakePacked(long base)
{
    return new Packed { a = (ushort)(base + 9), b = (uint)(base + 10), c = (byte)(base + 11) };
}
@extern("C") @export("chic_take_packed")
public static long ChicTakePacked(Packed value) { return (long)value.a + value.b + value.c + 1; }

@extern("C") @export("chic_make_hfa4")
public static Hfa4 ChicMakeHfa4(long base)
{
    return new Hfa4 { a = (float)(base + 1), b = (float)(base + 2), c = (float)(base + 3), d = (float)(base + 4) };
}
@extern("C") @export("chic_take_hfa4")
public static long ChicTakeHfa4(Hfa4 value)
{
    var sum = (double)value.a + value.b + value.c + value.d;
    return (long)(sum + 4.0);
}

@extern("C") @export("chic_make_mixed16")
public static Mixed16 ChicMakeMixed16(long base)
{
    var asDouble = (double)base;
    return new Mixed16 { a = asDouble + 0.5d, b = (float)(asDouble + 1.5d) };
}
@extern("C") @export("chic_take_mixed16")
public static long ChicTakeMixed16(Mixed16 value)
{
    var sum = value.a + (double)value.b;
    return (long)(sum + 10.0d);
}

static long SumS64(S64 value)
{
    return value.a + value.b + value.c + value.d + value.e + value.f + value.g + value.h;
}

static long SumS72(S72 value)
{
    return value.a + value.b + value.c + value.d + value.e + value.f + value.g + value.h + value.i;
}

static long SumPacked(Packed value) { return (long)value.a + value.b + value.c; }

static long SumHfa4(Hfa4 value)
{
    var sum = (double)value.a + value.b + value.c + value.d;
    return (long)sum;
}

static long SumMixed16(Mixed16 value)
{
    var sum = value.a + (double)value.b;
    return (long)sum;
}

public static int Main()
{
    unsafe
    {
        let s1 = Native.make_s1(2);
        if (s1.a != 3u8 || Native.sum_s1(s1) != 3)
        {
            return 1;
        }
        let s1_local = new S1 { a = 9u8 };
        if (Native.call_chic_make_s1(5) != (long)ChicMakeS1(5).a
            || Native.call_chic_take_s1(s1_local) != ChicTakeS1(s1_local))
        {
            return 2;
        }

        let s2 = Native.make_s2(4);
        if (s2.a != 5u8) { return 31; }
        if (s2.b != 6u8) { return 200 + (int)s2.b; }
        let s2_sum = Native.sum_s2(s2);
        if (s2_sum != 11) { return 33; }
        let s2_local = new S2 { a = 10u8, b = 20u8 };
        if (Native.call_chic_make_s2(7) != (long)ChicMakeS2(7).a + ChicMakeS2(7).b
            || Native.call_chic_take_s2(s2_local) != ChicTakeS2(s2_local))
        {
            return 4;
        }

        let s3 = Native.make_s3(1);
        if (Native.sum_s3(s3) != (long)s3.a + s3.b + s3.c)
        {
            return 5;
        }
        let s3_local = new S3 { a = 3u8, b = 4u8, c = 5u8 };
        if (Native.call_chic_make_s3(8) != (long)ChicMakeS3(8).a + ChicMakeS3(8).b + ChicMakeS3(8).c
            || Native.call_chic_take_s3(s3_local) != ChicTakeS3(s3_local))
        {
            return 6;
        }

        let s4 = Native.make_s4(10);
        if (s4.a != 20 || Native.sum_s4(s4) != 20)
        {
            return 7;
        }
        let s4_local = new S4 { a = 12 };
        if (Native.call_chic_make_s4(11) != ChicMakeS4(11).a
            || Native.call_chic_take_s4(s4_local) != ChicTakeS4(s4_local))
        {
            return 8;
        }

        let s8 = Native.make_s8(9);
        if (Native.sum_s8(s8) != (long)s8.a + s8.b)
        {
            return 9;
        }
        let s8_local = new S8 { a = 4, b = 6 };
        if (Native.call_chic_make_s8(3) != (long)ChicMakeS8(3).a + ChicMakeS8(3).b
            || Native.call_chic_take_s8(s8_local) != ChicTakeS8(s8_local))
        {
            return 10;
        }

        let s16 = Native.make_s16(2);
        if (Native.sum_s16(s16) != s16.a + s16.b)
        {
            return 11;
        }
        let s16_local = new S16 { a = 30, b = 40 };
        if (Native.call_chic_make_s16(4) != ChicMakeS16(4).a + ChicMakeS16(4).b
            || Native.call_chic_take_s16(s16_local) != ChicTakeS16(s16_local))
        {
            return 12;
        }

        let s24 = Native.make_s24(10);
        if (Native.sum_s24(s24) != s24.a + s24.b + s24.c)
        {
            return 13;
        }
        let s24_local = new S24 { a = 3, b = 4, c = 5 };
        if (Native.call_chic_make_s24(5) != ChicMakeS24(5).a + ChicMakeS24(5).b + ChicMakeS24(5).c
            || Native.call_chic_take_s24(s24_local) != ChicTakeS24(s24_local))
        {
            return 14;
        }

        let s32 = Native.make_s32(6);
        if (Native.sum_s32(s32) != s32.a + s32.b + s32.c + s32.d)
        {
            return 15;
        }
        let s32_local = new S32 { a = 1, b = 2, c = 3, d = 4 };
        if (Native.call_chic_make_s32(9) != ChicMakeS32(9).a + ChicMakeS32(9).b + ChicMakeS32(9).c + ChicMakeS32(9).d
            || Native.call_chic_take_s32(s32_local) != ChicTakeS32(s32_local))
        {
            return 16;
        }

        let s64 = Native.make_s64(5);
        if (Native.sum_s64(s64) != SumS64(s64))
        {
            return 17;
        }
        let s64_local = new S64 { a = 10, b = 11, c = 12, d = 13, e = 14, f = 15, g = 16, h = 17 };
        if (Native.call_chic_make_s64(7) != SumS64(ChicMakeS64(7))
            || Native.call_chic_take_s64(s64_local) != ChicTakeS64(s64_local))
        {
            return 18;
        }

        let s72 = Native.make_s72(2);
        if (Native.sum_s72(s72) != SumS72(s72))
        {
            return 19;
        }
        let s72_local = new S72 { a = 1, b = 2, c = 3, d = 4, e = 5, f = 6, g = 7, h = 8, i = 9 };
        if (Native.call_chic_make_s72(4) != SumS72(ChicMakeS72(4))
            || Native.call_chic_take_s72(s72_local) != ChicTakeS72(s72_local))
        {
            return 20;
        }

        let packed = Native.make_packed(3);
        if (Native.sum_packed(packed) != SumPacked(packed))
        {
            return 21;
        }
        let packed_local = new Packed { a = 1, b = 2, c = 3 };
        if (Native.call_chic_make_packed(8) != SumPacked(ChicMakePacked(8))
            || Native.call_chic_take_packed(packed_local) != ChicTakePacked(packed_local))
        {
            return 22;
        }

        let hfa = Native.make_hfa4(1);
        if (Native.sum_hfa4(hfa) != SumHfa4(hfa))
        {
            return 23;
        }
        let hfa_local = new Hfa4 { a = 1.0f, b = 2.0f, c = 3.0f, d = 4.0f };
        if (Native.call_chic_make_hfa4(5) != SumHfa4(ChicMakeHfa4(5))
            || Native.call_chic_take_hfa4(hfa_local) != ChicTakeHfa4(hfa_local))
        {
            return 24;
        }

        let mixed = Native.make_mixed16(2);
        if (Native.sum_mixed16(mixed) != SumMixed16(mixed))
        {
            return 25;
        }
        let mixed_local = new Mixed16 { a = 1.0d, b = 2.0f };
        if (Native.call_chic_make_mixed16(6) != SumMixed16(ChicMakeMixed16(6))
            || Native.call_chic_take_mixed16(mixed_local) != ChicTakeMixed16(mixed_local))
        {
            return 26;
        }
    }

    return 0;
}
