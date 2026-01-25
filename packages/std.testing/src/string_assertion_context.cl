namespace Std.Testing;
import Std;
import Std.Core;
import Std.Span;
import Std.Strings;
/// <summary>Fluent assertions for UTF-8 strings.</summary>
public struct StringAssertionContext
{
    private readonly bool _isNull;
    private readonly ReadOnlySpan <byte >_actualUtf8;
    public init(string value) {
        _isNull = value is null;
        _actualUtf8 = _isNull ?CoreIntrinsics.DefaultValue <ReadOnlySpan <byte >>() : value.AsUtf8Span();
    }
    private static bool Utf8Equals(ReadOnlySpan <byte >left, ReadOnlySpan <byte >right) {
        if (left.Length != right.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <left.Length)
        {
            if (left[idx] != right[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
    private static int Utf8IndexOf(ReadOnlySpan <byte >haystack, ReadOnlySpan <byte >needle) {
        if (needle.Length == 0usize)
        {
            return 0;
        }
        if (needle.Length >haystack.Length)
        {
            return - 1;
        }
        var idx = 0usize;
        while (idx + needle.Length <= haystack.Length)
        {
            var matched = true;
            var needleIdx = 0usize;
            while (needleIdx <needle.Length)
            {
                if (haystack[idx + needleIdx] != needle[needleIdx])
                {
                    matched = false;
                    break;
                }
                needleIdx += 1usize;
            }
            if (matched)
            {
                return(int) idx;
            }
            idx += 1usize;
        }
        return - 1;
    }
    public StringAssertionContext IsNull() {
        if (!_isNull) {
            throw new AssertionFailedException("expected null but was non-null");
        }
        return this;
    }
    public StringAssertionContext IsNotNull() {
        if (_isNull) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        return this;
    }
    public StringAssertionContext IsEqualTo(string expected) {
        if (_isNull) {
            if (expected is null) {
                return this;
            }
            throw new AssertionFailedException("expected " + FormatValue(expected) + " but was " + FormatActualValue());
        }
        if (expected is null)
        {
            throw new AssertionFailedException("expected " + FormatValue(expected) + " but was " + FormatActualValue());
        }
        let expectedUtf8 = expected.AsUtf8Span();
        if (!Utf8Equals (_actualUtf8, expectedUtf8))
        {
            throw new AssertionFailedException("expected " + FormatValue(expected) + " but was " + FormatActualValue());
        }
        return this;
    }
    public StringAssertionContext IsNotEqualTo(string unexpected) {
        if (_isNull) {
            if (unexpected is null) {
                throw new AssertionFailedException("did not expect null but was null");
            }
            return this;
        }
        if (unexpected is null)
        {
            return this;
        }
        let unexpectedUtf8 = unexpected.AsUtf8Span();
        if (Utf8Equals (_actualUtf8, unexpectedUtf8))
        {
            throw new AssertionFailedException("did not expect " + FormatValue(unexpected) + " but was " + FormatActualValue());
        }
        return this;
    }
    public StringAssertionContext Contains(string substring) {
        if (_isNull) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        if (substring is null) {
            throw new AssertionFailedException("expected non-null substring");
        }
        let needle = substring.AsUtf8Span();
        if (Utf8IndexOf (_actualUtf8, needle) <0)
        {
            throw new AssertionFailedException("expected " + FormatActualValue() + " to contain " + FormatValue(substring));
        }
        return this;
    }
    public StringAssertionContext StartsWith(string prefix) {
        if (_isNull) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        if (prefix is null) {
            throw new AssertionFailedException("expected non-null prefix");
        }
        let needle = prefix.AsUtf8Span();
        if (needle.Length >_actualUtf8.Length)
        {
            throw new AssertionFailedException("expected " + FormatActualValue() + " to start with " + FormatValue(prefix));
        }
        var idx = 0usize;
        while (idx <needle.Length)
        {
            if (_actualUtf8[idx] != needle[idx])
            {
                throw new AssertionFailedException("expected " + FormatActualValue() + " to start with " + FormatValue(prefix));
            }
            idx += 1usize;
        }
        return this;
    }
    public StringAssertionContext EndsWith(string suffix) {
        if (_isNull) {
            throw new AssertionFailedException("expected a non-null value but was null");
        }
        if (suffix is null) {
            throw new AssertionFailedException("expected non-null suffix");
        }
        let needle = suffix.AsUtf8Span();
        if (needle.Length >_actualUtf8.Length)
        {
            throw new AssertionFailedException("expected " + FormatActualValue() + " to end with " + FormatValue(suffix));
        }
        let start = _actualUtf8.Length - needle.Length;
        var idx = 0usize;
        while (idx <needle.Length)
        {
            if (_actualUtf8[start + idx] != needle[idx])
            {
                throw new AssertionFailedException("expected " + FormatActualValue() + " to end with " + FormatValue(suffix));
            }
            idx += 1usize;
        }
        return this;
    }
    private string FormatActualValue() {
        if (_isNull)
        {
            return "null";
        }
        let value = Utf8String.FromSpan(_actualUtf8);
        return "\"" + value + "\"";
    }
    private static string FormatValue(string value) {
        if (value is null) {
            return "null";
        }
        return "\"" + value + "\"";
    }
    @allow(dead_code)
    public static bool operator !(StringAssertionContext _context) => false;
}
testcase Given_assert_string_is_null_When_executed_Then_assert_string_is_null()
{
    let nullString = CoreIntrinsics.DefaultValue <string >();
    let ctx : StringAssertionContext = Assert.That(nullString);
    ctx.IsNull();
}
testcase Given_assert_string_is_not_null_When_executed_Then_assert_string_is_not_null()
{
    let ctx : StringAssertionContext = Assert.That("hi");
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
    Assert.Throws <AssertionFailedException >(FailureActions.StringIsEqualMismatch);
}
testcase Given_assert_string_is_not_equal_to_failure_When_executed_Then_assert_string_is_not_equal_to_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringIsNotEqualMismatch);
}
testcase Given_assert_string_is_not_null_failure_When_executed_Then_assert_string_is_not_null_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringIsNotNullOnNull);
}
testcase Given_assert_string_is_null_failure_When_executed_Then_assert_string_is_null_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringIsNullOnNonNull);
}
testcase Given_assert_string_is_not_equal_to_null_failure_When_executed_Then_assert_string_is_not_equal_to_null_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringIsNotEqualOnNulls);
}
testcase Given_assert_string_contains_When_executed_Then_assert_string_contains()
{
    Assert.That("hello world").Contains("world");
}
testcase Given_assert_string_contains_failure_When_executed_Then_assert_string_contains_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringContainsMissing);
}
testcase Given_assert_string_starts_with_When_executed_Then_assert_string_starts_with()
{
    Assert.That("hello").StartsWith("he");
}
testcase Given_assert_string_starts_with_failure_When_executed_Then_assert_string_starts_with_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringStartsWithMissing);
}
testcase Given_assert_string_ends_with_When_executed_Then_assert_string_ends_with()
{
    Assert.That("hello").EndsWith("lo");
}
testcase Given_assert_string_ends_with_failure_When_executed_Then_assert_string_ends_with_failure()
{
    Assert.Throws <AssertionFailedException >(FailureActions.StringEndsWithMissing);
}
