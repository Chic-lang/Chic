namespace Tests.FfiExternGlobals.Tls;

@extern("C") @threadlocal @link("ffi_extern_globals_tls")
public extern static mut int tls_value;

@extern("C") @export("chic_tls_read")
public static int ReadTls()
{
    unsafe
    {
        return tls_value;
    }
}

@extern("C") @export("chic_tls_add")
public static int AddTls(int delta)
{
    unsafe
    {
        tls_value = tls_value + delta;
        return tls_value;
    }
}

@extern("C") @link("ffi_extern_globals_tls")
public static extern int run_tls_threads(int delta_a, int delta_b);

@extern("C") @link("pthread")
public static extern void pthread_link_anchor();

public static int Main()
{
    pthread_link_anchor();
    unsafe
    {
        if (tls_value != 1)
        {
            return 1;
        }
        tls_value = 5;
        if (tls_value != 5)
        {
            return 2;
        }
    }

    let combined = run_tls_threads(2, 3);
    if (combined != (1 + 2) + (1 + 3))
    {
        return 3;
    }

    unsafe
    {
        if (tls_value != 5)
        {
            return 4;
        }
    }
    return 0;
}
