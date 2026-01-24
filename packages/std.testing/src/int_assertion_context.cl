namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 32-bit signed integers.</summary>
public struct IntAssertionContext
{
    private readonly int _actual;
    public init(int value) {
        _actual = value;
    }
    public IntAssertionContext IsEqualTo(int expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public IntAssertionContext IsNotEqualTo(int unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    @allow(dead_code)
    public static bool operator !(IntAssertionContext _context) => false;
}
testcase Given_assert_int_is_equal_to_When_executed_Then_assert_int_is_equal_to()
{
    Assert.That(5).IsEqualTo(5);
}
testcase Given_assert_int_is_not_equal_to_When_executed_Then_assert_int_is_not_equal_to()
{
    Assert.That(5).IsNotEqualTo(6);
}
testcase Given_assert_int_is_equal_to_failure_When_executed_Then_assert_int_is_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.IntIsEqualMismatch);
}
testcase Given_assert_int_context_negation_When_executed_Then_assert_int_context_negation_returns_false()
{
    let ctx : IntAssertionContext = Assert.That(0);
    Assert.That(!ctx).IsFalse();
}
