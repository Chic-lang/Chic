namespace Std;
import Std.Core;
import Std.Numeric;
import Std.Span;
import Std.Testing;
testcase Given_numeric_formatting_matrix_When_executed_Then_common_formats_succeed()
{
    let invariant = "invariant";
    let fr = "fr-FR";
    let i32 = new Int32(- 12345);
    Assert.That(i32.ToString("G", invariant).Length >0).IsTrue();
    Assert.That(i32.ToString("D5", invariant)).Contains("12345");
    Assert.That(i32.ToString("N0", invariant)).Contains(",");
    Assert.That(new Int32(- 1).ToString("X", invariant)).IsEqualTo("FFFFFFFF");
    let u32 = new UInt32(12345u);
    Assert.That(u32.ToString("D", invariant)).IsEqualTo("12345");
    Assert.That(u32.ToString("X", invariant)).IsEqualTo("3039");
    let i64 = new Int64(- 1234567890L);
    Assert.That(i64.ToString("G", invariant)).Contains("1234567890");
    Assert.That(i64.ToString("N0", fr)).Contains(" ");
    let u64 = new UInt64(18446744073709551615ul);
    Assert.That(u64.ToString("X", invariant)).IsEqualTo("FFFFFFFFFFFFFFFF");
    let d = new Float64(12345.6789d);
    Assert.That(d.ToString("F2", invariant)).Contains(".");
    Assert.That(d.ToString("N2", invariant)).Contains(",");
    Assert.That(new Float64(1e10d).ToString("G", invariant)).Contains("E");
    Assert.That(new Float64(1e-6d).ToString("G", invariant)).Contains("E");
    Assert.That(new Float64(12.0d).ToString("e2", invariant)).Contains("e");
    let f = new Float32(123.25f);
    Assert.That(f.ToString("F1", invariant)).Contains(".");
    let dec = 1234.5m;
    Assert.That(dec.ToString("F2", invariant)).IsEqualTo("1234.50");
    Assert.That(dec.ToString("N1", fr)).Contains(",");
    var buffer = Span <byte >.StackAlloc(64);
    let ok = d.TryFormat(buffer, out var written, "E2", invariant);
    Assert.That(ok).IsTrue();
    Assert.That(written >0usize).IsTrue();
}
