namespace Std.Runtime;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Core;
import Std.Core.Testing;
public static class DropRuntime
{
    @extern("C") private static extern void chic_rt_drop_invoke(isize dropFn, * mut @expose_address byte value);
    @extern("C") private static extern isize chic_rt_drop_noop_ptr();
    public static void Invoke(isize dropFn, * mut @expose_address byte value) {
        chic_rt_drop_invoke(dropFn, value);
    }
    public static void Invoke(isize dropFn, Std.Runtime.Collections.ValueMutPtr value) {
        chic_rt_drop_invoke(dropFn, value.Pointer);
    }
    public static isize DropNoopPtr() {
        return chic_rt_drop_noop_ptr();
    }
}

testcase Given_drop_runtime_noop_invoke_keeps_handle_valid_When_executed_Then_drop_runtime_noop_invoke_keeps_handle_valid()
{
    let dropFn = DropRuntime.DropNoopPtr();
    unsafe {
        var value = 1;
        var * mut @expose_address int ptr = & value;
        let handle = ValuePointer.CreateMut(
            PointerIntrinsics.AsByteMut(ptr),
            __sizeof<int>(),
            __alignof<int>()
        );
        DropRuntime.Invoke(dropFn, handle.Pointer);
        DropRuntime.Invoke(dropFn, handle);
        Assert.That(ValuePointer.IsNullMut(handle)).IsFalse();
    }
}

testcase Given_drop_runtime_noop_ptr_is_nonzero_When_executed_Then_drop_runtime_noop_ptr_is_nonzero()
{
    let ptr = DropRuntime.DropNoopPtr();
    Assert.That(ptr != 0isize).IsTrue();
}
