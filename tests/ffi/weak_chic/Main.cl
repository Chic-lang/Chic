namespace Tests.FfiWeak;

@extern("C") @weak_import @link("ffi_weak_import")
public static extern int optional_weak_symbol();

@extern("C") @link("ffi_weak_import")
public static extern void weak_import_anchor();

@extern("C") @weak @export("chic_weak_value")
public static int WeakValue()
{
    return 5;
}

@extern("C") @link("ffi_weak_override")
public static extern void weak_override_anchor();

public static unsafe int Main()
{
    weak_import_anchor();
    weak_override_anchor();

    let opt_value = 0;
    if (optional_weak_symbol != null)
    {
        opt_value = optional_weak_symbol();
    }

    let weak_value = WeakValue();
    if (weak_value != 5 && weak_value != 77)
    {
        return 20;
    }

    return opt_value + weak_value;
}
