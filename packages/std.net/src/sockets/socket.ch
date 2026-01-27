namespace Std.Net.Sockets;
import Std.Async;
import Std.Core;
import Std.IO;
import Std.Net;
import Std.Numeric;
import Std.Span;
import PlatformSocket = Std.Platform.IO.Socket;
import PlatformSocketError = Std.Platform.IO.SocketError;
/// <summary>High-level socket wrapper with span-first IO (IPv4 bootstrap).</summary>
public sealed class Socket
{
    private PlatformSocket _inner;
    private bool _ownsHandle;
    private bool _connected;
    private AddressFamily _family;
    private SocketType _type;
    private ProtocolType _protocol;
    private bool _noDelay;
    private uint _scopeId;
    public init(AddressFamily addressFamily, SocketType socketType, ProtocolType protocolType) {
        _family = addressFamily;
        _type = socketType;
        _protocol = protocolType;
        _ownsHandle = true;
        _connected = false;
        _noDelay = false;
        CreateUnderlying();
    }
    private init(PlatformSocket inner, AddressFamily family, SocketType socketType, ProtocolType protocolType, bool connected) {
        _inner = inner;
        _family = family;
        _type = socketType;
        _protocol = protocolType;
        _ownsHandle = true;
        _connected = connected;
        _scopeId = 0;
    }
    public AddressFamily AddressFamily => _family;
    public SocketType SocketType => _type;
    public ProtocolType ProtocolType => _protocol;
    public bool Connected => _connected && _inner.IsValid;
    public bool IsValid => _inner.IsValid;
    public bool NoDelay {
        get {
            return _noDelay;
        }
        set {
            _noDelay = value;
        }
    }
    public EndPoint ?LocalEndPoint => null;
    public EndPoint ?RemoteEndPoint => null;
    public int Available => 0;
    public SocketError Connect(IPAddress address, int port) {
        if (port <0 || port >65535)
        {
            return SocketError.Invalid;
        }
        if (address.IsIPv4)
        {
            let status = _inner.Connect(address.ToIpv4Address(), NumericUnchecked.ToUInt16(port));
            let mapped = MapError(status);
            _connected = mapped == SocketError.Success;
            return mapped;
        }
        let span = address.RawV6Span();
        let status6 = _inner.ConnectV6(span, NumericUnchecked.ToUInt16(port), address.ScopeId);
        let mapped6 = MapError(status6);
        _connected = mapped6 == SocketError.Success;
        return mapped6;
    }
    public SocketError Connect(EndPoint endPoint) {
        if (endPoint == null)
        {
            return SocketError.Invalid;
        }
        return SocketError.Unsupported;
    }
    public Task <SocketError >ConnectAsync(IPAddress address, int port, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Connect canceled");
        }
        let status = Connect(address, port);
        return TaskRuntime.FromResult <SocketError >(status);
    }
    public Task <SocketError >ConnectAsync(string host, int port, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Connect canceled");
        }
        let addresses = Dns.GetHostAddresses(host);
        if (addresses == null || addresses.Length == 0)
        {
            return TaskRuntime.FromResult <SocketError >(SocketError.Unknown);
        }
        let status = Connect(addresses[0], port);
        return TaskRuntime.FromResult <SocketError >(status);
    }
    public SocketError Bind(IPAddress address, int port) {
        if (port <0 || port >65535)
        {
            return SocketError.Invalid;
        }
        if (address.IsIPv4)
        {
            let status = _inner.Bind(address.ToIpv4Address(), NumericUnchecked.ToUInt16(port));
            return MapError(status);
        }
        let status6 = _inner.BindV6(address.RawV6Span(), NumericUnchecked.ToUInt16(port), address.ScopeId);
        return MapError(status6);
    }
    public SocketError Bind(EndPoint localEP) {
        if (localEP == null)
        {
            return SocketError.Invalid;
        }
        return SocketError.Unsupported;
    }
    public SocketError Listen(int backlog) {
        if (_type != SocketType.Stream)
        {
            return SocketError.Unsupported;
        }
        let status = _inner.Listen(backlog);
        return MapError(status);
    }
    public Socket Accept() {
        if (_type != SocketType.Stream)
        {
            throw new Std.NotSupportedException("Accept requires stream sockets");
        }
        var accepted = CoreIntrinsics.DefaultValue <PlatformSocket >();
        var status = PlatformSocketError.Success;
        if (_family == AddressFamily.InterNetworkV6)
        {
            status = _inner.AcceptV6(out accepted);
        }
        else
        {
            status = _inner.Accept(out accepted);
        }
        let mapped = MapError(status);
        if (mapped != SocketError.Success)
        {
            throw new Std.IOException("Accept failed");
        }
        return new Socket(accepted, _family, _type, _protocol, true);
    }
    public Task <Socket >AcceptAsync(CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Accept canceled");
        }
        let socket = Accept();
        return TaskRuntime.FromResult(socket);
    }
    public int Send(ReadOnlySpan <byte >buffer, SocketFlags flags = SocketFlags.None) {
        if (buffer.Length == 0)
        {
            return 0;
        }
        let status = _inner.Send(buffer, out var written);
        let mapped = MapError(status);
        if (mapped != SocketError.Success)
        {
            throw new Std.IOException("Socket send failed");
        }
        return NumericUnchecked.ToInt32(written);
    }
    public Task <int >SendAsync(ReadOnlyMemory <byte >buffer, SocketFlags flags, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Send canceled");
        }
        let written = Send(buffer.Span, flags);
        return TaskRuntime.FromResult(written);
    }
    public int Receive(Span <byte >buffer, SocketFlags flags = SocketFlags.None) {
        if (buffer.Length == 0)
        {
            return 0;
        }
        let status = _inner.Receive(buffer, out var read);
        let mapped = MapError(status);
        if (mapped != SocketError.Success)
        {
            throw new Std.IOException("Socket receive failed");
        }
        return NumericUnchecked.ToInt32(read);
    }
    public Task <int >ReceiveAsync(Memory <byte >buffer, SocketFlags flags, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Receive canceled");
        }
        let read = Receive(buffer.Span, flags);
        return TaskRuntime.FromResult(read);
    }
    public int SendTo(ReadOnlySpan <byte >buffer, EndPoint endPoint) {
        throw new Std.NotSupportedException("Endpoint type not supported");
    }
    public Task <int >SendToAsync(ReadOnlyMemory <byte >buffer, EndPoint endPoint, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("SendTo canceled");
        }
        let written = SendTo(buffer.Span, endPoint);
        return TaskRuntime.FromResult(written);
    }
    public int ReceiveFrom(Memory <byte >buffer, ref EndPoint remoteEP) {
        remoteEP = remoteEP;
        if (_family == AddressFamily.InterNetwork)
        {
            let status = _inner.ReceiveFrom(buffer.Span, out var read, out var addr, out var port);
            let mapped = MapError(status);
            if (mapped != SocketError.Success)
            {
                throw new Std.IOException("Socket recvfrom failed");
            }
            return NumericUnchecked.ToInt32(read);
        }
        let status6 = _inner.ReceiveFromV6(buffer.Span, out var read6, Span <byte >.Empty, out var addr6, out var port6);
        let mapped6 = MapError(status6);
        if (mapped6 != SocketError.Success)
        {
            throw new Std.IOException("Socket recvfrom failed");
        }
        return NumericUnchecked.ToInt32(read6);
    }
    public Task <int >ReceiveFromAsync(Memory <byte >buffer, CancellationToken ct, ref EndPoint remoteEP) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("ReceiveFrom canceled");
        }
        let read = ReceiveFrom(buffer, ref remoteEP);
        return TaskRuntime.FromResult(read);
    }
    public void Shutdown(SocketShutdown how) {
        if (!_inner.IsValid)
        {
            return;
        }
        if (how == SocketShutdown.Send || how == SocketShutdown.Both)
        {
            _inner.ShutdownWrite();
        }
    }
    public void Close() {
        if (_ownsHandle && _inner.IsValid)
        {
            _inner.Close();
        }
        _connected = false;
    }
    public void dispose(ref this) {
        Close();
    }
    private void CreateUnderlying() {
        var owner = this;
        var status = PlatformSocketError.Success;
        if (_type == SocketType.Stream && _protocol == ProtocolType.Tcp)
        {
            if (_family == AddressFamily.InterNetworkV6)
            {
                status = PlatformSocket.CreateTcpV6(out owner._inner);
            }
            else
            {
                status = PlatformSocket.CreateTcp(out owner._inner);
            }
        }
        else if (_type == SocketType.Dgram && _protocol == ProtocolType.Udp)
        {
            if (_family == AddressFamily.InterNetworkV6)
            {
                status = PlatformSocket.CreateUdpV6(out owner._inner);
            }
            else
            {
                status = PlatformSocket.CreateUdp(out owner._inner);
            }
        }
        else if (_type == SocketType.Raw)
        {
            status = PlatformSocket.CreateRaw(NumericUnchecked.ToInt32(_protocol), out owner._inner);
        }
        else
        {
            throw new Std.NotSupportedException("Socket combination not supported");
        }
        if (status != PlatformSocketError.Success)
        {
            throw new Std.IOException("Socket creation failed");
        }
    }
    private static SocketError MapError(PlatformSocketError error) {
        if (error == PlatformSocketError.Success)
        {
            return SocketError.Success;
        }
        if (error == PlatformSocketError.WouldBlock)
        {
            return SocketError.WouldBlock;
        }
        if (error == PlatformSocketError.Interrupted)
        {
            return SocketError.Interrupted;
        }
        if (error == PlatformSocketError.Invalid)
        {
            return SocketError.Invalid;
        }
        if (error == PlatformSocketError.Unsupported)
        {
            return SocketError.Unsupported;
        }
        return SocketError.Unknown;
    }
}
