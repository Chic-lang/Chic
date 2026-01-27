namespace Std.Net.Sockets;
/// <summary>Socket type.</summary>
public enum SocketType
{
    Stream = 1, Dgram = 2, Raw = 3,
}
/// <summary>Protocol selection.</summary>
public enum ProtocolType
{
    Tcp = 6, Udp = 17, Icmp = 1, IcmpV6 = 58, Raw = 255,
}
public enum SocketShutdown
{
    Receive = 0, Send = 1, Both = 2,
}
public enum SocketFlags
{
    None = 0,
}
public enum SocketError
{
    Success = 0, WouldBlock = 1, Interrupted = 2, Invalid = 3, Unsupported = 4, PermissionDenied = 5, Unknown = 255,
}
