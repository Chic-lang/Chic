namespace Std.Runtime.Native;
// Provide IPv6 socket symbols expected by Std.Platform.IO.Socket on platforms
// where the libc surface exposes only the generic accept/bind/connect/recvfrom/sendto names.
public static class SocketShims
{
    @repr(c) public struct SockAddrIn6
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
    }
    @extern("C") private static extern int accept(int fd, * mut SockAddrIn6 addr, * mut int addrlen);
    @extern("C") private static extern int bind(int fd, * mut SockAddrIn6 addr, int addrlen);
    @extern("C") private static extern int connect(int fd, * mut SockAddrIn6 addr, int addrlen);
    @extern("C") private static extern isize recvfrom(int fd, * mut @expose_address byte buffer, usize length, int flags,
    * mut SockAddrIn6 addr, * mut int addrlen);
    @extern("C") private static extern isize sendto(int fd, * const @expose_address byte buffer, usize length, int flags,
    * const SockAddrIn6 addr, int addrlen);
    @export("accept6") public static int Accept6(int fd, * mut SockAddrIn6 addr, * mut int addrlen) {
        return accept(fd, addr, addrlen);
    }
    @export("bind6") public static int Bind6(int fd, * mut SockAddrIn6 addr, int addrlen) {
        return bind(fd, addr, addrlen);
    }
    @export("connect6") public static int Connect6(int fd, * mut SockAddrIn6 addr, int addrlen) {
        return connect(fd, addr, addrlen);
    }
    @export("recvfrom6") public static isize ReceiveFrom6(int fd, * mut @expose_address byte buffer, usize length, int flags,
    * mut SockAddrIn6 addr, * mut int addrlen) {
        return recvfrom(fd, buffer, length, flags, addr, addrlen);
    }
    @export("sendto6") public static isize SendTo6(int fd, * const @expose_address byte buffer, usize length, int flags,
    * const SockAddrIn6 addr, int addrlen) {
        return sendto(fd, buffer, length, flags, addr, addrlen);
    }
}
