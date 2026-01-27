namespace Std.Core;
import Std.Memory;
import Std.Core.Testing;
/// <summary>
/// Core-level helpers that avoid pulling higher-level allocator or platform
/// surfaces into foundational types.
/// </summary>
public static class CoreIntrinsics
{
    /// <summary>
    /// Writes the zero-value for <typeparamref name="T"/> into <paramref name="target"/>.
    /// The operation is implemented via the runtime memset primitive and honours the
    /// recorded size/alignment for <typeparamref name="T"/>.
    /// </summary>
    public static void InitializeDefault <T >(out T target) {
        Intrinsics.ZeroInit(out target);
    }
    /// <summary>
    /// Returns the zero-value for <typeparamref name="T"/>.
    /// </summary>
    public static T DefaultValue <T >() {
        return Intrinsics.ZeroValue <T >();
    }
}
testcase Given_core_intrinsics_default_value_When_executed_Then_core_intrinsics_default_value()
{
    let value = CoreIntrinsics.DefaultValue <int >();
    Assert.That(value == 0).IsTrue();
}
testcase Given_core_intrinsics_initialize_default_When_executed_Then_core_intrinsics_initialize_default()
{
    var value = 99;
    CoreIntrinsics.InitializeDefault(out value);
    Assert.That(value == 0).IsTrue();
}
