namespace Std.Platform.Net;
import Std.Memory;
import Std.Span;
import Std.Strings;
import Std.Numeric;
import Std.Core;
@repr(c) internal struct AddrInfo
{
    internal int Flags;
    internal int Family;
    internal int SockType;
    internal int Protocol;
    internal usize AddrLen;
    internal * mut @expose_address byte Addr;
    internal * mut @expose_address AddrInfo Next;
}
internal static class DnsNative
{
    @extern("C") internal static extern int getaddrinfo(* const @expose_address byte node, * const @expose_address byte service,
    * const @expose_address byte hints, * mut @expose_address AddrInfo * result);
    @extern("C") internal static extern void freeaddrinfo(* mut @expose_address AddrInfo res);
}
public enum DnsError
{
    Success = 0, Failure = 1, Temporary = 2, NotSupported = 3,
}
public struct DnsResult
{
    public DnsError Error;
    internal AddrInfo * Head;
}
public static class DnsPlatform
{
    public static DnsResult Resolve(string host, int family) {
        var hostText = host;
        let utf8 = SpanIntrinsics.chic_rt_string_as_slice(hostText);
        var hostBuf = Span <byte >.StackAlloc(utf8.len + 1);
        hostBuf.Slice(0, utf8.len).CopyFrom(IoTyped.FromStringSlice(utf8));
        let hostPtr = PointerIntrinsics.AsByteConstFromMut(hostBuf.Raw.Data.Pointer);
        var head = CoreIntrinsics.DefaultValue <AddrInfo * >();
        var hints = CoreIntrinsics.DefaultValue <AddrInfo >();
        var hintsPtr = CoreIntrinsics.DefaultValue <AddrInfo * >();
        if (family != 0)
        {
            hints.Family = family;
            hintsPtr = & hints;
        }
        let status = DnsNative.getaddrinfo(hostPtr, null, hintsPtr, & head);
        var result = CoreIntrinsics.DefaultValue <DnsResult >();
        result.Head = head;
        result.Error = status == 0 ?DnsError.Success : DnsError.Failure;
        return result;
    }
    public static void Free(DnsResult result) {
        if (result.Head != null)
        {
            DnsNative.freeaddrinfo(result.Head);
        }
    }
}
