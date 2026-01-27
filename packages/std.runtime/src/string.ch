namespace Std.Runtime;
import Std.Memory;
import Std.Numeric;
import Std.Runtime.Intrinsics;
import Std.Runtime.Collections;
import Std.Core;
import Std.Core.Testing;
public static class StringRuntime
{
    internal static string ZeroValue;
    private static bool InlineCapacityComputed;
    private static usize InlineCapacityValue;
    @extern("C") private static extern string chic_rt_string_from_slice(StrPtr slice);
    @extern("C") private static extern int chic_rt_string_clone(ref string dest, in string src);
    private static usize InlineCapacity() {
        if (!InlineCapacityComputed)
        {
            InlineCapacityValue = StringInternals.InlineCapacity();
            InlineCapacityComputed = true;
        }
        return InlineCapacityValue;
    }
    public static string Create() {
        var text = ZeroValue;
        ResetInline(ref text);
        return text;
    }
    public static void Destroy(ref string value) {
        var cap = StringInternals.Capacity(ref value);
        if ( (cap & InlineTag ()) != 0)
        {
            ResetInline(ref value);
            return;
        }
        var * mut @expose_address byte ptr = StringInternals.Data(ref value);
        unsafe {
            let handle = ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(ptr), cap, 1);
            if (!ValuePointer.IsNullMut (handle) && cap >0)
            {
                Std.Memory.GlobalAllocator.Free(handle);
            }
        }
        ResetInline(ref value);
    }
    public static bool IsInline(ref string value) {
        var cap = StringInternals.Capacity(ref value);
        return(cap & InlineTag()) != 0;
    }
    private static void ResetInline(ref string value) {
        var * mut @expose_address byte inlinePtr = StringInternals.InlineBuffer(ref value);
        var inlineCapacity = InlineCapacity();
        let handle = ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(inlinePtr), inlineCapacity, 1);
        Std.Memory.GlobalAllocator.Set(handle, 0, inlineCapacity);
        StringInternals.SetPtr(ref value, inlinePtr);
        StringInternals.SetLen(ref value, 0);
        StringInternals.SetCap(ref value, InlineTag() | inlineCapacity);
    }
    private static usize InlineTag() {
        unchecked {
            var max = NumericPlatform.UIntPtrMaxValue;
            return max ^ (max >> 1);
        }
    }
    public static string FromStr(str value) {
        let slice = StrPtr.FromStr(value);
        return chic_rt_string_from_slice(slice);
    }
    public static string Clone(in string value) {
        var cloned = Create();
        let status = chic_rt_string_clone(ref cloned, in value);
        if (status != 0)
        {
            throw new InvalidOperationException("chic_rt_string_clone failed");
        }
        return cloned;
    }
}
testcase Given_string_runtime_from_str_roundtrip_When_executed_Then_string_runtime_from_str_roundtrip()
{
    let text = StringRuntime.FromStr("hello");
    Assert.That(text == "hello").IsTrue();
}
testcase Given_string_runtime_clone_roundtrip_When_executed_Then_clone_roundtrips()
{
    let text = StringRuntime.FromStr("hello");
    let cloned = StringRuntime.Clone(in text);
    Assert.That(cloned == text).IsTrue();
}
testcase Given_string_runtime_is_inline_returns_false_for_stub_When_executed_Then_string_runtime_is_inline_returns_false_for_stub()
{
    var text = "";
    let isInline = StringRuntime.IsInline(ref text);
    Assert.That(isInline).IsFalse();
}
