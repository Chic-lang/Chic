namespace Tests.FfiVarargs;

public static class Native
{
    @extern("C") @link("ffi_varargs")
    public static extern int check_promotions(int count, ...);
}

public static int Main()
{
    var ok = 0;
    unsafe
    {
        ok = Native.check_promotions(3, 2.5f, 7, 9);
    }
    if (ok != 1)
    {
        return 1;
    }
    return 0;
}
