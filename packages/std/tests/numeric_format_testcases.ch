namespace Std;
import Std.Core;
import Std.Numeric;
import Std.Testing;
import Std.Span;
testcase Given_int32_try_format_hex_with_small_buffer_When_executed_Then_returns_false()
{
    let value = new Int32(255);
    var buffer = Span <byte >.StackAlloc(1);
    let ok = value.TryFormat(buffer, out var written, "X2");
    Assert.That(ok).IsFalse();
    Assert.That(written).IsEqualTo(0usize);
}
testcase Given_int32_try_format_hex_upper_and_lower_When_executed_Then_expected_output()
{
    let value = new Int32(48879);
    // 0xBEEF
    var buffer = Span <byte >.StackAlloc(4);
    let okUpper = value.TryFormat(buffer, out var writtenUpper, "X");
    Assert.That(okUpper).IsTrue();
    Assert.That(writtenUpper).IsEqualTo(4usize);
    Assert.That(Utf8String.FromSpan(buffer.Slice(0, writtenUpper).AsReadOnly())).IsEqualTo("BEEF");
    let okLower = value.TryFormat(buffer, out var writtenLower, "x");
    Assert.That(okLower).IsTrue();
    Assert.That(writtenLower).IsEqualTo(4usize);
    Assert.That(Utf8String.FromSpan(buffer.Slice(0, writtenLower).AsReadOnly())).IsEqualTo("beef");
}
testcase Given_int32_parse_with_whitespace_and_sign_When_executed_Then_parses()
{
    let parsed = Int32.Parse("  -42  ");
    Assert.That(parsed.ToInt32()).IsEqualTo(- 42);
}
testcase Given_int32_parse_invalid_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = Int32.Parse("12x");
    }
    );
}
testcase Given_int32_try_parse_invalid_When_executed_Then_returns_false()
{
    let ok = Int32.TryParse("++1", out var value);
    Assert.That(ok).IsFalse();
    Assert.That(value.ToInt32()).IsEqualTo(0);
}
testcase Given_int32_format_decimal_precision_When_executed_Then_zero_pads()
{
    Assert.That(new Int32(42).ToString("D5")).IsEqualTo("00042");
    Assert.That(new Int32(- 42).ToString("D5")).IsEqualTo("-00042");
}
testcase Given_int32_format_negative_hex_default_width_When_executed_Then_uses_twos_complement_width()
{
    Assert.That(new Int32(- 1).ToString("X")).IsEqualTo("FFFFFFFF");
    Assert.That(new Int32(- 1).ToString("x")).IsEqualTo("ffffffff");
}
testcase Given_int32_format_grouping_invariant_When_executed_Then_group_separator_inserted()
{
    Assert.That(new Int32(1234567).ToString("N0", "invariant")).IsEqualTo("1,234,567");
    Assert.That(new Int32(- 1234).ToString("N0", "invariant")).IsEqualTo("-1,234");
}
testcase Given_int32_format_grouping_french_When_executed_Then_uses_space_separator()
{
    Assert.That(new Int32(1234).ToString("N0", "fr-FR")).IsEqualTo("1 234");
    Assert.That(new Int32(1234).ToString("N2", "fr-FR")).IsEqualTo("1 234,00");
}
testcase Given_int32_format_invalid_format_specifier_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = new Int32(1).ToString("Q");
    }
    );
}
testcase Given_int32_format_invalid_precision_string_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = new Int32(1).ToString("D-1");
    }
    );
}
testcase Given_int32_format_unsupported_culture_When_executed_Then_throws_argument_exception()
{
    Assert.Throws <ArgumentException >(() => {
        let _ = new Int32(1).ToString("G", "zz-ZZ");
    }
    );
}
testcase Given_int32_parse_with_underscores_When_executed_Then_parses()
{
    Assert.That(Int32.Parse("1_000").ToInt32()).IsEqualTo(1000);
    Assert.That(Int32.Parse("-2_147_483_648").ToInt32()).IsEqualTo(Int32.MinValue);
}
testcase Given_int32_parse_invalid_underscore_patterns_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = Int32.Parse("_1");
    }
    );
    Assert.Throws <FormatException >(() => {
        let _ = Int32.Parse("1__0");
    }
    );
    Assert.Throws <FormatException >(() => {
        let _ = Int32.Parse("1_");
    }
    );
}
testcase Given_int32_parse_overflow_When_executed_Then_throws_overflow_exception()
{
    Assert.Throws <OverflowException >(() => {
        let _ = Int32.Parse("2147483648");
    }
    );
}
testcase Given_int64_parse_min_value_When_executed_Then_parses()
{
    Assert.That(Int64.Parse("-9223372036854775808").ToInt64()).IsEqualTo(Int64.MinValue);
}
testcase Given_uint64_parse_overflow_When_executed_Then_throws_overflow_exception()
{
    Assert.Throws <OverflowException >(() => {
        let _ = UInt64.Parse("18446744073709551616");
    }
    );
}
testcase Given_decimal_format_fixed_and_number_When_executed_Then_expected_strings()
{
    let value = 12.5m;
    Assert.That(value.ToString("F2", "invariant")).IsEqualTo("12.50");
    Assert.That(value.ToString("N1", "invariant")).IsEqualTo("12.5");
    Assert.That((- 12.5m).ToString("F0", "invariant")).IsEqualTo("-12");
}
testcase Given_decimal_format_invalid_specifier_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = 1.0m.ToString("X", "invariant");
    }
    );
}
testcase Given_float64_format_special_values_When_executed_Then_outputs_tokens()
{
    let nan = Float64.NaN;
    let posInf = Float64.PositiveInfinity;
    let negInf = Float64.NegativeInfinity;
    Assert.That(nan.ToString("G", "invariant")).IsEqualTo("NaN");
    Assert.That(posInf.ToString("G", "invariant")).IsEqualTo("Infinity");
    Assert.That(negInf.ToString("G", "invariant")).IsEqualTo("-Infinity");
}
testcase Given_float64_format_fixed_general_and_exponent_When_executed_Then_outputs_are_reasonable()
{
    let value = new Float64(12345.6789d);
    Assert.That(value.ToString("F2", "invariant")).Contains(".");
    Assert.That(value.ToString("N2", "invariant")).Contains(",");
    let large = new Float64(10000000000.0d);
    Assert.That(large.ToString("G", "invariant")).Contains("E");
    let small = new Float64(0.00001d);
    Assert.That(small.ToString("G", "invariant")).Contains("E");
    let expLower = new Float64(12.0d);
    Assert.That(expLower.ToString("e2", "invariant")).Contains("e");
}
testcase Given_float64_format_invalid_specifier_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = new Float64(1.0d).ToString("Q", "invariant");
    }
    );
}
testcase Given_float64_format_fraction_precision_over_limit_When_executed_Then_throws_format_exception()
{
    Assert.Throws <FormatException >(() => {
        let _ = new Float64(1.0d).ToString("F10", "invariant");
    }
    );
}
testcase Given_float64_try_format_special_value_with_small_buffer_When_executed_Then_returns_false()
{
    let posInf = Float64.PositiveInfinity;
    var buffer = Span <byte >.StackAlloc(3);
    let ok = posInf.TryFormat(buffer, out var written, "G", "invariant");
    Assert.That(ok).IsFalse();
    Assert.That(written).IsEqualTo(0usize);
}
testcase Given_uint64_format_hex_and_decimal_When_executed_Then_expected_values_returned()
{
    let value = new UInt64(48879ul);
    Assert.That(value.ToString("X", "invariant")).IsEqualTo("BEEF");
    Assert.That(value.ToString("D", "invariant")).IsEqualTo("48879");
}
