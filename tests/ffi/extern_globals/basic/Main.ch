namespace Tests.FfiExternGlobals.Basic;

@extern("C") @link("ffi_extern_globals_basic")
public extern static mut int g_counter;

@extern("C") @link("ffi_extern_globals_basic")
public static extern int c_read_counter();

@extern("C") @link("ffi_extern_globals_basic")
public static extern void c_write_counter(int value);

@extern("C") @link("ffi_extern_globals_basic")
public static extern void extern_global_anchor();

public static int Main()
{
    extern_global_anchor();
    unsafe
    {
        if (g_counter != 7)
        {
            return 1;
        }
        g_counter = 13;
        if (c_read_counter() != 13)
        {
            return 2;
        }
        c_write_counter(21);
        if (g_counter != 21)
        {
            return 3;
        }
        g_counter = 42;
    }
    if (c_read_counter() != 42)
    {
        return 4;
    }
    return 0;
}
