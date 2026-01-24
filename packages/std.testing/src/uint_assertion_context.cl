namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 32-bit unsigned integers.</summary>
public struct UIntAssertionContext
{
    private readonly uint _actual;
    public init(uint value) {
        _actual = value;
    }
    public UIntAssertionContext IsEqualTo(uint expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public UIntAssertionContext IsNotEqualTo(uint unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    @allow(dead_code)
    public static bool operator !(UIntAssertionContext _context) => false;
}
testcase Given_assert_uint_is_equal_to_When_executed_Then_assert_uint_is_equal_to()
{
    Assert.That(3u).IsEqualTo(3u);
}
testcase Given_assert_uint_is_not_equal_to_When_executed_Then_assert_uint_is_not_equal_to()
{
    Assert.That(3u).IsNotEqualTo(4u);
}
testcase Given_assert_uint_is_equal_to_failure_When_executed_Then_assert_uint_is_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.UIntIsEqualMismatch);
}
testcase Given_assert_uint_context_negation_When_executed_Then_assert_uint_context_negation_returns_false()
{
    let ctx : UIntAssertionContext = Assert.That(0u);
    Assert.That(!ctx).IsFalse();
}
