namespace Std.Testing;
import Std;
import Std.Core;
import Std.Numeric;
import Std.Span;
/// <summary>
/// Entry point for fluent assertions.
/// </summary>
public static class Assert
{
    public static BoolAssertionContext That(bool value) {
        return new BoolAssertionContext(value);
    }
    public static IntAssertionContext That(int value) {
        return new IntAssertionContext(value);
    }
    public static UIntAssertionContext That(uint value) {
        return new UIntAssertionContext(value);
    }
    public static LongAssertionContext That(long value) {
        return new LongAssertionContext(value);
    }
    public static ULongAssertionContext That(ulong value) {
        return new ULongAssertionContext(value);
    }
    public static USizeAssertionContext That(usize value) {
        return new USizeAssertionContext(value);
    }
    public static FloatAssertionContext That(float value) {
        return new FloatAssertionContext(value);
    }
    public static DoubleAssertionContext That(double value) {
        return new DoubleAssertionContext(value);
    }
    public static StringAssertionContext That(string value) {
        return new StringAssertionContext(value);
    }
    public static AssertionContext <char >That(char value) {
        return new AssertionContext <char >(value);
    }
    public static SpanAssertionContext <T >That <T >(ReadOnlySpan <T >value) {
        return new SpanAssertionContext <T >(value);
    }
    public static SpanAssertionContext <T >That <T >(Span <T >value) {
        return new SpanAssertionContext <T >(value.AsReadOnly());
    }
    public static SpanAssertionContext <T >That <T >(T[] value) {
        return new SpanAssertionContext <T >(ReadOnlySpan <T >.FromArray(in value));
    }
    public static AssertionContext <T >That <T >(T value) {
        return new AssertionContext <T >(value);
    }
    /// <summary>
    /// Expect the provided action to throw a particular exception type.
    /// </summary>
    public static void Throws <TException >(ThrowingAction action) {
        if (action == null)
        {
            throw new AssertionFailedException("expected action to throw but received null delegate");
        }
        var caught = CoreIntrinsics.DefaultValue <Exception >();
        try {
            action();
        }
        catch(Exception ex) {
            caught = ex;
        }
        if (caught == null)
        {
            throw new AssertionFailedException("expected exception of the requested type to be thrown");
        }
        if (caught is TException) {
            return;
        }
        throw new AssertionFailedException("expected exception of the requested type but caught a different exception");
    }
}
static class FailureActions
{
    public static void BoolIsFalseOnTrue() {
        Assert.That(true).IsFalse();
    }
    public static void BoolIsTrueOnFalse() {
        Assert.That(false).IsTrue();
    }
    public static void BoolIsEqualMismatch() {
        Assert.That(true).IsEqualTo(false);
    }
    public static void BoolIsNotEqualMismatch() {
        Assert.That(true).IsNotEqualTo(true);
    }
    public static void UIntIsEqualMismatch() {
        Assert.That(1u).IsEqualTo(2u);
    }
    public static void ULongIsNotEqualMismatch() {
        Assert.That(7ul).IsNotEqualTo(7ul);
    }
    public static void USizeIsNotEqualMismatch() {
        Assert.That(2usize).IsNotEqualTo(2usize);
    }
    public static void IntIsEqualMismatch() {
        Assert.That(5).IsEqualTo(4);
    }
    public static void LongIsNotEqualMismatch() {
        Assert.That(12l).IsNotEqualTo(12l);
    }
    public static void FloatIsCloseMismatch() {
        Assert.That(1.25f).IsCloseTo(2.0f, 0.1f);
    }
    public static void FloatIsCloseNegativeTolerance() {
        Assert.That(1.25f).IsCloseTo(1.3f, - 0.1f);
    }
    public static void DoubleIsCloseMismatch() {
        Assert.That(2.5).IsCloseTo(3.0, 0.1);
    }
    public static void DoubleIsCloseNegativeTolerance() {
        Assert.That(2.5).IsCloseTo(2.6, - 0.1);
    }
    public static void StringIsEqualMismatch() {
        Assert.That("hi").IsEqualTo("nope");
    }
    public static void StringIsNotEqualMismatch() {
        Assert.That("same").IsNotEqualTo("same");
    }
    public static void StringIsNotNullOnNull() {
        let nullString = CoreIntrinsics.DefaultValue <string >();
        Assert.That(nullString).IsNotNull();
    }
    public static void StringIsNullOnNonNull() {
        Assert.That("x").IsNull();
    }
    public static void StringIsNotEqualOnNulls() {
        let nullString = CoreIntrinsics.DefaultValue <string >();
        Assert.That(nullString).IsNotEqualTo(nullString);
    }
    public static void StringContainsMissing() {
        Assert.That("hello").Contains("planet");
    }
    public static void StringStartsWithMissing() {
        Assert.That("hello").StartsWith("lo");
    }
    public static void StringEndsWithMissing() {
        Assert.That("hello").EndsWith("he");
    }
    public static void SpanLengthMismatch() {
        let span = ReadOnlySpan.FromString("hi");
        Assert.That(span).HasLength(3usize);
    }
    public static void SpanIsEmptyMismatch() {
        let span = ReadOnlySpan.FromString("hi");
        Assert.That(span).IsEmpty();
    }
    public static void SpanIsNotEmptyMismatch() {
        let span = ReadOnlySpan <byte >.Empty;
        Assert.That(span).IsNotEmpty();
    }
    public static void SpanIsEqualMismatch() {
        let left = ReadOnlySpan.FromString("hi");
        let right = ReadOnlySpan.FromString("ho");
        Assert.That(left).IsEqualTo(right);
    }
    public static void SpanIsNotEqualMismatch() {
        let left = ReadOnlySpan.FromString("hi");
        Assert.That(left).IsNotEqualTo(left);
    }
    public static void GenericIsEqualMismatch() {
        Assert.That <int >(5).IsEqualTo(4);
    }
    public static void GenericIsNotEqualMismatch() {
        Assert.That <int >(5).IsNotEqualTo(5);
    }
    public static void GenericIsTrueOnInt() {
        Assert.That(123).IsTrue();
    }
    public static void GenericIsFalseOnInt() {
        Assert.That(123).IsFalse();
    }
    public static void ThrowsWrongType() {
        Assert.Throws <ArgumentNullException >(() => {
            throw new ArgumentException("boom");
        }
        );
    }
    public static void ThrowsNullAction() {
        Assert.Throws <ArgumentException >(null);
    }
    public static void ThrowsMissingThrow() {
        Assert.Throws <ArgumentException >(() => {
            // no throw
        }
        );
    }
}
testcase Given_assert_throws_matches_type_When_executed_Then_assert_throws_matches_type()
{
    Assert.Throws <ArgumentException >(() => {
        throw new ArgumentException("boom");
    }
    );
}
testcase Given_assert_throws_wrong_type_failure_When_executed_Then_assert_throws_wrong_type_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.ThrowsWrongType);
}
testcase Given_assert_throws_null_action_failure_When_executed_Then_assert_throws_null_action_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.ThrowsNullAction);
}
testcase Given_assert_throws_missing_throw_failure_When_executed_Then_assert_throws_missing_throw_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.ThrowsMissingThrow);
}
testcase Given_assert_context_negation_When_executed_Then_context_negation_returns_false()
{
    Assert.That(!Assert.That(1)).IsFalse();
}
testcase Given_assert_bool_context_negation_When_executed_Then_assert_bool_context_negation_returns_false()
{
    Assert.That(!Assert.That(true)).IsFalse();
}
testcase Given_assert_ulong_context_negation_When_executed_Then_assert_ulong_context_negation_returns_false()
{
    Assert.That(!Assert.That(1ul)).IsFalse();
}
testcase Given_assert_uint_context_negation_When_executed_Then_assert_uint_context_negation_returns_false()
{
    Assert.That(!Assert.That(1u)).IsFalse();
}
testcase Given_assert_long_context_negation_When_executed_Then_assert_long_context_negation_returns_false()
{
    Assert.That(!Assert.That(1l)).IsFalse();
}
testcase Given_assert_usize_context_negation_When_executed_Then_assert_usize_context_negation_returns_false()
{
    Assert.That(!Assert.That(1usize)).IsFalse();
}
testcase Given_assert_float_context_negation_When_executed_Then_assert_float_context_negation_returns_false()
{
    Assert.That(!Assert.That(1.0f)).IsFalse();
}
testcase Given_assert_double_context_negation_When_executed_Then_assert_double_context_negation_returns_false()
{
    Assert.That(!Assert.That(1.0)).IsFalse();
}
testcase Given_assert_string_context_negation_When_executed_Then_assert_string_context_negation_returns_false()
{
    Assert.That(!Assert.That("x")).IsFalse();
}
