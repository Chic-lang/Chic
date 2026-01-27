namespace Std.Runtime.InteropServices;
import Std.Core;
import Std.Core.Testing;
/// Layout directives recognised by `@StructLayout`.
public enum LayoutKind
{
    Sequential = 0,
}

testcase Given_layout_kind_sequential_is_zero_When_executed_Then_layout_kind_sequential_is_zero()
{
    Assert.That(LayoutKind.Sequential == LayoutKind.Sequential).IsTrue();
}
