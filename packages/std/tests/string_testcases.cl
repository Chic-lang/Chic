namespace Std;
import Std.Testing;
testcase Given_string_is_null_or_empty_null_When_executed_Then_string_is_null_or_empty_null()
{
    Assert.That(String.IsNullOrEmpty(null)).IsTrue();
}
testcase Given_string_is_null_or_empty_empty_When_executed_Then_string_is_null_or_empty_empty()
{
    Assert.That(String.IsNullOrEmpty("")).IsTrue();
}
testcase Given_string_is_null_or_empty_non_empty_When_executed_Then_string_is_null_or_empty_non_empty()
{
    Assert.That(String.IsNullOrEmpty("hi")).IsFalse();
}
testcase Given_string_index_of_char_found_When_executed_Then_string_index_of_char_found()
{
    Assert.That("abc".IndexOf('b')).IsEqualTo(1);
}
testcase Given_string_index_of_char_missing_When_executed_Then_string_index_of_char_missing()
{
    Assert.That("abc".IndexOf('z')).IsEqualTo(- 1);
}
testcase Given_string_index_of_string_found_When_executed_Then_string_index_of_string_found()
{
    Assert.That("hello".IndexOf("ell")).IsEqualTo(1);
}
testcase Given_string_index_of_string_empty_returns_start_When_executed_Then_string_index_of_string_empty_returns_start()
{
    Assert.That("hello".IndexOf("", 2)).IsEqualTo(2);
}
testcase Given_string_substring_start_length_When_executed_Then_string_substring_start_length()
{
    Assert.That("hello".Substring(1, 3)).IsEqualTo("ell");
}
testcase Given_string_starts_with_true_When_executed_Then_string_starts_with_true()
{
    Assert.That("hello".StartsWith("he")).IsTrue();
}
testcase Given_string_to_boolean_true_When_executed_Then_string_to_boolean_true()
{
    Assert.That("true".ToBoolean(null)).IsTrue();
}
testcase Given_string_index_of_negative_start_throws_When_executed_Then_string_index_of_negative_start_throws()
{
    Assert.Throws <ArgumentOutOfRangeException >(() => {
        let _ = "abc".IndexOf('a', - 1);
    }
    );
}
