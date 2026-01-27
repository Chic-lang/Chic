namespace Std.Platform.Thread;
import Std.Runtime;
// Exported trampolines that the native runtime calls when spawning or
// cancelling threads. They forward to the managed callbacks so thread payloads
// stay Chic-native.
public static class ThreadRuntimeExports
{
    // Export unmangled trampolines so the native runtime can resolve them.
    @extern("C") @export("chic_thread_invoke") public static void chic_thread_invoke(Std.Runtime.Collections.ValueMutPtr context) {
        RuntimeCallbacks.Invoke(context);
    }
    @extern("C") @export("chic_thread_drop") public static void chic_thread_drop(Std.Runtime.Collections.ValueMutPtr context) {
        RuntimeCallbacks.Drop(context);
    }
}
