namespace Std.Testing;
import Std;
import Std.Core;
import Std.Numeric;
import Std.Runtime;
/// <summary>
/// Fluent assertion helper returned by <see cref="Assert.That"/> to validate values.
/// </summary>
public struct AssertionContext <T >
{
    private readonly T _value;
    /// <summary>
    /// Captures the value being asserted.
    /// </summary>
    public init(T value) {
        _value = value;
    }
    /// <summary>
    /// Asserts that the captured value equals the expected value.
    /// </summary>
    /// <param name="expected">Expected value.</param>
    public AssertionContext <T >IsEqualTo(T expected) {
        if (!AreEqual (expected))
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _value));
        }
        return this;
    }
    /// <summary>
    /// Asserts that the captured value does not equal the unexpected value.
    /// </summary>
    /// <param name="unexpected">Value that should not match.</param>
    public AssertionContext <T >IsNotEqualTo(T unexpected) {
        if (AreEqual (unexpected))
        {
            throw new AssertionFailedException(FormatUnexpectedActual(unexpected, _value));
        }
        return this;
    }
    /// <summary>
    /// Asserts that the captured value is null (compares against the default value for <typeparamref name="T"/>).
    /// </summary>
    public AssertionContext <T >IsNull() {
        let defaultValue = CoreIntrinsics.DefaultValue <T >();
        let matches = AreEqual(defaultValue);
        let _ = defaultValue;
        if (!matches)
        {
            throw new AssertionFailedException("expected null but was non-null");
        }
        return this;
    }
    /// <summary>
    /// Asserts that the captured value is not null (compares against the default value for <typeparamref name="T"/>).
    /// </summary>
    public AssertionContext <T >IsNotNull() {
        let defaultValue = CoreIntrinsics.DefaultValue <T >();
        let matches = AreEqual(defaultValue);
        let _ = defaultValue;
        if (matches)
        {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        return this;
    }
    /// <summary>
    /// Asserts that the captured boolean value is true.
    /// </summary>
    public AssertionContext <T >IsTrue() {
        if (__type_id_of <T > () != __type_id_of <bool > ())
        {
            throw new AssertionFailedException("expected a boolean value");
        }
        let defaultValue = CoreIntrinsics.DefaultValue <T >();
        if (AreEqual (defaultValue))
        {
            throw new AssertionFailedException("expected true but was false");
        }
        return this;
    }
    /// <summary>
    /// Asserts that the captured boolean value is false.
    /// </summary>
    public AssertionContext <T >IsFalse() {
        if (__type_id_of <T > () != __type_id_of <bool > ())
        {
            throw new AssertionFailedException("expected a boolean value");
        }
        let defaultValue = CoreIntrinsics.DefaultValue <T >();
        if (!AreEqual (defaultValue))
        {
            throw new AssertionFailedException("expected false but was true");
        }
        return this;
    }
    /// <summary>
	    /// Logical negation operator always evaluates to false for assertion contexts to avoid misuse in conditionals.
	    /// </summary>
	    /// <param name="context">Context to negate.</param>
	    /// <returns>Always false.</returns>
	    @allow(dead_code)
	    public static bool operator !(AssertionContext <T >_context) => false;
    private static string FormatExpectedActual(T expected, T actual) {
        return "expected " + FormatValue(expected) + " but was " + FormatValue(actual);
    }
    private static string FormatUnexpectedActual(T unexpected, T actual) {
        return "expected not " + FormatValue(unexpected) + " but was " + FormatValue(actual);
    }
    private static string FormatValue(T value) {
        if (__type_id_of <T > () == __type_id_of <bool > ())
        {
            unsafe {
                var * mut @expose_address T ptr = & value;
                let raw = PointerIntrinsics.AsByteConstFromMut(ptr);
                let b = * raw;
                return b == 0u8 ?"false" : "true";
            }
        }
        return "<value>";
    }
    private bool AreEqual(T other) {
        let eqFn = (isize) __eq_glue_of <T >();
        if (eqFn == 0isize)
        {
            let _ = other;
            throw new AssertionFailedException("expected values to support equality");
        }
        unsafe {
            var * mut @expose_address T leftPtr = & _value;
            var * mut @expose_address T rightPtr = & other;
            let leftBytes = PointerIntrinsics.AsByteConstFromMut(leftPtr);
            let rightBytes = PointerIntrinsics.AsByteConstFromMut(rightPtr);
            let result = EqRuntime.Invoke(eqFn, leftBytes, rightBytes);
            let _ = other;
            return result;
        }
    }
}
testcase Given_assert_generic_is_null_When_executed_Then_assert_generic_is_null()
{
    let nullException = CoreIntrinsics.DefaultValue <Exception >();
    Assert.That(nullException).IsNull();
    let _ = nullException;
}
testcase Given_assert_generic_is_not_null_When_executed_Then_assert_generic_is_not_null()
{
    let ex = new Exception("ok");
    Assert.That(ex).IsNotNull();
    let _ = ex;
}
testcase Given_assert_generic_is_equal_to_When_executed_Then_assert_generic_is_equal_to()
{
    Assert.That <int >(5).IsEqualTo(5);
}
testcase Given_assert_generic_is_not_equal_to_When_executed_Then_assert_generic_is_not_equal_to()
{
    Assert.That <int >(5).IsNotEqualTo(6);
}
testcase Given_assert_generic_is_equal_to_failure_When_executed_Then_assert_generic_is_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.GenericIsEqualMismatch);
}
testcase Given_assert_generic_is_not_equal_to_failure_When_executed_Then_assert_generic_is_not_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.GenericIsNotEqualMismatch);
}
testcase Given_assert_generic_is_true_failure_When_executed_Then_assert_generic_is_true_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.GenericIsTrueOnInt);
}
testcase Given_assert_generic_is_false_failure_When_executed_Then_assert_generic_is_false_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.GenericIsFalseOnInt);
}
testcase Given_assert_generic_is_true_When_executed_Then_assert_generic_is_true()
{
    Assert.That <bool >(true).IsTrue();
}
testcase Given_assert_generic_is_false_When_executed_Then_assert_generic_is_false()
{
    Assert.That <bool >(false).IsFalse();
}
testcase Given_assert_generic_requires_equality_When_executed_Then_missing_equality_throws()
{
    Assert.Throws <AssertionFailedException >(() => {
        let value = new NoEqualityType {
            Value = 1
        }
        ; Assert.That <NoEqualityType >(value).IsEqualTo(value);
    }
    );
}
testcase Given_assert_generic_custom_type_is_not_equal_to_When_executed_Then_assert_generic_custom_type_is_not_equal_to()
{
    let left = new EquatableType {
        Value = 1
    }
    ;
    let right = new EquatableType {
        Value = 2
    }
    ;
    Assert.That(left).IsNotEqualTo(right);
}
testcase Given_assert_generic_custom_type_is_not_equal_to_failure_path_When_executed_Then_unexpected_match_throws()
{
    Assert.Throws <AssertionFailedException >(() => {
        let value = new EquatableType {
            Value = 1
        }
        ; Assert.That(value).IsNotEqualTo(value);
    }
    );
}
testcase Given_assert_generic_custom_type_context_negation_When_executed_Then_generic_context_negation_returns_false()
{
    let value = new EquatableType {
        Value = 1
    }
    ;
    let ctx = Assert.That(value);
    Assert.That(!ctx).IsFalse();
}
