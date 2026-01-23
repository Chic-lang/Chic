namespace Std.Platform.IO;
import Std.Span;
import Std.Testing;
testcase Given_platform_read_into_empty_returns_success_When_executed_Then_platform_read_into_empty_returns_success()
{
    var read = 1usize;
    let status = Platform.ReadInto(Platform.FdStdin, Span <byte >.Empty, out read);
    Assert.That(status).IsEqualTo(IoError.Success);
}
testcase Given_platform_read_into_empty_reads_zero_When_executed_Then_platform_read_into_empty_reads_zero()
{
    var read = 1usize;
    let _ = Platform.ReadInto(Platform.FdStdin, Span <byte >.Empty, out read);
    Assert.That(read).IsEqualTo(0usize);
}
testcase Given_platform_write_all_empty_returns_success_When_executed_Then_platform_write_all_empty_returns_success()
{
    let status = Platform.WriteAll(Platform.FdStdout, ReadOnlySpan <byte >.Empty);
    Assert.That(status).IsEqualTo(IoError.Success);
}
