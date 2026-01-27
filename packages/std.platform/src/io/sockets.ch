namespace Std.Platform.IO;
import Std.Memory;
import Std.Span;
import Std.Strings;
import Std.Numeric;
import Std.Core;
public enum SocketError
{
    Success = 0, WouldBlock = 1, Interrupted = 2, Invalid = 3, Unsupported = 4, Unknown = 255,
}
@repr(c) internal struct SockAddrIn
{
    public ushort Family;
    public ushort PortNet;
    public uint AddrNet;
    public ulong ZeroPadding;
}
@repr(c) internal struct SockAddrIn6
{
    public ushort Family;
    public ushort PortNet;
    public uint FlowInfo;
    public byte Addr0;
    public byte Addr1;
    public byte Addr2;
    public byte Addr3;
    public byte Addr4;
    public byte Addr5;
    public byte Addr6;
    public byte Addr7;
    public byte Addr8;
    public byte Addr9;
    public byte Addr10;
    public byte Addr11;
    public byte Addr12;
    public byte Addr13;
    public byte Addr14;
    public byte Addr15;
    public uint ScopeId;
    internal static void SetAddr(ref SockAddrIn6 addr, usize index, byte value) {
        let idx = NumericUnchecked.ToInt32(index);
        if (idx == 0)
        {
            addr.Addr0 = value;
            return;
        }
        if (idx == 1)
        {
            addr.Addr1 = value;
            return;
        }
        if (idx == 2)
        {
            addr.Addr2 = value;
            return;
        }
        if (idx == 3)
        {
            addr.Addr3 = value;
            return;
        }
        if (idx == 4)
        {
            addr.Addr4 = value;
            return;
        }
        if (idx == 5)
        {
            addr.Addr5 = value;
            return;
        }
        if (idx == 6)
        {
            addr.Addr6 = value;
            return;
        }
        if (idx == 7)
        {
            addr.Addr7 = value;
            return;
        }
        if (idx == 8)
        {
            addr.Addr8 = value;
            return;
        }
        if (idx == 9)
        {
            addr.Addr9 = value;
            return;
        }
        if (idx == 10)
        {
            addr.Addr10 = value;
            return;
        }
        if (idx == 11)
        {
            addr.Addr11 = value;
            return;
        }
        if (idx == 12)
        {
            addr.Addr12 = value;
            return;
        }
        if (idx == 13)
        {
            addr.Addr13 = value;
            return;
        }
        if (idx == 14)
        {
            addr.Addr14 = value;
            return;
        }
        if (idx == 15)
        {
            addr.Addr15 = value;
            return;
        }
    }
    internal static byte GetAddr(ref SockAddrIn6 addr, usize index) {
        let idx = NumericUnchecked.ToInt32(index);
        if (idx == 0)
        {
            return addr.Addr0;
        }
        if (idx == 1)
        {
            return addr.Addr1;
        }
        if (idx == 2)
        {
            return addr.Addr2;
        }
        if (idx == 3)
        {
            return addr.Addr3;
        }
        if (idx == 4)
        {
            return addr.Addr4;
        }
        if (idx == 5)
        {
            return addr.Addr5;
        }
        if (idx == 6)
        {
            return addr.Addr6;
        }
        if (idx == 7)
        {
            return addr.Addr7;
        }
        if (idx == 8)
        {
            return addr.Addr8;
        }
        if (idx == 9)
        {
            return addr.Addr9;
        }
        if (idx == 10)
        {
            return addr.Addr10;
        }
        if (idx == 11)
        {
            return addr.Addr11;
        }
        if (idx == 12)
        {
            return addr.Addr12;
        }
        if (idx == 13)
        {
            return addr.Addr13;
        }
        if (idx == 14)
        {
            return addr.Addr14;
        }
        if (idx == 15)
        {
            return addr.Addr15;
        }
        return 0;
    }
}
public struct Ipv4Address
{
    public uint RawNet;
    public static Ipv4Address Loopback() {
        var addr = CoreIntrinsics.DefaultValue <Ipv4Address >();
        addr.RawNet = 0x0100007F;
        // 127.0.0.1 in network order (little-endian store)
        return addr;
    }
    public static bool TryParse(string text, out Ipv4Address addr) {
        addr = CoreIntrinsics.DefaultValue <Ipv4Address >();
        var tmp = text;
        let utf8 = IoTyped.FromStringSlice(SpanIntrinsics.chic_rt_string_as_slice(tmp));
        var buf = Span <byte >.StackAlloc(utf8.Length + 1);
        buf.Slice(0, utf8.Length).CopyFrom(utf8);
        let ptr = PointerIntrinsics.AsByteConstFromMut(buf.Raw.Data.Pointer);
        var parsed = 0u;
        let status = SocketPlatform.InetPton(ptr, out parsed);
        if (status == 1)
        {
            addr.RawNet = parsed;
            return true;
        }
        addr.RawNet = 0;
        return false;
    }
}
public struct Socket
{
    internal int Fd;
    private const int AfInet = 2;
    private const int AfInet6 = 10;
    private const int SockStream = 1;
    private const int SockDgram = 2;
    private const int SockRaw = 3;
    private const int ShutdownWrite = 1;
    public static SocketError CreateTcp(out Socket socket) {
        socket = CoreIntrinsics.DefaultValue <Socket >();
        socket.Fd = - 1;
        socket.Fd = SocketNative.socket(AfInet, SockStream, 0);
        if (socket.Fd <0)
        {
            return SocketError.Unknown;
        }
        return SocketError.Success;
    }
    public static SocketError CreateTcpV6(out Socket socket) {
        socket = CoreIntrinsics.DefaultValue <Socket >();
        socket.Fd = - 1;
        socket.Fd = SocketNative.socket(AfInet6, SockStream, 0);
        if (socket.Fd <0)
        {
            return SocketError.Unknown;
        }
        return SocketError.Success;
    }
    public static SocketError CreateUdp(out Socket socket) {
        socket = CoreIntrinsics.DefaultValue <Socket >();
        socket.Fd = - 1;
        socket.Fd = SocketNative.socket(AfInet, SockDgram, 0);
        if (socket.Fd <0)
        {
            return SocketError.Unknown;
        }
        return SocketError.Success;
    }
    public static SocketError CreateUdpV6(out Socket socket) {
        socket = CoreIntrinsics.DefaultValue <Socket >();
        socket.Fd = - 1;
        socket.Fd = SocketNative.socket(AfInet6, SockDgram, 0);
        if (socket.Fd <0)
        {
            return SocketError.Unknown;
        }
        return SocketError.Success;
    }
    public static SocketError CreateRaw(int protocol, out Socket socket) {
        socket = CoreIntrinsics.DefaultValue <Socket >();
        socket.Fd = - 1;
        socket.Fd = SocketNative.socket(AfInet, SockRaw, protocol);
        if (socket.Fd <0)
        {
            return SocketError.Unknown;
        }
        return SocketError.Success;
    }
    public bool IsValid => Fd >= 0;
    public SocketError Connect(Ipv4Address address, ushort port) {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn >();
        sockaddr.Family = AfInet;
        sockaddr.PortNet = SocketNative.htons(port);
        sockaddr.AddrNet = address.RawNet;
        sockaddr.ZeroPadding = 0;
        let status = SocketNative.connect(Fd, ref sockaddr, sizeof(SockAddrIn));
        if (status == 0)
        {
            return SocketError.Success;
        }
        return SocketError.Unknown;
    }
    public SocketError ConnectV6(ReadOnlySpan <byte >address16, ushort port, uint scopeId) {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn6 >();
        sockaddr.Family = AfInet6;
        sockaddr.PortNet = SocketNative.htons(port);
        sockaddr.FlowInfo = 0;
        sockaddr.ScopeId = scopeId;
        var idx = 0usize;
        while (idx <16 && idx <address16.Length)
        {
            SockAddrIn6.SetAddr(ref sockaddr, idx, address16[idx]);
            idx += 1;
        }
        let status = SocketNative.connect6(Fd, ref sockaddr, sizeof(SockAddrIn6));
        return status == 0 ?SocketError.Success : SocketError.Unknown;
    }
    public SocketError Send(ReadOnlySpan <byte >data, out usize written) {
        var count = 0usize;
        if (!IsValid)
        {
            written = 0;
            return SocketError.Invalid;
        }
        if (data.Length == 0)
        {
            written = 0;
            return SocketError.Success;
        }
        let result = SocketNative.send(Fd, data.Raw.Data.Pointer, data.Length, 0);
        if (result <0)
        {
            written = 0;
            return SocketError.Unknown;
        }
        count = NumericUnchecked.ToUSize(result);
        written = count;
        return SocketError.Success;
    }
    public SocketError Receive(Span <byte >destination, out usize read) {
        if (!IsValid)
        {
            read = 0;
            return SocketError.Invalid;
        }
        if (destination.Length == 0)
        {
            read = 0;
            return SocketError.Success;
        }
        let result = SocketNative.recv(Fd, destination.Raw.Data.Pointer, destination.Length, 0);
        if (result <0)
        {
            read = 0;
            return SocketError.Unknown;
        }
        if (result == 0)
        {
            // Peer closed the connection gracefully.
            read = 0;
            return SocketError.Success;
        }
        read = NumericUnchecked.ToUSize(result);
        return SocketError.Success;
    }
    public SocketError Bind(Ipv4Address address, ushort port) {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn >();
        sockaddr.Family = AfInet;
        sockaddr.PortNet = SocketNative.htons(port);
        sockaddr.AddrNet = address.RawNet;
        sockaddr.ZeroPadding = 0;
        let status = SocketNative.bind(Fd, ref sockaddr, sizeof(SockAddrIn));
        return status == 0 ?SocketError.Success : SocketError.Unknown;
    }
    public SocketError BindV6(ReadOnlySpan <byte >address16, ushort port, uint scopeId) {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn6 >();
        sockaddr.Family = AfInet6;
        sockaddr.PortNet = SocketNative.htons(port);
        sockaddr.ScopeId = scopeId;
        var idx = 0usize;
        while (idx <16 && idx <address16.Length)
        {
            SockAddrIn6.SetAddr(ref sockaddr, idx, address16[idx]);
            idx += 1;
        }
        let status = SocketNative.bind6(Fd, ref sockaddr, sizeof(SockAddrIn6));
        return status == 0 ?SocketError.Success : SocketError.Unknown;
    }
    public SocketError Listen(int backlog) {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        let status = SocketNative.listen(Fd, backlog);
        return status == 0 ?SocketError.Success : SocketError.Unknown;
    }
    public SocketError Accept(out Socket accepted) {
        accepted = CoreIntrinsics.DefaultValue <Socket >();
        accepted.Fd = - 1;
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var addr = CoreIntrinsics.DefaultValue <SockAddrIn >();
        var len = (int) sizeof(SockAddrIn);
        let fd = SocketNative.accept(Fd, ref addr, ref len);
        if (fd <0)
        {
            return SocketError.Unknown;
        }
        accepted.Fd = fd;
        return SocketError.Success;
    }
    public SocketError AcceptV6(out Socket accepted) {
        accepted = CoreIntrinsics.DefaultValue <Socket >();
        accepted.Fd = - 1;
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var addr = CoreIntrinsics.DefaultValue <SockAddrIn6 >();
        var len = (int) sizeof(SockAddrIn6);
        let fd = SocketNative.accept6(Fd, ref addr, ref len);
        if (fd <0)
        {
            return SocketError.Unknown;
        }
        accepted.Fd = fd;
        return SocketError.Success;
    }
    public SocketError SendTo(ReadOnlySpan <byte >data, Ipv4Address address, ushort port, out usize written) {
        written = 0usize;
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn >();
        sockaddr.Family = AfInet;
        sockaddr.PortNet = SocketNative.htons(port);
        sockaddr.AddrNet = address.RawNet;
        sockaddr.ZeroPadding = 0;
        let result = SocketNative.sendto(Fd, data.Raw.Data.Pointer, data.Length, 0, ref sockaddr, sizeof(SockAddrIn));
        if (result <0)
        {
            return SocketError.Unknown;
        }
        written = NumericUnchecked.ToUSize(result);
        return SocketError.Success;
    }
    public SocketError SendToV6(ReadOnlySpan <byte >data, ReadOnlySpan <byte >address16, ushort port, uint scopeId, out usize written) {
        written = 0usize;
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn6 >();
        sockaddr.Family = AfInet6;
        sockaddr.PortNet = SocketNative.htons(port);
        sockaddr.ScopeId = scopeId;
        var idx = 0usize;
        while (idx <16 && idx <address16.Length)
        {
            SockAddrIn6.SetAddr(ref sockaddr, idx, address16[idx]);
            idx += 1;
        }
        let result = SocketNative.sendto6(Fd, data.Raw.Data.Pointer, data.Length, 0, ref sockaddr, sizeof(SockAddrIn6));
        if (result <0)
        {
            return SocketError.Unknown;
        }
        written = NumericUnchecked.ToUSize(result);
        return SocketError.Success;
    }
    public SocketError ReceiveFrom(Span <byte >destination, out usize read, out Ipv4Address address, out ushort port) {
        read = 0usize;
        address = CoreIntrinsics.DefaultValue <Ipv4Address >();
        port = 0;
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn >();
        var len = (int) sizeof(SockAddrIn);
        let result = SocketNative.recvfrom(Fd, destination.Raw.Data.Pointer, destination.Length, 0, ref sockaddr, ref len);
        if (result <0)
        {
            return SocketError.Unknown;
        }
        read = NumericUnchecked.ToUSize(result);
        address.RawNet = sockaddr.AddrNet;
        port = SocketNative.ntohs(sockaddr.PortNet);
        return SocketError.Success;
    }
    public SocketError ReceiveFromV6(Span <byte >destination, out usize read, Span <byte >address16, out ushort port, out uint scopeId) {
        read = 0usize;
        port = 0;
        scopeId = 0;
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        var sockaddr = CoreIntrinsics.DefaultValue <SockAddrIn6 >();
        var len = (int) sizeof(SockAddrIn6);
        let result = SocketNative.recvfrom6(Fd, destination.Raw.Data.Pointer, destination.Length, 0, ref sockaddr, ref len);
        if (result <0)
        {
            return SocketError.Unknown;
        }
        read = NumericUnchecked.ToUSize(result);
        scopeId = sockaddr.ScopeId;
        var idx = 0usize;
        while (idx <16 && idx <address16.Length)
        {
            address16[idx] = SockAddrIn6.GetAddr(ref sockaddr, idx);
            idx += 1;
        }
        port = SocketNative.ntohs(sockaddr.PortNet);
        return SocketError.Success;
    }
    public SocketError ShutdownWrite() {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        let status = SocketNative.shutdown(Fd, ShutdownWrite);
        return status == 0 ?SocketError.Success : SocketError.Unknown;
    }
    public SocketError Close() {
        if (!IsValid)
        {
            return SocketError.Invalid;
        }
        let status = SocketNative.close(Fd);
        Fd = - 1;
        return status == 0 ?SocketError.Success : SocketError.Unknown;
    }
}
internal static class SocketPlatform
{
    @extern("C") public static extern int inet_pton(int af, * const @expose_address byte src, * mut uint dst);
    public static int InetPton(* const @expose_address byte src, out uint parsed) {
        var value = 0u;
        let status = inet_pton(2, src, & value);
        parsed = value;
        return status;
    }
    public static int InetPton6(* const @expose_address byte src, Span <byte >destination) {
        unsafe {
            let status = inet_pton(10, src, destination.Raw.Data.Pointer);
            return status;
        }
    }
}
internal static class SocketNative
{
    @extern("C") internal static extern int socket(int domain, int typ, int protocol);
    @extern("C") internal static extern int connect(int fd, ref SockAddrIn sockaddr, int addrlen);
    @extern("C") internal static extern int connect6(int fd, ref SockAddrIn6 sockaddr, int addrlen);
    @extern("C") internal static extern int bind(int fd, ref SockAddrIn sockaddr, int addrlen);
    @extern("C") internal static extern int bind6(int fd, ref SockAddrIn6 sockaddr, int addrlen);
    @extern("C") internal static extern int listen(int fd, int backlog);
    @extern("C") internal static extern int accept(int fd, ref SockAddrIn sockaddr, ref int addrlen);
    @extern("C") internal static extern int accept6(int fd, ref SockAddrIn6 sockaddr, ref int addrlen);
    @extern("C") internal static extern isize recv(int fd, * mut @expose_address byte buffer, usize length, int flags);
    @extern("C") internal static extern isize send(int fd, * const @expose_address byte buffer, usize length, int flags);
    @extern("C") internal static extern isize recvfrom6(int fd, * mut @expose_address byte buffer, usize length, int flags,
    ref SockAddrIn6 addr, ref int addrlen);
    @extern("C") internal static extern isize sendto6(int fd, * const @expose_address byte buffer, usize length, int flags,
    ref SockAddrIn6 addr, int addrlen);
    @extern("C") internal static extern isize recvfrom(int fd, * mut @expose_address byte buffer, usize length, int flags,
    ref SockAddrIn addr, ref int addrlen);
    @extern("C") internal static extern isize sendto(int fd, * const @expose_address byte buffer, usize length, int flags,
    ref SockAddrIn addr, int addrlen);
    @extern("C") internal static extern int close(int fd);
    @extern("C") internal static extern int shutdown(int fd, int how);
    @extern("C") internal static extern ushort htons(ushort value);
    @extern("C") internal static extern ushort ntohs(ushort value);
}
