namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 64-bit floating-point values.</summary>
public struct DoubleAssertionContext
{
    private readonly double _actual;
    public init(double value) {
        _actual = value;
    }
    public DoubleAssertionContext IsEqualTo(double expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public DoubleAssertionContext IsNotEqualTo(double unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    public DoubleAssertionContext IsCloseTo(double target, double tolerance) {
        if (tolerance <0.0)
        {
            throw new AssertionFailedException("expected non-negative tolerance");
        }
        var delta = _actual - target;
        if (delta <0.0)
        {
            delta = 0.0 - delta;
        }
        if (delta >tolerance)
        {
            throw new AssertionFailedException("expected value to be within tolerance of target");
        }
        return this;
    }
    public static bool operator !(DoubleAssertionContext context) => false;
}
testcase Given_assert_double_is_equal_to_When_executed_Then_assert_double_is_equal_to()
{
    Assert.That(2.5).IsEqualTo(2.5);
}
testcase Given_assert_double_is_not_equal_to_When_executed_Then_assert_double_is_not_equal_to()
{
    Assert.That(2.5).IsNotEqualTo(3.5);
}
testcase Given_assert_double_is_close_to_When_executed_Then_assert_double_is_close_to()
{
    Assert.That(2.5).IsCloseTo(2.55, 0.1);
}
testcase Given_assert_double_is_close_to_failure_When_executed_Then_assert_double_is_close_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.DoubleIsCloseMismatch);
}
testcase Given_assert_double_is_close_to_negative_tolerance_failure_When_executed_Then_assert_double_is_close_to_negative_tolerance_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.DoubleIsCloseNegativeTolerance);
}
testcase Given_assert_double_context_negation_When_executed_Then_assert_double_context_negation_returns_false()
{
    let ctx : DoubleAssertionContext = Assert.That(0.0);
    Assert.That(!ctx).IsFalse();
}
