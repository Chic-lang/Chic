namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 64-bit signed integers.</summary>
public struct LongAssertionContext
{
    private readonly long _actual;
    public init(long value) {
        _actual = value;
    }
    public LongAssertionContext IsEqualTo(long expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public LongAssertionContext IsNotEqualTo(long unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    @allow(dead_code) public static bool operator !(LongAssertionContext _context) => false;
}
testcase Given_assert_long_is_equal_to_When_executed_Then_assert_long_is_equal_to()
{
    let ctx : LongAssertionContext = Assert.That(12l);
    ctx.IsEqualTo(12l);
}
testcase Given_assert_long_is_not_equal_to_When_executed_Then_assert_long_is_not_equal_to()
{
    let ctx : LongAssertionContext = Assert.That(12l);
    ctx.IsNotEqualTo(13l);
}
testcase Given_assert_long_is_not_equal_to_failure_When_executed_Then_assert_long_is_not_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.LongIsNotEqualMismatch);
}
testcase Given_assert_long_context_negation_When_executed_Then_assert_long_context_negation_returns_false()
{
    let ctx : LongAssertionContext = Assert.That(0l);
    Assert.That(!ctx).IsFalse();
}
