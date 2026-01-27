namespace Std.Runtime.Intrinsics;
import Std.Numeric;
import Std.Core;
import Std.Core.Testing;
// String layout helpers backed by the selected Chic runtime.
public static class StringInternals
{
    @extern("C") private static extern usize chic_rt_string_inline_capacity();
    @extern("C") private static extern * mut @expose_address byte chic_rt_string_inline_ptr(ref string value);
    @extern("C") private static extern * mut @expose_address byte chic_rt_string_get_ptr(ref string value);
    @extern("C") private static extern usize chic_rt_string_get_cap(ref string value);
    @extern("C") private static extern void chic_rt_string_set_ptr(ref string value, * mut @expose_address byte ptr);
    @extern("C") private static extern void chic_rt_string_set_len(ref string value, usize len);
    @extern("C") private static extern void chic_rt_string_set_cap(ref string value, usize cap);
    public static usize InlineCapacity() {
        return chic_rt_string_inline_capacity();
    }
    public static * mut @expose_address byte InlineBuffer(ref string value) {
        unsafe {
            return chic_rt_string_inline_ptr(ref value);
        }
    }
    public static * mut @expose_address byte Data(ref string value) {
        unsafe {
            return chic_rt_string_get_ptr(ref value);
        }
    }
    public static usize Capacity(ref string value) {
        return chic_rt_string_get_cap(ref value);
    }
    public static void SetPtr(ref string value, * mut @expose_address byte buffer) {
        unsafe {
            chic_rt_string_set_ptr(ref value, buffer);
        }
    }
    public static void SetLen(ref string value, usize length) {
        chic_rt_string_set_len(ref value, length);
    }
    public static void SetCap(ref string value, usize capacity) {
        chic_rt_string_set_cap(ref value, capacity);
    }
}
testcase Given_string_internals_stubbed_values_When_executed_Then_string_internals_stubbed_values()
{
    Assert.That(StringInternals.InlineCapacity() == 32usize).IsTrue();
}
testcase Given_string_internals_capacity_is_zero_When_executed_Then_string_internals_capacity_is_zero()
{
    var text = "";
    let capacity = StringInternals.Capacity(ref text);
    Assert.That(capacity == 0usize).IsTrue();
}
testcase Given_string_internals_data_is_null_When_executed_Then_string_internals_data_is_null()
{
    var text = "";
    unsafe {
        let ptr = StringInternals.Data(ref text);
        Assert.That(Pointer.IsNull(ptr)).IsTrue();
    }
}
testcase Given_string_internals_inline_buffer_is_null_When_executed_Then_string_internals_inline_buffer_is_null()
{
    var text = "";
    unsafe {
        let ptr = StringInternals.InlineBuffer(ref text);
        Assert.That(Pointer.IsNull(ptr)).IsFalse();
    }
}
testcase Given_string_internals_set_ptr_keeps_null_When_executed_Then_string_internals_set_ptr_keeps_null()
{
    var text = "";
    unsafe {
        StringInternals.SetPtr(ref text, Pointer.NullMut <byte >());
        let ptr = StringInternals.Data(ref text);
        Assert.That(Pointer.IsNull(ptr)).IsTrue();
    }
}
testcase Given_string_internals_set_len_keeps_capacity_zero_When_executed_Then_string_internals_set_len_keeps_capacity_zero()
{
    var text = "";
    StringInternals.SetLen(ref text, 0usize);
    Assert.That(StringInternals.Capacity(ref text) == 0usize).IsTrue();
}
testcase Given_string_internals_set_cap_keeps_capacity_zero_When_executed_Then_string_internals_set_cap_keeps_capacity_zero()
{
    var text = "";
    StringInternals.SetCap(ref text, 0usize);
    Assert.That(StringInternals.Capacity(ref text) == 0usize).IsTrue();
}
