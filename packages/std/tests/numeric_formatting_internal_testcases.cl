namespace Std;
import Std.Core;
import Std.Numeric;
import Std.Span;
import Std.Testing;
testcase Given_numeric_formatting_internal_integer_matrix_When_executed_Then_formats_return_strings()
{
    let invariant = "invariant";
    Assert.That(NumericFormatting.FormatSByte(- 5, "D", invariant)).IsEqualTo("-5");
    Assert.That(NumericFormatting.FormatByte(255u8, "X2", invariant)).IsEqualTo("FF");
    Assert.That(NumericFormatting.FormatInt16(- 42, "D", invariant)).IsEqualTo("-42");
    Assert.That(NumericFormatting.FormatUInt16(65535u16, "X", invariant)).IsEqualTo("FFFF");
    Assert.That(NumericFormatting.FormatInt32(- 1, "X", invariant)).IsEqualTo("FFFFFFFF");
    Assert.That(NumericFormatting.FormatUInt32(0u, "D", invariant)).IsEqualTo("0");
    Assert.That(NumericFormatting.FormatInt64(- 1L, "X", invariant)).IsEqualTo("FFFFFFFFFFFFFFFF");
    Assert.That(NumericFormatting.FormatUInt64(48879ul, "X", invariant)).IsEqualTo("BEEF");
}
testcase Given_numeric_formatting_internal_floating_formats_When_executed_Then_outputs_contain_expected_tokens()
{
    let invariant = "invariant";
    Assert.That(NumericFormatting.FormatFloat64(1e10d, "G", invariant)).Contains("E");
    Assert.That(NumericFormatting.FormatFloat64(12.0d, "e2", invariant)).Contains("e");
    Assert.That(NumericFormatting.FormatFloat64(12.0d, "E2", invariant)).Contains("E");
    Assert.That(NumericFormatting.FormatFloat32(1.25f, "F2", invariant)).Contains(".");
}
testcase Given_numeric_formatting_internal_try_format_buffer_too_small_When_executed_Then_returns_false()
{
    let invariant = "invariant";
    var buffer = Span <byte >.StackAlloc(2);
    let ok = NumericFormatting.TryFormatUInt64(123456789ul, buffer, out var written, "D", invariant);
    Assert.That(ok).IsFalse();
    Assert.That(written).IsEqualTo(0usize);
}
testcase Given_numeric_formatting_internal_unsupported_culture_When_executed_Then_throws_argument_exception()
{
    Assert.Throws <ArgumentException >(() => {
        let _ = NumericFormatting.FormatInt32(1, "G", "zz-ZZ");
    }
    );
}
