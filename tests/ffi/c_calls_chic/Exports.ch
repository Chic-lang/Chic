namespace Tests.FfiAbi;

@extern("C") @export("chic_make_s48")
public static S48 MakeS48(ulong v)
{
    return new S48 { a = v, b = v + 1ul, c = v + 2ul, d = v + 3ul, e = v + 4ul, f = v + 5ul };
}

@extern("C") @export("chic_sum_s48")
public static ulong SumS48(S48 v)
{
    return v.a + v.b + v.c + v.d + v.e + v.f;
}

@extern("C") @export("chic_bump_s48")
public static S48 BumpS48(S48 v)
{
    return new S48 {
        a = v.a + 10ul,
        b = v.b + 10ul,
        c = v.c + 10ul,
        d = v.d + 10ul,
        e = v.e + 10ul,
        f = v.f + 10ul
    };
}

@extern("C") @export("chic_make_s64")
public static S64 MakeS64(ulong v)
{
    return new S64 {
        a = v,
        b = v + 1ul,
        c = v + 2ul,
        d = v + 3ul,
        e = v + 4ul,
        f = v + 5ul,
        g = v + 6ul,
        h = v + 7ul
    };
}

@extern("C") @export("chic_sum_s64")
public static ulong SumS64(S64 v)
{
    return v.a + v.b + v.c + v.d + v.e + v.f + v.g + v.h;
}

@extern("C") @export("chic_make_hfa4d")
public static Hfa4d MakeHfa4d(double x)
{
    return new Hfa4d { a = x, b = x + 1.0, c = x + 2.0, d = x + 3.0 };
}

@extern("C") @export("chic_sum_hfa4d")
public static double SumHfa4d(Hfa4d v)
{
    return v.a + v.b + v.c + v.d;
}

@extern("C") @export("chic_make_mix")
public static Mix MakeMix(uint a, double b, ushort c)
{
    return new Mix { a = a, b = b, c = c };
}

