namespace Std.Testing;
import Std;
/// <summary>Fluent assertions for 32-bit floating-point values.</summary>
public struct FloatAssertionContext
{
    private readonly float _actual;
    public init(float value) {
        _actual = value;
    }
    public FloatAssertionContext IsEqualTo(float expected) {
        if (_actual != expected)
        {
            throw new AssertionFailedException("expected values to be equal");
        }
        return this;
    }
    public FloatAssertionContext IsNotEqualTo(float unexpected) {
        if (_actual == unexpected)
        {
            throw new AssertionFailedException("expected values to differ");
        }
        return this;
    }
    public FloatAssertionContext IsCloseTo(float target, float tolerance) {
        if (tolerance < 0.0f)
        {
            throw new AssertionFailedException("expected non-negative tolerance");
        }
        var delta = _actual - target;
        if (delta < 0.0f)
        {
            delta = 0.0f - delta;
        }
        if (delta > tolerance)
        {
            throw new AssertionFailedException("expected value to be within tolerance of target");
        }
        return this;
    }
    public static bool operator ! (FloatAssertionContext context) => false;
}

testcase Given_assert_float_is_equal_to_When_executed_Then_assert_float_is_equal_to()
{
    Assert.That(1.25f).IsEqualTo(1.25f);
}

testcase Given_assert_float_is_not_equal_to_When_executed_Then_assert_float_is_not_equal_to()
{
    Assert.That(1.25f).IsNotEqualTo(2.5f);
}

testcase Given_assert_float_is_close_to_When_executed_Then_assert_float_is_close_to()
{
    Assert.That(1.25f).IsCloseTo(1.3f, 0.1f);
}

testcase Given_assert_float_is_close_to_failure_When_executed_Then_assert_float_is_close_to_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.FloatIsCloseMismatch);
}

testcase Given_assert_float_is_close_to_negative_tolerance_failure_When_executed_Then_assert_float_is_close_to_negative_tolerance_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.FloatIsCloseNegativeTolerance);
}

testcase Given_assert_float_context_negation_When_executed_Then_assert_float_context_negation_returns_false()
{
    let ctx: FloatAssertionContext = Assert.That(0.0f);
    Assert.That(!ctx).IsFalse();
}
