namespace Std.Runtime;
import Std.Core;
import Std.Memory;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Core.Testing;
/// Minimal native export bridge used while the shim is being removed. These definitions
/// are implemented in Chic and provide the symbols the platform support objects expect.
public static class NativeExports
{
    // Stub panic entry used only when native_startup is not linked; kept non-exported here
    public static int Panic(int code) {
        // Panic paths will be replaced by Std-native startup once the shim is gone.
        return code;
    }
    public static int Abort(int code) {
        return code;
    }
    public static void ThreadInvoke(ValueMutPtr context) {
        // No-op: `std.runtime` must not depend on `std.platform`.
        // Native thread trampolines live under `std.platform` (`ThreadRuntimeExports`).
        if (ValuePointer.IsNullMut (context))
        {
            return;
        }
    }
    public static void ThreadDrop(ValueMutPtr context) {
        // No-op: `std.runtime` must not depend on `std.platform`.
        if (ValuePointer.IsNullMut (context))
        {
            return;
        }
    }
    public static int StartupCallEntry(* const @readonly @expose_address byte function_ptr, uint flags, int argc, * mut * mut char argv,
    * mut * mut char envp) {
        return 0;
    }
}
testcase Given_native_exports_panic_returns_code_When_executed_Then_native_exports_panic_returns_code()
{
    Assert.That(NativeExports.Panic(5) == 5).IsTrue();
}
testcase Given_native_exports_abort_returns_code_When_executed_Then_native_exports_abort_returns_code()
{
    Assert.That(NativeExports.Abort(7) == 7).IsTrue();
}
testcase Given_native_exports_thread_callbacks_ignore_null_When_executed_Then_native_exports_thread_callbacks_ignore_null()
{
    let context = ValuePointer.NullMut(0usize, 0usize);
    NativeExports.ThreadInvoke(context);
    NativeExports.ThreadDrop(context);
    Assert.That(ValuePointer.IsNullMut(context)).IsTrue();
}
testcase Given_native_exports_startup_call_entry_returns_zero_When_executed_Then_native_exports_startup_call_entry_returns_zero()
{
    unsafe {
        let fnPtr = Pointer.NullConst <byte >();
        let argv = Pointer.NullMut <* mut char >();
        let envp = Pointer.NullMut <* mut char >();
        let result = NativeExports.StartupCallEntry(fnPtr, 0u, 0, argv, envp);
        Assert.That(result == 0).IsTrue();
    }
}
