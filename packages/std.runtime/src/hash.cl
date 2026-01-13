namespace Std.Runtime;
import Std.Runtime.Collections;
import Std.Core;
import Std.Core.Testing;
public static class HashRuntime
{
    @extern("C") private static extern ulong chic_rt_hash_invoke(isize glue, ValueConstPtr value);
    public static ulong Invoke(isize glue, ValueConstPtr value) {
        return chic_rt_hash_invoke(glue, value);
    }
}

testcase Given_hash_runtime_returns_zero_for_null_When_executed_Then_hash_runtime_returns_zero_for_null()
{
    let ptr = ValuePointer.NullConst(0usize, 0usize);
    let hash = HashRuntime.Invoke(0isize, ptr);
    Assert.That(hash == 0UL).IsTrue();
}
