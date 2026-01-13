namespace Std.Platform.IO;
import Std.Numeric;
import Std.Span;
import Std.Runtime.Collections;
internal static class Platform
{
    public const int FdStdin = 0;
    public const int FdStdout = 1;
    public const int FdStderr = 2;
    @extern("C") private static extern isize read(int fd, * mut @expose_address byte buffer, usize length);
    @extern("C") private static extern isize write(int fd, * const @expose_address byte buffer, usize length);
    @extern("C") private static extern int isatty(int fd);
    public static bool IsTerminal(int fd) {
        return isatty(fd) != 0;
    }
    public static IoError ReadInto(int fd, Span <byte >destination, out usize readCount) {
        var count = 0usize;
        if (destination.Length == 0)
        {
            readCount = 0;
            return IoError.Success;
        }
        if (ValuePointer.IsNullMut (destination.Raw.Data))
        {
            readCount = 0;
            return IoError.InvalidPointer;
        }
        let result = read(fd, destination.Raw.Data.Pointer, destination.Length);
        if (result <0)
        {
            readCount = 0;
            return IoError.Unknown;
        }
        let advanced = Std.Numeric.NumericUnchecked.ToUSize(result);
        count = advanced;
        if (result == 0)
        {
            readCount = count;
            return IoError.Eof;
        }
        readCount = count;
        return IoError.Success;
    }
    public static IoError WriteAll(int fd, ReadOnlySpan <byte >buffer) {
        if (buffer.Length == 0)
        {
            return IoError.Success;
        }
        var remaining = buffer;
        while (remaining.Length >0)
        {
            if (ValuePointer.IsNullConst (remaining.Raw.Data))
            {
                return IoError.InvalidPointer;
            }
            let result = write(fd, remaining.Raw.Data.Pointer, remaining.Length);
            if (result <0)
            {
                return IoError.Unknown;
            }
            if (result == 0)
            {
                return IoError.BrokenPipe;
            }
            let advanced = Std.Numeric.NumericUnchecked.ToUSize(result);
            if (advanced >remaining.Length)
            {
                return IoError.Unknown;
            }
            remaining = remaining.Slice(advanced, remaining.Length - advanced);
        }
        return IoError.Success;
    }
    public static IoError FlushFd(int fd) {
        return IoError.Success;
    }
    public static void ConfigureWasmHooks(isize writeHook, isize flushHook, isize readHook) {
        // no-op in bootstrap
    }
    public static void ConfigureWasmTerminals(bool stdinTerminal, bool stdoutTerminal, bool stderrTerminal) {
        // no-op in bootstrap
    }
}
