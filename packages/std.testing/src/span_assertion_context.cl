namespace Std.Testing;
import Std;
import Std.Runtime;
import Std.Span;
import Std.Numeric;
/// <summary>Fluent assertions for spans.</summary>
public struct SpanAssertionContext <T >
{
    private readonly ReadOnlySpan <T >_actual;
    public init(ReadOnlySpan <T >value) {
        _actual = value;
    }
    public SpanAssertionContext <T >HasLength(usize expected) {
        if (_actual.Length != expected)
        {
            throw new AssertionFailedException("expected span length to match");
        }
        return this;
    }
    public SpanAssertionContext <T >IsEmpty() {
        if (_actual.Length != 0)
        {
            throw new AssertionFailedException("expected empty span");
        }
        return this;
    }
    public SpanAssertionContext <T >IsNotEmpty() {
        if (_actual.Length == 0)
        {
            throw new AssertionFailedException("expected non-empty span but length was 0");
        }
        return this;
    }
    public SpanAssertionContext <T >IsEqualTo(ReadOnlySpan <T >expected) {
        if (_actual.Length != expected.Length)
        {
            throw new AssertionFailedException("expected spans to have the same length");
        }
        var idx = 0usize;
        while (idx <expected.Length)
        {
            let actualValue = _actual[idx];
            let expectedValue = expected[idx];
            if (!AreEqual (actualValue, expectedValue))
            {
                throw new AssertionFailedException("expected spans to match but found a different element (expected " + FormatValue(expectedValue) + " but was " + FormatValue(actualValue) + ")");
            }
            idx = idx + 1usize;
        }
        return this;
    }
    public SpanAssertionContext <T >IsEqualTo(Span <T >expected) {
        return IsEqualTo(expected.AsReadOnly());
    }
    public SpanAssertionContext <T >IsNotEqualTo(ReadOnlySpan <T >unexpected) {
        if (SequenceEqual (_actual, unexpected))
        {
            throw new AssertionFailedException("expected sequences to differ but they matched");
        }
        return this;
    }
    public SpanAssertionContext <T >IsNotEqualTo(Span <T >unexpected) {
        return IsNotEqualTo(unexpected.AsReadOnly());
    }
    public static bool operator !(SpanAssertionContext <T >context) => false;
    private static bool SequenceEqual(ReadOnlySpan <T >left, ReadOnlySpan <T >right) {
        if (left.Length != right.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <left.Length)
        {
            if (!AreEqual (left[idx], right[idx]))
            {
                return false;
            }
            idx = idx + 1usize;
        }
        return true;
    }
    private static bool AreEqual(T left, T right) {
        let eqFn = (isize) __eq_glue_of <T >();
        if (eqFn == 0isize)
        {
            throw new AssertionFailedException("expected values to support equality");
        }
        unsafe {
            var * mut @expose_address T leftPtr = & left;
            var * mut @expose_address T rightPtr = & right;
            let leftBytes = PointerIntrinsics.AsByteConstFromMut(leftPtr);
            let rightBytes = PointerIntrinsics.AsByteConstFromMut(rightPtr);
            return EqRuntime.Invoke(eqFn, leftBytes, rightBytes);
        }
    }
    private static string FormatValue(T value) {
        return "<value>";
    }
}
testcase Given_assert_span_has_length_When_executed_Then_assert_span_has_length()
{
    let span = ReadOnlySpan.FromString("hi");
    Assert.That(span).HasLength(2usize);
}
testcase Given_assert_span_is_empty_When_executed_Then_assert_span_is_empty()
{
    let span = ReadOnlySpan <byte >.Empty;
    Assert.That(span).IsEmpty();
}
testcase Given_assert_span_is_not_empty_When_executed_Then_assert_span_is_not_empty()
{
    let span = ReadOnlySpan.FromString("hi");
    Assert.That(span).IsNotEmpty();
}
testcase Given_assert_span_is_equal_to_When_executed_Then_assert_span_is_equal_to()
{
    let left = ReadOnlySpan.FromString("hi");
    let right = ReadOnlySpan.FromString("hi");
    Assert.That(left).IsEqualTo(right);
}
testcase Given_assert_span_is_not_equal_to_When_executed_Then_assert_span_is_not_equal_to()
{
    let left = ReadOnlySpan.FromString("hi");
    let right = ReadOnlySpan.FromString("ho");
    Assert.That(left).IsNotEqualTo(right);
}
testcase Given_assert_span_has_length_failure_When_executed_Then_assert_span_has_length_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.SpanLengthMismatch);
}
testcase Given_assert_span_is_empty_failure_When_executed_Then_assert_span_is_empty_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.SpanIsEmptyMismatch);
}
testcase Given_assert_span_is_not_empty_failure_When_executed_Then_assert_span_is_not_empty_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.SpanIsNotEmptyMismatch);
}
testcase Given_assert_span_is_equal_to_failure_When_executed_Then_assert_span_is_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.SpanIsEqualMismatch);
}
testcase Given_assert_span_is_not_equal_to_failure_When_executed_Then_assert_span_is_not_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.SpanIsNotEqualMismatch);
}
testcase Given_assert_span_is_equal_to_span_When_executed_Then_assert_span_is_equal_to_span()
{
    var left = new byte[2];
    left[0] = 1u8;
    left[1] = 2u8;
    var right = new byte[2];
    right[0] = 1u8;
    right[1] = 2u8;
    var leftSpan = Span <byte >.FromArray(ref left);
    var rightSpan = Span <byte >.FromArray(ref right);
    Assert.That(leftSpan).IsEqualTo(rightSpan);
}
testcase Given_assert_span_is_not_equal_to_span_When_executed_Then_assert_span_is_not_equal_to_span()
{
    var left = new byte[2];
    left[0] = 1u8;
    left[1] = 2u8;
    var right = new byte[2];
    right[0] = 1u8;
    right[1] = 3u8;
    var leftSpan = Span <byte >.FromArray(ref left);
    var rightSpan = Span <byte >.FromArray(ref right);
    Assert.That(leftSpan).IsNotEqualTo(rightSpan);
}
testcase Given_assert_span_context_negation_When_executed_Then_assert_span_context_negation_returns_false()
{
    let ctx : SpanAssertionContext <byte >= Assert.That(ReadOnlySpan <byte >.Empty);
    Assert.That(SpanAssertionContext <byte >.op_LogicalNot(ctx)).IsFalse();
}
