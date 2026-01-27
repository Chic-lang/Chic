import Std;
import Std.Memory;
import Std.Numeric;
import Std.Span;
import Std.Strings;

namespace Exec;

public int Main()
{
    IntPtr ptr = IntPtr.From((nint)0x2a);
    Span<byte> buf = StackAlloc.Span<byte>(16);
    if (!ptr.TryFormat(buf, out var written, "x"))
    {
        return 10;
    }
    string hex = Utf8String.FromSpan(buf.AsReadOnly().Slice(0, written));
    if (hex != "2a")
    {
        return 11;
    }

    UIntPtr uptr = UIntPtr.From((nuint)0x2a);
    Span<byte> ubuf = StackAlloc.Span<byte>(16);
    if (!uptr.TryFormat(ubuf, out written, "X"))
    {
        return 12;
    }
    string uhex = Utf8String.FromSpan(ubuf.AsReadOnly().Slice(0, written));
    if (uhex != "2A")
    {
        return 13;
    }
    if (
        !NumericParse.TryParseUIntPtr(uhex.AsUtf8Span(), out var parsed)
        || parsed != uptr.ToUIntPtr()
    )
    {
        return 14;
    }

    return 0;
}
