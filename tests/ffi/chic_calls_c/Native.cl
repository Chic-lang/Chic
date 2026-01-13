namespace Tests.FfiAbi;

public static class Native
{
    @extern("C") @link("ffi_aggs")
    public static extern S1 make_s1(byte v);

    @extern("C") @link("ffi_aggs")
    public static extern S2 make_s2(ushort v);

    @extern("C") @link("ffi_aggs")
    public static extern S3 make_s3(byte a, ushort b);

    @extern("C") @link("ffi_aggs")
    public static extern S4 make_s4(uint v);

    @extern("C") @link("ffi_aggs")
    public static extern S8 make_s8(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern S16 make_s16(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern S24 make_s24(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern S32 make_s32(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern S48 make_s48(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern ulong sum_s48(S48 v);

    @extern("C") @link("ffi_aggs")
    public static extern S48 bump_s48(S48 v);

    @extern("C") @link("ffi_aggs")
    public static extern S64 make_s64(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern ulong sum_s64(S64 v);

    @extern("C") @link("ffi_aggs")
    public static extern S72 make_s72(ulong v);

    @extern("C") @link("ffi_aggs")
    public static extern ulong sum_s72(S72 v);

    @extern("C") @link("ffi_aggs")
    public static extern Mix make_mix(uint a, double b, ushort c);

    @extern("C") @link("ffi_aggs")
    public static extern Outer make_outer(ulong v, uint tail);

    @extern("C") @link("ffi_aggs")
    public static extern Hfa4d make_hfa4d(double x);

    @extern("C") @link("ffi_aggs")
    public static extern double sum_hfa4d(Hfa4d v);
}

