namespace Std.Runtime;
import Std.Numeric;
import Std.Core;
import Std.Core.Testing;
public static class EqRuntime
{
    @extern("C") private static extern int chic_rt_eq_invoke(isize eqFn, * const @readonly @expose_address byte left,
    * const @readonly @expose_address byte right);
    public static bool Invoke(isize eqFn, * const @readonly @expose_address byte left, * const @readonly @expose_address byte right) {
        if (eqFn == 0 || left == null || right == null)
        {
            return false;
        }
        return chic_rt_eq_invoke(eqFn, left, right) != 0;
    }
}

testcase Given_eq_runtime_invoke_null_eq_false_When_executed_Then_eq_runtime_invoke_null_eq_false()
{
    unsafe {
        var left = 5;
        var right = 5;
        var * mut @expose_address int leftPtr = & left;
        var * mut @expose_address int rightPtr = & right;
        let leftBytes = PointerIntrinsics.AsByteConstFromMut(leftPtr);
        let rightBytes = PointerIntrinsics.AsByteConstFromMut(rightPtr);
        Assert.That(EqRuntime.Invoke(0isize, leftBytes, rightBytes)).IsFalse();
    }
}

testcase Given_eq_runtime_invoke_null_left_false_When_executed_Then_eq_runtime_invoke_null_left_false()
{
    unsafe {
        var right = 5;
        var * mut @expose_address int rightPtr = & right;
        let rightBytes = PointerIntrinsics.AsByteConstFromMut(rightPtr);
        Assert.That(EqRuntime.Invoke(1isize, PointerIntrinsics.AsByteConstFromMut((* mut @expose_address int) 0), rightBytes))
            .IsFalse();
    }
}
