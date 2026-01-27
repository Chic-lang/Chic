namespace Tests.FfiTls;

@threadlocal
public static mut int Counter = 0;

@extern("C") @export("chic_tls_get")
public static int GetCounter()
{
    unsafe
    {
        return Counter;
    }
}

@extern("C") @export("chic_tls_inc")
public static int IncrementCounter(int delta)
{
    unsafe
    {
        Counter = Counter + delta;
        return Counter;
    }
}

@extern("C") @export("chic_tls_reset")
public static void ResetCounter(int value)
{
    unsafe
    {
        Counter = value;
    }
}

@extern("C") @link("ffi_tls")
public static extern int run_tls_threads(int delta_a, int delta_b);

@extern("C") @link("pthread")
public static extern void pthread_link_anchor();

public static int Main()
{
    if (GetCounter() != 0)
    {
        return 1;
    }

    if (IncrementCounter(5) != 5 || GetCounter() != 5)
    {
        return 2;
    }

    let combined = run_tls_threads(2, 3);
    if (combined != (2 * 2 + 3 * 3))
    {
        return 3;
    }

    if (GetCounter() != 5)
    {
        return 4;
    }

    let second = run_tls_threads(1, 4);
    if (second != (1 * 2 + 4 * 3))
    {
        return 5;
    }

    if (GetCounter() != 5)
    {
        return 6;
    }

    return 0;
}
