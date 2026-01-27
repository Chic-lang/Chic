namespace Std.Platform.IO;
import Std.Core;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Testing;
testcase Given_io_exports_stdin_read_line_rejects_null_pointer_When_executed_Then_io_exports_stdin_read_line_rejects_null_pointer()
{
    unsafe {
        let ptr = Pointer.NullMut <ChicString >();
        let status = IoExports.StdinReadLine(ptr);
        Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.InvalidPointer));
    }
}
testcase Given_io_exports_stdout_write_string_rejects_null_pointer_When_executed_Then_io_exports_stdout_write_string_rejects_null_pointer()
{
    unsafe {
        let ptr = Pointer.NullConst <ChicString >();
        let status = IoExports.StdoutWriteString(ptr);
        Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.InvalidPointer));
    }
}
testcase Given_io_exports_stderr_write_string_rejects_null_pointer_When_executed_Then_io_exports_stderr_write_string_rejects_null_pointer()
{
    unsafe {
        let ptr = Pointer.NullConst <ChicString >();
        let status = IoExports.StderrWriteString(ptr);
        Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.InvalidPointer));
    }
}
testcase Given_io_exports_stdout_write_zero_length_succeeds_When_executed_Then_io_exports_stdout_write_zero_length_succeeds()
{
    let handle = ValuePointer.NullConst(1usize, 1usize);
    let status = IoExports.StdoutWrite(handle, 0usize);
    Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_exports_stderr_write_zero_length_succeeds_When_executed_Then_io_exports_stderr_write_zero_length_succeeds()
{
    let handle = ValuePointer.NullConst(1usize, 1usize);
    let status = IoExports.StderrWrite(handle, 0usize);
    Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_exports_stdin_read_exact_zero_length_succeeds_When_executed_Then_io_exports_stdin_read_exact_zero_length_succeeds()
{
    let handle = ValuePointer.NullMut(1usize, 1usize);
    let status = IoExports.StdinReadExact(handle, 0usize);
    Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_exports_stdin_read_zero_length_succeeds_When_executed_Then_io_exports_stdin_read_zero_length_succeeds()
{
    let handle = ValuePointer.NullMut(1usize, 1usize);
    unsafe {
        let outRead = Pointer.NullMut <usize >();
        let status = IoExports.StdinRead(handle, 0usize, outRead);
        Assert.That(status).IsEqualTo(IoStatus.ToStatus(IoError.Success));
    }
}
testcase Given_io_state_replace_stdin_updates_terminal_flag_When_executed_Then_io_state_replace_stdin_updates_terminal_flag()
{
    var handle = CoreIntrinsics.DefaultValue <StdinHandle >();
    handle.Fd = Platform.FdStdin;
    handle.Terminal = true;
    IoState.ReplaceStdin(handle);
    Assert.That(IoExports.StdinIsTerminal()).IsEqualTo(1);
}
testcase Given_io_state_replace_stdin_updates_terminal_flag_false_When_executed_Then_io_state_replace_stdin_updates_terminal_flag_false()
{
    var handle = CoreIntrinsics.DefaultValue <StdinHandle >();
    handle.Fd = Platform.FdStdin;
    handle.Terminal = false;
    IoState.ReplaceStdin(handle);
    Assert.That(IoExports.StdinIsTerminal()).IsEqualTo(0);
}
testcase Given_io_api_stdin_is_terminal_true_When_executed_Then_io_api_stdin_is_terminal_true()
{
    var handle = CoreIntrinsics.DefaultValue <StdinHandle >();
    handle.Fd = Platform.FdStdin;
    handle.Terminal = true;
    IoState.ReplaceStdin(handle);
    Assert.That(Stdin.IsTerminal()).IsTrue();
}
testcase Given_io_api_stdin_is_terminal_false_When_executed_Then_io_api_stdin_is_terminal_false()
{
    var handle = CoreIntrinsics.DefaultValue <StdinHandle >();
    handle.Fd = Platform.FdStdin;
    handle.Terminal = false;
    IoState.ReplaceStdin(handle);
    Assert.That(Stdin.IsTerminal()).IsFalse();
}
testcase Given_io_exports_stdout_line_buffered_success_When_executed_Then_io_exports_stdout_line_buffered_success()
{
    Assert.That(IoExports.StdoutSetLineBuffered(1)).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_exports_stdout_flush_on_newline_success_When_executed_Then_io_exports_stdout_flush_on_newline_success()
{
    Assert.That(IoExports.StdoutSetFlushOnNewline(0)).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_exports_stdout_normalize_newlines_success_When_executed_Then_io_exports_stdout_normalize_newlines_success()
{
    Assert.That(IoExports.StdoutSetNormalizeNewlines(1)).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_exports_stderr_flags_return_success_When_executed_Then_io_exports_stderr_flags_return_success()
{
    Assert.That(IoExports.StderrSetNormalizeNewlines(0)).IsEqualTo(IoStatus.ToStatus(IoError.Success));
}
testcase Given_io_api_stdout_write_empty_succeeds_When_executed_Then_io_api_stdout_write_empty_succeeds()
{
    let status = Stdout.Write("");
    Assert.That(status).IsEqualTo(IoError.Success);
}
testcase Given_io_api_stderr_write_empty_succeeds_When_executed_Then_io_api_stderr_write_empty_succeeds()
{
    let status = Stderr.Write("");
    Assert.That(status).IsEqualTo(IoError.Success);
}
