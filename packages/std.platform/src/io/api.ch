@allow(dead_code) namespace Std.Platform.IO;
import Std.Async;
import Std.Runtime.Native;
import Std.Span;
public static class Stdin
{
    public static string ReadLine() {
        var line = "";
        var status = IoError.Unknown;
        TryReadLine(out line, out status);
        return status == IoError.Success ?line : "";
    }
    public static bool TryReadLine(out string destination, out IoError ioError) {
        var guard = IoState.LockStdin();
        var native = StringIntrinsics.chic_rt_string_new();
        var status = IoState.BorrowStdin(ref guard).ReadLine(ref native);
        guard.Release();
        ioError = status;
        if (status != IoError.Success)
        {
            destination = "";
            return false;
        }
        let runtimeSlice = StringIntrinsics.chic_rt_string_as_slice(ref native);
        let slice = IoTyped.FromRuntimeStr(runtimeSlice);
        destination = SpanIntrinsics.chic_rt_string_from_slice(slice);
        return true;
    }
    public static string ReadLineAsync() {
        return ReadLine();
    }
    public static bool IsTerminal() {
        var guard = IoState.LockStdin();
        let result = IoState.BorrowStdin(ref guard).Terminal;
        guard.Release();
        return result;
    }
}
public static class Stdout
{
    public static IoError WriteAsync(string value) {
        return Write(value);
    }
    public static IoError WriteLineAsync() {
        return WriteLine();
    }
    public static IoError WriteLineAsync(string value) {
        return WriteLine(value);
    }
    public static IoError Write(string value) {
        var text = value;
        var guard = IoState.LockStdout();
        let status = IoState.BorrowStdout(ref guard).WriteString(ref text);
        guard.Release();
        return status;
    }
    public static IoError WriteLine() {
        var guard = IoState.LockStdout();
        let status = IoState.BorrowStdout(ref guard).WriteLine();
        guard.Release();
        return status;
    }
    public static IoError WriteLine(string value) {
        var text = value;
        var guard = IoState.LockStdout();
        let status = IoState.BorrowStdout(ref guard).WriteLineString(ref text);
        guard.Release();
        return status;
    }
    public static IoError Flush() {
        var guard = IoState.LockStdout();
        let status = IoState.BorrowStdout(ref guard).Flush();
        guard.Release();
        return status;
    }
    public static IoError SetLineBuffered(bool enabled) {
        var guard = IoState.LockStdout();
        IoState.BorrowStdout(ref guard).SetLineBuffered(enabled);
        guard.Release();
        return IoError.Success;
    }
    public static IoError SetFlushOnNewline(bool enabled) {
        var guard = IoState.LockStdout();
        IoState.BorrowStdout(ref guard).SetFlushOnNewline(enabled);
        guard.Release();
        return IoError.Success;
    }
    public static IoError SetNormalizeNewlines(bool enabled) {
        var guard = IoState.LockStdout();
        IoState.BorrowStdout(ref guard).SetNormalizeNewlines(enabled);
        guard.Release();
        return IoError.Success;
    }
    public static bool IsTerminal() {
        var guard = IoState.LockStdout();
        let result = IoState.BorrowStdout(ref guard).IsTerminal();
        guard.Release();
        return result;
    }
}
public static class Stderr
{
    public static IoError WriteAsync(string value) {
        return Write(value);
    }
    public static IoError WriteLineAsync() {
        return WriteLine();
    }
    public static IoError WriteLineAsync(string value) {
        return WriteLine(value);
    }
    public static IoError Write(string value) {
        var text = value;
        var guard = IoState.LockStderr();
        let status = IoState.BorrowStderr(ref guard).WriteString(ref text);
        guard.Release();
        return status;
    }
    public static IoError WriteLine() {
        var guard = IoState.LockStderr();
        let status = IoState.BorrowStderr(ref guard).WriteLine();
        guard.Release();
        return status;
    }
    public static IoError WriteLine(string value) {
        var text = value;
        var guard = IoState.LockStderr();
        let status = IoState.BorrowStderr(ref guard).WriteLineString(ref text);
        guard.Release();
        return status;
    }
    public static IoError Flush() {
        var guard = IoState.LockStderr();
        let status = IoState.BorrowStderr(ref guard).Flush();
        guard.Release();
        return status;
    }
    public static IoError SetNormalizeNewlines(bool enabled) {
        var guard = IoState.LockStderr();
        IoState.BorrowStderr(ref guard).SetNormalizeNewlines(enabled);
        guard.Release();
        return IoError.Success;
    }
    public static bool IsTerminal() {
        var guard = IoState.LockStderr();
        let result = IoState.BorrowStderr(ref guard).IsTerminal();
        guard.Release();
        return result;
    }
}
public static class Console
{
    public static string ?ReadLine() {
        return Std.Console.ReadLine();
    }
    public static void Write(string value) {
        Std.Console.Write(value);
    }
    public static void WriteLine() {
        Std.Console.WriteLine();
    }
    public static void WriteLine(string value) {
        Std.Console.WriteLine(value);
    }
    public static void Flush() {
        Std.Console.Out.Flush();
    }
}
