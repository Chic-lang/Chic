namespace Tests.FfiExternGlobals.Weak;

@extern("C") @weak_import @link("ffi_extern_global_opt")
public extern static mut int optional_global;

@extern("C") @link("ffi_extern_global_opt")
public static extern void weak_global_anchor();

public static unsafe int Main()
{
    weak_global_anchor();
    let ptr = &optional_global;
    if (ptr == null)
    {
        return 5;
    }
    if (optional_global != 9)
    {
        return 10;
    }
    optional_global = 12;
    return optional_global + 30;
}
