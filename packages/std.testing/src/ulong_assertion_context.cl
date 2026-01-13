namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 64-bit unsigned integers.</summary>
public struct ULongAssertionContext
{
    private readonly ulong _actual;
    public init(ulong value) {
        _actual = value;
    }
    public ULongAssertionContext IsEqualTo(ulong expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public ULongAssertionContext IsNotEqualTo(ulong unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    public static bool operator ! (ULongAssertionContext context) => false;
}

testcase Given_assert_ulong_is_equal_to_When_executed_Then_assert_ulong_is_equal_to()
{
    let ctx: ULongAssertionContext = Assert.That(9ul);
    ctx.IsEqualTo(9ul);
}

testcase Given_assert_ulong_is_not_equal_to_When_executed_Then_assert_ulong_is_not_equal_to()
{
    let ctx: ULongAssertionContext = Assert.That(9ul);
    ctx.IsNotEqualTo(10ul);
}

testcase Given_assert_ulong_is_not_equal_to_failure_When_executed_Then_assert_ulong_is_not_equal_to_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.ULongIsNotEqualMismatch);
}

testcase Given_assert_ulong_context_negation_When_executed_Then_assert_ulong_context_negation_returns_false()
{
    let ctx: ULongAssertionContext = Assert.That(0ul);
    Assert.That(!ctx).IsFalse();
}
