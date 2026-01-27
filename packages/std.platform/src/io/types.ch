namespace Std.Platform.IO;
import Std.Runtime.Collections;
import Std.Runtime.Native;
import Std.Span;
import Std.Numeric;
import Std.Core;
public enum IoError
{
    Success = 0, WouldBlock = 1, BrokenPipe = 2, InvalidData = 3, UnexpectedEof = 4, PermissionDenied = 5, Eof = 6, InvalidPointer = 7, Aborted = 8, RuntimePanic = 9, Unsupported = 10, Unknown = 255,
}
@repr(c) public struct StringInlineBytes32
{
    public byte b00;
    public byte b01;
    public byte b02;
    public byte b03;
    public byte b04;
    public byte b05;
    public byte b06;
    public byte b07;
    public byte b08;
    public byte b09;
    public byte b10;
    public byte b11;
    public byte b12;
    public byte b13;
    public byte b14;
    public byte b15;
    public byte b16;
    public byte b17;
    public byte b18;
    public byte b19;
    public byte b20;
    public byte b21;
    public byte b22;
    public byte b23;
    public byte b24;
    public byte b25;
    public byte b26;
    public byte b27;
    public byte b28;
    public byte b29;
    public byte b30;
    public byte b31;
}
@repr(c) public struct ChicString
{
    public * mut @expose_address byte ptr;
    public usize len;
    public usize cap;
    public StringInlineBytes32 inline_data;
}
@repr(c) public struct ChicStr
{
    public * const @readonly @expose_address byte ptr;
    public usize len;
}
internal static class IoStatus
{
    public static int ToStatus(IoError ioError) {
        switch (ioError)
        {
            case IoError.Success:
                return 0;
            case IoError.WouldBlock:
                return 1;
            case IoError.BrokenPipe:
                return 2;
            case IoError.InvalidData:
                return 3;
            case IoError.UnexpectedEof:
                return 4;
            case IoError.PermissionDenied:
                return 5;
            case IoError.Eof:
                return 6;
            case IoError.InvalidPointer:
                return 7;
            case IoError.Aborted:
                return 8;
            case IoError.RuntimePanic:
                return 9;
            case IoError.Unsupported:
                return 10;
            default :
                return 255;
            }
        }
        }
        internal static class StringIntrinsics
        {
            @extern("C") public static extern ChicString chic_rt_string_new();
            @extern("C") public static extern ChicString chic_rt_string_from_slice(ChicStr slice);
            @extern("C") public static extern ChicStr chic_rt_string_as_slice(ref ChicString value);
            @extern("C") public static extern int chic_rt_string_clone_slice(ref ChicString dest, ChicStr src);
        }
        internal static class IoTyped
        {
            private const string BYTE_HANDLE_MESSAGE = "IO expects Value{Const,Mut}Ptr handles with byte-sized, byte-aligned elements";
            public static ChicStr ToRuntimeStr(StrPtr slice) {
                var runtime = CoreIntrinsics.DefaultValue <ChicStr >();
                runtime.ptr = slice.Pointer;
                runtime.len = slice.Length;
                return runtime;
            }
            public static ChicStr ToRuntimeStr(ReadOnlySpan <byte >span) {
                var runtime = CoreIntrinsics.DefaultValue <ChicStr >();
                runtime.ptr = span.Raw.Data.Pointer;
                runtime.len = span.Length;
                return runtime;
            }
            public static StrPtr FromRuntimeStr(ChicStr slice) {
                var managed = CoreIntrinsics.DefaultValue <StrPtr >();
                managed.Pointer = slice.ptr;
                managed.Length = slice.len;
                return managed;
            }
            public static ReadOnlySpan <byte >FromStringSlice(StrPtr slice) {
                let handle = ValuePointer.CreateConst(PointerIntrinsics.AsByteConst(slice.Pointer), 1, 1);
                return ReadOnlySpan <byte >.FromValuePointer(handle, slice.Length);
            }
            public static ReadOnlySpan <byte >ReadOnlyBytes(ValueConstPtr handle, usize length) {
                if (handle.Size != 1 || handle.Alignment != 1)
                {
                    throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr(BYTE_HANDLE_MESSAGE));
                }
                if (length != 0 && Std.Runtime.Collections.ValuePointer.IsNullConst (handle))
                {
                    throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr(BYTE_HANDLE_MESSAGE));
                }
                return ReadOnlySpan <byte >.FromValuePointer(handle, length);
            }
            public static Span <byte >MutableBytes(ValueMutPtr handle, usize length) {
                if (handle.Size != 1 || handle.Alignment != 1)
                {
                    throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr(BYTE_HANDLE_MESSAGE));
                }
                if (length != 0 && Std.Runtime.Collections.ValuePointer.IsNullMut (handle))
                {
                    throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr(BYTE_HANDLE_MESSAGE));
                }
                return Span <byte >.FromValuePointer(handle, length);
            }
            public static IoError FromStringStatus(int status) {
                switch (status)
                {
                    case 0:
                        // Success
                        return IoError.Success;
                    case 4:
                        // InvalidPointer
                        return IoError.InvalidPointer;
                    case 5:
                        // OutOfBounds
                        return IoError.InvalidData;
                    case 1:
                        // Utf8
                        return IoError.InvalidData;
                    case 3:
                        // AllocationFailed
                        return IoError.Unknown;
                    case 2:
                        // CapacityOverflow
                        return IoError.Unknown;
                    default :
                        return IoError.Unknown;
                    }
                }
                }
