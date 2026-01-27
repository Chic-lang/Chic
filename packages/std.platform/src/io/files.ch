namespace Std.Platform.IO;
import Std.Memory;
import Std.Span;
import Std.Strings;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Core;
@repr(c) internal struct FileHandle
{
    public * mut @expose_address byte Ptr;
}
internal static class FileNative
{
    @extern("C") internal static extern * mut @expose_address byte fopen(* const @expose_address byte path, * const @expose_address byte mode);
    @extern("C") internal static extern usize fread(* mut @expose_address byte buffer, usize size, usize count, * mut @expose_address byte stream);
    @extern("C") internal static extern usize fwrite(* const @expose_address byte buffer, usize size, usize count, * mut @expose_address byte stream);
    @extern("C") internal static extern int fflush(* mut @expose_address byte stream);
    @extern("C") internal static extern int fclose(* mut @expose_address byte stream);
    @extern("C") internal static extern int fseek(* mut @expose_address byte stream, isize offset, int origin);
    @extern("C") internal static extern isize ftell(* mut @expose_address byte stream);
}
public struct File
{
    internal FileHandle Handle;
    public static File OpenRead(string path, out IoError err) {
        return Open(path, "r", out err);
    }
    public static File OpenWrite(string path, bool append, out IoError err) {
        return Open(path, append ?"a" : "w", out err);
    }
    public static File Open(string path, string mode, out IoError err) {
        var outErr = IoError.Unknown;
        var pathText = path;
        var modeText = mode;
        let pathUtf8 = IoTyped.FromStringSlice(SpanIntrinsics.chic_rt_string_as_slice(pathText));
        let modeUtf8 = IoTyped.FromStringSlice(SpanIntrinsics.chic_rt_string_as_slice(modeText));
        var pathBuf = Span <byte >.StackAlloc(pathUtf8.Length + 1);
        var modeBuf = Span <byte >.StackAlloc(modeUtf8.Length + 1);
        pathBuf.Slice(0, pathUtf8.Length).CopyFrom(pathUtf8);
        modeBuf.Slice(0, modeUtf8.Length).CopyFrom(modeUtf8);
        let pathPtr = PointerIntrinsics.AsByteConstFromMut(pathBuf.Raw.Data.Pointer);
        let modePtr = PointerIntrinsics.AsByteConstFromMut(modeBuf.Raw.Data.Pointer);
        let handle = FileNative.fopen(pathPtr, modePtr);
        let handleWrapper = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(handle), 0usize, 0usize);
        var file = CoreIntrinsics.DefaultValue <File >();
        file.Handle.Ptr = handle;
        if (! ValuePointer.IsNullMut (handleWrapper))
        {
            outErr = IoError.Success;
        }
        err = outErr;
        return file;
    }
    public bool IsValid {
        get {
            let handle = ValuePointer.CreateMut(PointerIntrinsics.AsByteMut(Handle.Ptr), 0usize, 0usize);
            return ! ValuePointer.IsNullMut(handle);
        }
    }
    public bool Read(Span <byte >destination, out usize read, out IoError err) {
        if (! IsValid)
        {
            read = 0;
            err = IoError.InvalidPointer;
            return false;
        }
        if (destination.Length == 0)
        {
            read = 0;
            err = IoError.Success;
            return true;
        }
        var count = FileNative.fread(destination.Raw.Data.Pointer, 1, destination.Length, Handle.Ptr);
        read = count;
        err = count == 0 ?IoError.Eof : IoError.Success;
        return err == IoError.Success;
    }
    public IoError Write(ReadOnlySpan <byte >source) {
        if (! IsValid)
        {
            return IoError.InvalidPointer;
        }
        if (source.Length == 0)
        {
            return IoError.Success;
        }
        var written = FileNative.fwrite(source.Raw.Data.Pointer, 1, source.Length, Handle.Ptr);
        if (written != source.Length)
        {
            return IoError.Unknown;
        }
        return IoError.Success;
    }
    public IoError Flush() {
        if (! IsValid)
        {
            return IoError.InvalidPointer;
        }
        var status = FileNative.fflush(Handle.Ptr);
        return status == 0 ?IoError.Success : IoError.Unknown;
    }
    public IoError Close(out IoError status) {
        if (! IsValid)
        {
            status = IoError.InvalidPointer;
            return IoError.InvalidPointer;
        }
        var code = FileNative.fclose(Handle.Ptr);
        let nullHandle = Std.Runtime.Collections.ValuePointer.NullMut(0usize, 0usize);
        Handle.Ptr = nullHandle.Pointer;
        let result = code == 0 ?IoError.Success : IoError.Unknown;
        status = result;
        return result;
    }
    public bool Seek(isize offset, int origin, out IoError status) {
        if (! IsValid)
        {
            status = IoError.InvalidPointer;
            return false;
        }
        let code = FileNative.fseek(Handle.Ptr, offset, origin);
        status = code == 0 ?IoError.Success : IoError.Unknown;
        return status == IoError.Success;
    }
    public bool Tell(out isize position, out IoError status) {
        if (! IsValid)
        {
            position = 0;
            status = IoError.InvalidPointer;
            return false;
        }
        let pos = FileNative.ftell(Handle.Ptr);
        position = pos;
        status = pos >= 0 ?IoError.Success : IoError.Unknown;
        return status == IoError.Success;
    }
}
