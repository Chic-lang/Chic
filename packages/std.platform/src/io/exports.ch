namespace Std.Platform.IO;
import Std.Runtime.Collections;
import Std.Runtime.Native;
import Std.Span;
internal static class IoExports
{
    public static int StdinReadLine(* mut @expose_address ChicString destination) {
        unsafe {
            if (Std.Numeric.Pointer.IsNull (destination))
            {
                return IoStatus.ToStatus(IoError.InvalidPointer);
            }
            var dst = destination;
            var guard = IoState.LockStdin();
            var status = IoState.BorrowStdin(ref guard).ReadLine(ref * dst);
            guard.Release();
            return IoStatus.ToStatus(status);
        }
    }
    public static int StdinRead(ValueMutPtr buffer, usize length, * mut usize outRead) {
        var destination = IoTyped.MutableBytes(buffer, length);
        var guard = IoState.LockStdin();
        var readCount = 0usize;
        var status = IoState.BorrowStdin(ref guard).Read(destination, out readCount);
        guard.Release();
        unsafe {
            if (! Std.Numeric.Pointer.IsNull (outRead))
            {
                * outRead = readCount;
            }
        }
        return IoStatus.ToStatus(status);
    }
    public static int StdinReadExact(ValueMutPtr buffer, usize length) {
        var destination = IoTyped.MutableBytes(buffer, length);
        var guard = IoState.LockStdin();
        var status = IoState.BorrowStdin(ref guard).ReadExact(destination);
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StdinIsTerminal() {
        var guard = IoState.LockStdin();
        var result = IoState.BorrowStdin(ref guard).Terminal ?1 : 0;
        guard.Release();
        return result;
    }
    public static int StdoutWrite(ValueConstPtr slice, usize length) {
        let bytes = IoTyped.ReadOnlyBytes(slice, length);
        var guard = IoState.LockStdout();
        var status = IoState.BorrowStdout(ref guard).WriteBytes(bytes, false);
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StdoutWriteLine(ValueConstPtr slice, usize length) {
        let bytes = IoTyped.ReadOnlyBytes(slice, length);
        var guard = IoState.LockStdout();
        var status = IoState.BorrowStdout(ref guard).WriteBytes(bytes, true);
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StdoutWriteString(* const @expose_address ChicString value) {
        unsafe {
            if (Std.Numeric.Pointer.IsNullConst (value))
            {
                return IoStatus.ToStatus(IoError.InvalidPointer);
            }
            var local = * value;
            let slice = StringIntrinsics.chic_rt_string_as_slice(ref local);
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.ptr), 1, 1);
            let bytes = IoTyped.ReadOnlyBytes(handle, slice.len);
            var guard = IoState.LockStdout();
            var status = IoState.BorrowStdout(ref guard).WriteBytes(bytes, false);
            guard.Release();
            return IoStatus.ToStatus(status);
        }
    }
    public static int StdoutWriteLineString(* const @expose_address ChicString value) {
        unsafe {
            if (Std.Numeric.Pointer.IsNullConst (value))
            {
                return IoStatus.ToStatus(IoError.InvalidPointer);
            }
            var local = * value;
            let slice = StringIntrinsics.chic_rt_string_as_slice(ref local);
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.ptr), 1, 1);
            let bytes = IoTyped.ReadOnlyBytes(handle, slice.len);
            var guard = IoState.LockStdout();
            var status = IoState.BorrowStdout(ref guard).WriteBytes(bytes, true);
            guard.Release();
            return IoStatus.ToStatus(status);
        }
    }
    public static int StdoutFlush() {
        var guard = IoState.LockStdout();
        var status = IoState.BorrowStdout(ref guard).Flush();
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StdoutSetLineBuffered(int enabled) {
        var guard = IoState.LockStdout();
        IoState.BorrowStdout(ref guard).SetLineBuffered(enabled != 0);
        guard.Release();
        return IoStatus.ToStatus(IoError.Success);
    }
    public static int StdoutSetFlushOnNewline(int enabled) {
        var guard = IoState.LockStdout();
        IoState.BorrowStdout(ref guard).SetFlushOnNewline(enabled != 0);
        guard.Release();
        return IoStatus.ToStatus(IoError.Success);
    }
    public static int StdoutSetNormalizeNewlines(int enabled) {
        var guard = IoState.LockStdout();
        IoState.BorrowStdout(ref guard).SetNormalizeNewlines(enabled != 0);
        guard.Release();
        return IoStatus.ToStatus(IoError.Success);
    }
    public static int StdoutIsTerminal() {
        var guard = IoState.LockStdout();
        var result = IoState.BorrowStdout(ref guard).IsTerminal() ?1 : 0;
        guard.Release();
        return result;
    }
    public static int StderrWrite(ValueConstPtr slice, usize length) {
        let bytes = IoTyped.ReadOnlyBytes(slice, length);
        var guard = IoState.LockStderr();
        var status = IoState.BorrowStderr(ref guard).WriteBytes(bytes);
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StderrWriteLine(ValueConstPtr slice, usize length) {
        let bytes = IoTyped.ReadOnlyBytes(slice, length);
        var guard = IoState.LockStderr();
        var status = IoState.BorrowStderr(ref guard).WriteLineBytes(bytes);
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StderrWriteString(* const @expose_address ChicString value) {
        unsafe {
            if (Std.Numeric.Pointer.IsNullConst (value))
            {
                return IoStatus.ToStatus(IoError.InvalidPointer);
            }
            var local = * value;
            let slice = StringIntrinsics.chic_rt_string_as_slice(ref local);
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.ptr), 1, 1);
            let bytes = IoTyped.ReadOnlyBytes(handle, slice.len);
            var guard = IoState.LockStderr();
            var status = IoState.BorrowStderr(ref guard).WriteBytes(bytes);
            guard.Release();
            return IoStatus.ToStatus(status);
        }
    }
    public static int StderrWriteLineString(* const @expose_address ChicString value) {
        unsafe {
            if (Std.Numeric.Pointer.IsNullConst (value))
            {
                return IoStatus.ToStatus(IoError.InvalidPointer);
            }
            var local = * value;
            let slice = StringIntrinsics.chic_rt_string_as_slice(ref local);
            let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.ptr), 1, 1);
            let bytes = IoTyped.ReadOnlyBytes(handle, slice.len);
            var guard = IoState.LockStderr();
            var status = IoState.BorrowStderr(ref guard).WriteLineBytes(bytes);
            guard.Release();
            return IoStatus.ToStatus(status);
        }
    }
    public static int StderrFlush() {
        var guard = IoState.LockStderr();
        var status = IoState.BorrowStderr(ref guard).Flush();
        guard.Release();
        return IoStatus.ToStatus(status);
    }
    public static int StderrSetNormalizeNewlines(int enabled) {
        var guard = IoState.LockStderr();
        IoState.BorrowStderr(ref guard).SetNormalizeNewlines(enabled != 0);
        guard.Release();
        return IoStatus.ToStatus(IoError.Success);
    }
    public static int StderrIsTerminal() {
        var guard = IoState.LockStderr();
        var result = IoState.BorrowStderr(ref guard).IsTerminal() ?1 : 0;
        guard.Release();
        return result;
    }
    public static void WasmIoRegister(isize writeHook, isize flushHook, isize readHook) {
        Platform.ConfigureWasmHooks(writeHook, flushHook, readHook);
    }
    public static void WasmIoSetTerminals(int stdin, int stdout, int stderr) {
        Platform.ConfigureWasmTerminals(stdin != 0, stdout != 0, stderr != 0);
    }
}
