namespace Std.Testing;
import Std;
import Std.Core;
import Std.Strings;
/// <summary>Fluent assertions for UTF-8 strings.</summary>
public struct StringAssertionContext
{
    private readonly string _actual;
    public init(string value) {
        _actual = value;
    }
    public StringAssertionContext IsNull() {
        if (_actual is not null) {
            throw new AssertionFailedException("expected null but was non-null");
        }
        return this;
    }
    public StringAssertionContext IsNotNull() {
        if (_actual is null) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        return this;
    }
    public StringAssertionContext IsEqualTo(string expected) {
        if (_actual is null) {
            if (expected is null) {
                return this;
            }
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        if (!(_actual == expected))
        {
            throw new AssertionFailedException(FormatExpectedActual(expected, _actual));
        }
        return this;
    }
    public StringAssertionContext IsNotEqualTo(string unexpected) {
        if (_actual is null) {
            if (unexpected is null) {
                throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
            }
            return this;
        }
        if (_actual == unexpected)
        {
            throw new AssertionFailedException(FormatExpectedActual(unexpected, _actual));
        }
        return this;
    }
    public StringAssertionContext Contains(string substring) {
        if (_actual is null) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        if (substring is null) {
            throw new AssertionFailedException("expected non-null substring");
        }
        if (_actual.IndexOf(substring) <0)
        {
            throw new AssertionFailedException("expected " + FormatValue(_actual) + " to contain " + FormatValue(substring));
        }
        return this;
    }
    public StringAssertionContext StartsWith(string prefix) {
        if (_actual is null) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        if (prefix is null) {
            throw new AssertionFailedException("expected non-null prefix");
        }
        if (!_actual.StartsWith(prefix))
        {
            throw new AssertionFailedException("expected " + FormatValue(_actual) + " to start with " + FormatValue(prefix));
        }
        return this;
    }
    public StringAssertionContext EndsWith(string suffix) {
        if (_actual is null) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        if (suffix is null) {
            throw new AssertionFailedException("expected non-null suffix");
        }
        if (suffix.Length > _actual.Length)
        {
            throw new AssertionFailedException("expected " + FormatValue(_actual) + " to end with " + FormatValue(suffix));
        }
        let start = _actual.Length - suffix.Length;
        let tail = _actual.Substring((int) start, (int) suffix.Length);
        if (!(tail == suffix))
        {
            throw new AssertionFailedException("expected " + FormatValue(_actual) + " to end with " + FormatValue(suffix));
        }
        return this;
    }
    private static string FormatExpectedActual(string expected, string actual) {
        return "expected " + FormatValue(expected) + " but was " + FormatValue(actual);
    }
    private static string FormatValue(string value) {
        if (value is null) {
            return "null";
        }
        return "\"" + value + "\"";
    }
    public static bool operator ! (StringAssertionContext context) => false;
}

testcase Given_assert_string_is_null_When_executed_Then_assert_string_is_null()
{
    let nullString = CoreIntrinsics.DefaultValue<string>();
    let ctx: StringAssertionContext = Assert.That(nullString);
    ctx.IsNull();
}

testcase Given_assert_string_is_not_null_When_executed_Then_assert_string_is_not_null()
{
    let ctx: StringAssertionContext = Assert.That("hi");
    ctx.IsNotNull();
}

testcase Given_assert_string_is_equal_to_When_executed_Then_assert_string_is_equal_to()
{
    Assert.That("hi").IsEqualTo("hi");
}

testcase Given_assert_string_is_not_equal_to_When_executed_Then_assert_string_is_not_equal_to()
{
    Assert.That("hi").IsNotEqualTo("bye");
}

testcase Given_assert_string_is_equal_to_failure_When_executed_Then_assert_string_is_equal_to_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringIsEqualMismatch);
}

testcase Given_assert_string_is_not_equal_to_failure_When_executed_Then_assert_string_is_not_equal_to_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringIsNotEqualMismatch);
}

testcase Given_assert_string_is_not_null_failure_When_executed_Then_assert_string_is_not_null_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringIsNotNullOnNull);
}

testcase Given_assert_string_is_null_failure_When_executed_Then_assert_string_is_null_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringIsNullOnNonNull);
}

testcase Given_assert_string_is_not_equal_to_null_failure_When_executed_Then_assert_string_is_not_equal_to_null_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringIsNotEqualOnNulls);
}

testcase Given_assert_string_contains_When_executed_Then_assert_string_contains()
{
    Assert.That("hello world").Contains("world");
}

testcase Given_assert_string_contains_failure_When_executed_Then_assert_string_contains_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringContainsMissing);
}

testcase Given_assert_string_starts_with_When_executed_Then_assert_string_starts_with()
{
    Assert.That("hello").StartsWith("he");
}

testcase Given_assert_string_starts_with_failure_When_executed_Then_assert_string_starts_with_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringStartsWithMissing);
}

testcase Given_assert_string_ends_with_When_executed_Then_assert_string_ends_with()
{
    Assert.That("hello").EndsWith("lo");
}

testcase Given_assert_string_ends_with_failure_When_executed_Then_assert_string_ends_with_failure()
{
    Assert.Throws<AssertionFailedException>(FailureActions.StringEndsWithMissing);
}
