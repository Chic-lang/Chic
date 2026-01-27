namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for boolean values.</summary>
public struct BoolAssertionContext
{
    private readonly bool _actual;
    public init(bool value) {
        _actual = value;
    }
    public BoolAssertionContext IsTrue() {
        if (!_actual)
        {
            throw new AssertionFailedException("expected true but was false");
        }
        return this;
    }
    public BoolAssertionContext IsFalse() {
        if (_actual)
        {
            throw new AssertionFailedException("expected false but was true");
        }
        return this;
    }
    public BoolAssertionContext IsEqualTo(bool expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public BoolAssertionContext IsNotEqualTo(bool unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ but they matched");
        }
        return this;
    }
    @allow(dead_code) public static bool operator !(BoolAssertionContext _context) => false;
    private static string FormatExpectedActual(bool expected, bool actual) {
        return "expected " + (expected ?"true" : "false") + " but was " + (actual ?"true" : "false");
    }
}
testcase Given_assert_bool_is_true_When_executed_Then_assert_bool_is_true()
{
    let ctx : BoolAssertionContext = Assert.That(true);
    ctx.IsTrue();
}
testcase Given_assert_bool_is_false_When_executed_Then_assert_bool_is_false()
{
    let ctx : BoolAssertionContext = Assert.That(false);
    ctx.IsFalse();
}
testcase Given_assert_bool_is_equal_to_When_executed_Then_assert_bool_is_equal_to()
{
    Assert.That(true).IsEqualTo(true);
}
testcase Given_assert_bool_is_not_equal_to_When_executed_Then_assert_bool_is_not_equal_to()
{
    Assert.That(true).IsNotEqualTo(false);
}
testcase Given_assert_bool_is_true_failure_When_executed_Then_assert_bool_is_true_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.BoolIsFalseOnTrue);
}
testcase Given_assert_bool_is_false_failure_When_executed_Then_assert_bool_is_false_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.BoolIsTrueOnFalse);
}
testcase Given_assert_bool_is_equal_to_failure_When_executed_Then_assert_bool_is_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.BoolIsEqualMismatch);
}
testcase Given_assert_bool_is_not_equal_to_failure_When_executed_Then_assert_bool_is_not_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.BoolIsNotEqualMismatch);
}
testcase Given_assert_bool_context_negation_When_executed_Then_assert_bool_context_negation_returns_false()
{
    let ctx : BoolAssertionContext = Assert.That(true);
    Assert.That(!ctx).IsFalse();
}
