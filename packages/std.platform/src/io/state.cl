namespace Std.Platform.IO;
import Std.Sync;
internal static class IoState
{
    private static Mutex <StdinHandle >_stdin;
    private static Mutex <StdoutHandle >_stdout;
    private static Mutex <StderrHandle >_stderr;
    private static bool _initialized;
    private static void EnsureInit() {
        if (_initialized)
        {
            return;
        }
        _stdin = new Mutex <StdinHandle >(StdinHandle.System());
        _stdout = new Mutex <StdoutHandle >(StdoutHandle.System());
        _stderr = new Mutex <StderrHandle >(StderrHandle.System());
        _initialized = true;
    }
    public static ref StdinHandle BorrowStdin(ref MutexGuard <StdinHandle >guard) {
        EnsureInit();
        return guard.Value;
    }
    public static ref StdoutHandle BorrowStdout(ref MutexGuard <StdoutHandle >guard) {
        EnsureInit();
        return guard.Value;
    }
    public static ref StderrHandle BorrowStderr(ref MutexGuard <StderrHandle >guard) {
        EnsureInit();
        return guard.Value;
    }
    public static MutexGuard <StdinHandle >LockStdin() {
        EnsureInit();
        return _stdin.Lock();
    }
    public static MutexGuard <StdoutHandle >LockStdout() {
        EnsureInit();
        return _stdout.Lock();
    }
    public static MutexGuard <StderrHandle >LockStderr() {
        EnsureInit();
        return _stderr.Lock();
    }
    public static void ReplaceStdin(StdinHandle handle) {
        EnsureInit();
        var guard = _stdin.Lock();
        guard.ReplaceValue(handle);
        guard.Release();
    }
}
