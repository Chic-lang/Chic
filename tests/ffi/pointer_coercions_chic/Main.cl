namespace Tests.FfiPointers;

import Std.Runtime.InteropServices;

@StructLayout(LayoutKind.Sequential)
public struct Value
{
    public long marker;
    public long other;
}

public static class Native
{
    @extern("C") @link("ffi_pointers")
    public static extern void touch_void(*mut void ptr);

    @extern("C") @link("ffi_pointers")
    public static extern long read_const(*const Value ptr);

    @extern("C") @link("ffi_pointers")
    public static extern void* get_void_pointer();

    @extern("C") @link("ffi_pointers")
    public static extern int is_null(void* ptr);
}

public static int Main()
{
    unsafe
    {
        var local = new Value { marker = 1, other = 2 };
        let mut_ptr = &local;

        // typed* -> void* implicit in extern context
        Native.touch_void(mut_ptr);
        if (local.marker != 42)
        {
            return 1;
        }

        // *mut -> *const implicit
        let sum = Native.read_const(mut_ptr);
        if (sum != 42 + 2)
        {
            return 2;
        }

        // void* -> typed* requires explicit cast
        let raw = Native.get_void_pointer();
        var typed = (*mut Value)raw;
        (*typed).marker = 123;
        if (Native.read_const(typed) != 123)
        {
            return 3;
        }

        // null inhabits any pointer type
        if (Native.is_null(null) == 0)
        {
            return 4;
        }
    }

    return 0;
}
