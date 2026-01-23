namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for pointer-sized unsigned integers.</summary>
public struct USizeAssertionContext
{
    private readonly usize _actual;
    public init(usize value) {
        _actual = value;
    }
    public USizeAssertionContext IsEqualTo(usize expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public USizeAssertionContext IsNotEqualTo(usize unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    public static bool operator !(USizeAssertionContext context) => false;
}
testcase Given_assert_usize_is_equal_to_When_executed_Then_assert_usize_is_equal_to()
{
    let ctx : USizeAssertionContext = Assert.That(5usize);
    ctx.IsEqualTo(5usize);
}
testcase Given_assert_usize_is_not_equal_to_When_executed_Then_assert_usize_is_not_equal_to()
{
    let ctx : USizeAssertionContext = Assert.That(5usize);
    ctx.IsNotEqualTo(6usize);
}
testcase Given_assert_usize_is_not_equal_to_failure_When_executed_Then_assert_usize_is_not_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.USizeIsNotEqualMismatch);
}
testcase Given_assert_usize_context_negation_When_executed_Then_assert_usize_context_negation_returns_false()
{
    let ctx : USizeAssertionContext = Assert.That(0usize);
    Assert.That(!ctx).IsFalse();
}
