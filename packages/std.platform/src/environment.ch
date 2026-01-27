namespace Std.Platform;
import Std.Collections;
import Std.Numeric;
import Std.Strings;
import Std.Memory;
import Std.Runtime.Startup;
import Std.Span;
internal static class CString
{
    public static Span <byte >FromStringWithNull(string text) {
        let utf8 = Std.Span.ReadOnlySpan.FromString(text);
        var buffer = StackAlloc.Span <byte >(utf8.Length + 1);
        buffer.Slice(0, utf8.Length).CopyFrom(utf8);
        buffer[utf8.Length] = 0;
        return buffer;
    }
}
public static class EnvironmentInfo
{
    @extern("C") private static extern int getpid();
    @extern("C") private static extern * mut @expose_address byte getenv(* const @expose_address byte name);
    @extern("C") private static extern * mut @expose_address byte getcwd(* mut @expose_address byte buffer, usize size);
    public static string OsDescription() {
        let fallback = Std.Runtime.StringRuntime.FromStr("unix");
        let raw = EnvironmentVariables.Get("OSTYPE");
        if (raw == null)
        {
            return fallback;
        }
        if (raw == "")
        {
            return fallback;
        }
        return raw.ToString();
    }
    public static string Architecture() {
        if (Std.Numeric.NumericPlatform.PointerBits == 64u)
        {
            return "x86_64";
        }
        if (Std.Numeric.NumericPlatform.PointerBits == 32u)
        {
            return "x86";
        }
        return "unknown";
    }
    public static int ProcessId() {
        return getpid();
    }
    public static string WorkingDirectory() {
        var buffer = StackAlloc.Span <byte >(512);
        unsafe {
            let ptr = buffer.Raw.Data.Pointer;
            let result = getcwd(ptr, buffer.Length);
            if (Pointer.IsNull (result))
            {
                return "";
            }
            var len = 0usize;
            while (len <buffer.Length && buffer[len] != 0)
            {
                len += 1;
            }
            let slice = buffer.Slice(0, len).AsReadOnly();
            return Utf8String.FromSpan(slice);
        }
    }
    public static string NewLine() => "\n";
    public static ulong TickCountMilliseconds() {
        return Std.Platform.Time.MonotonicNanoseconds() / 1_000_000UL;
    }
    public static ulong UptimeMilliseconds() {
        return TickCountMilliseconds();
    }
}
public static class EnvironmentVariables
{
    internal static isize DropStringValue() {
        return 0isize;
    }
    @extern("C") private static extern * mut @expose_address byte getenv(* const @expose_address byte name);
    @extern("C") private static extern int setenv(* const @expose_address byte name, * const @expose_address byte value,
    int overwrite);
    @extern("C") private static extern int unsetenv(* const @expose_address byte name);
    public static string ?Get(string name) {
        let cName = CString.FromStringWithNull(name);
        unsafe {
            let value = getenv(cName.Raw.Data.Pointer);
            if (Pointer.IsNull (value))
            {
                return null;
            }
            let bits = Pointer.HandleFrom(value);
            return Std.Runtime.Startup.RuntimeIntrinsics.chic_rt_startup_cstr_to_string((isize) bits);
        }
    }
    public static bool Set(string name, string value) {
        let cName = CString.FromStringWithNull(name);
        let cValue = CString.FromStringWithNull(value);
        unsafe {
            let status = setenv(cName.Raw.Data.Pointer, cValue.Raw.Data.Pointer, 1);
            return status == 0;
        }
    }
    public static bool Remove(string name) {
        let cName = CString.FromStringWithNull(name);
        unsafe {
            let status = unsetenv(cName.Raw.Data.Pointer);
            return status == 0;
        }
    }
    public static VecPtr Enumerate() {
        let size = (usize) __sizeof <string >();
        let align = (usize) __alignof <string >();
        let drop_fn = DropStringValue();
        var entries = Foundation.Collections.VecIntrinsics.chic_rt_vec_with_capacity(size, align, 0usize, drop_fn);
        var index = 0;
        while (true)
        {
            let ptr = Std.Runtime.Startup.NativeStartup.GetEnvironmentPointer(index);
            if (ptr == null)
            {
                break;
            }
            let entry = Std.Runtime.Startup.RuntimeIntrinsics.chic_rt_startup_cstr_to_string(ptr);
            Foundation.Collections.Vec.Push <string >(ref entries, entry);
            index += 1;
        }
        return entries;
    }
}
public static class ProcessInfo
{
    public static VecPtr CommandLine() {
        let size = (usize) __sizeof <string >();
        let align = (usize) __alignof <string >();
        let drop_fn = EnvironmentVariables.DropStringValue();
        var args = Foundation.Collections.VecIntrinsics.chic_rt_vec_with_capacity(size, align, 0usize, drop_fn);
        let count = Std.Runtime.Startup.NativeStartup.StartupState.ArgCount();
        for (var i = 0; i <count; i += 1) {
            let ptr = Std.Runtime.Startup.NativeStartup.GetArgumentPointer(i);
            if (ptr == null)
            {
                continue;
            }
            let arg = Std.Runtime.Startup.RuntimeIntrinsics.chic_rt_startup_cstr_to_string(ptr);
            Foundation.Collections.Vec.Push <string >(ref args, arg);
        }
        return args;
    }
    public static void Exit(int code) {
        Std.Runtime.Startup.RuntimeIntrinsics.chic_rt_startup_exit(code);
    }
}
