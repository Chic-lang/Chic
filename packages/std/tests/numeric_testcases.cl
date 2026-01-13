namespace Std;
import Std.Span;
import Std.Testing;
testcase Given_int32_parse_string_value_When_executed_Then_int32_parse_string_value()
{
    Assert.That(Int32.Parse("42").ToInt32()).IsEqualTo(42);
}
testcase Given_int32_parse_null_throws_When_executed_Then_int32_parse_null_throws()
{
    Assert.Throws <ArgumentNullException >(() => {
        let _ = Int32.Parse(null);
    }
    );
}
testcase Given_int32_tryparse_valid_true_When_executed_Then_int32_tryparse_valid_true()
{
    let ok = Int32.TryParse("42", out var result);
    Assert.That(ok).IsTrue();
}
testcase Given_int32_tryparse_invalid_false_When_executed_Then_int32_tryparse_invalid_false()
{
    let ok = Int32.TryParse("nope", out var result);
    Assert.That(ok).IsFalse();
}
testcase Given_int32_tryparse_invalid_default_When_executed_Then_int32_tryparse_invalid_default()
{
    let _ = Int32.TryParse("nope", out var result);
    Assert.That(result.ToInt32()).IsEqualTo(0);
}
testcase Given_int32_tryparse_utf8_true_When_executed_Then_int32_tryparse_utf8_true()
{
    let span = ReadOnlySpan.FromString("5");
    let ok = Int32.TryParse(span, out var result);
    Assert.That(ok).IsTrue();
}
testcase Given_uint32_parse_string_value_When_executed_Then_uint32_parse_string_value()
{
    Assert.That(UInt32.Parse("7").ToUInt32()).IsEqualTo(7u);
}
testcase Given_uint32_tryparse_valid_true_When_executed_Then_uint32_tryparse_valid_true()
{
    let ok = UInt32.TryParse("7", out var result);
    Assert.That(ok).IsTrue();
}
testcase Given_uint32_tryparse_invalid_false_When_executed_Then_uint32_tryparse_invalid_false()
{
    let ok = UInt32.TryParse("nope", out var result);
    Assert.That(ok).IsFalse();
}
testcase Given_uint32_tryparse_invalid_default_When_executed_Then_uint32_tryparse_invalid_default()
{
    let _ = UInt32.TryParse("nope", out var result);
    Assert.That(result.ToUInt32()).IsEqualTo(0u);
}
testcase Given_uint32_tryparse_utf8_true_When_executed_Then_uint32_tryparse_utf8_true()
{
    let span = ReadOnlySpan.FromString("7");
    let ok = UInt32.TryParse(span, out var result);
    Assert.That(ok).IsTrue();
}
