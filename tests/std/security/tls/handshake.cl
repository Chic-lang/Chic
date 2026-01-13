namespace Exec;

import Std.Async;
import Std.Core;
import Std.IO;
import Std.Net;
import Std.Net.Sockets;
import Std.Numeric;
import Std.Platform.Thread;
import Std.Security.Tls;
import Std.Security.Cryptography;
import Std.Span;

/// <summary>Test stream that can corrupt the next read.</summary>
public sealed class CorruptingStream : Stream
{
    private Stream _inner;
    private bool _disposed;

    public bool CorruptNext { get; set; }

    public init(Stream inner)
    {
        _inner = inner;
        _disposed = false;
        CorruptNext = false;
    }

    public override bool CanRead => !_disposed && _inner.CanRead;

    public override bool CanWrite => !_disposed && _inner.CanWrite;

    public override bool CanSeek => false;

    public override int Read(Span<byte> buffer)
    {
        let read = _inner.Read(buffer);
        if (CorruptNext && read > 0)
        {
            if (read > 5usize)
            {
                let idx = NumericUnchecked.ToUSize(read - 1);
                buffer[idx] = NumericUnchecked.ToByte(buffer[idx] ^ 0xFFu8);
                CorruptNext = false;
            }
        }
        return read;
    }

    public override void Write(ReadOnlySpan<byte> buffer)
    {
        _inner.Write(buffer);
    }

    public override void Flush()
    {
        _inner.Flush();
    }

    public override long Seek(long offset, SeekOrigin origin)
    {
        throw new Std.NotSupportedException("Seek not supported");
    }

    public override void SetLength(long value)
    {
        throw new Std.NotSupportedException("SetLength not supported");
    }

    protected override void Dispose(bool disposing)
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        if (disposing)
        {
            _inner.Dispose();
        }
        base.Dispose(disposing);
    }
}

testcase Tls13HandshakeLoopback()
{
    let port = 50443;
    var serverOpts = new TlsServerOptions();
    serverOpts.ServerName = "localhost";
    serverOpts.CertificateChain = TestCertificate();
    var serverThread = SpawnEchoServer(port, serverOpts, 4);

    var clientSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let connectStatus = clientSocket.Connect(IPAddress.Parse("127.0.0.1"), port);
    if (connectStatus != SocketError.Success)
    {
        return false;
    }
    var clientStream = new NetworkStream(clientSocket, true);
    var tlsClient = new TlsStream(clientStream, false);
    var clientOpts = new TlsClientOptions();
    clientOpts.ServerName = "localhost";
    clientOpts.AllowUntrustedCertificates = true;
    tlsClient.AuthenticateAsClientAsync(clientOpts, CoreIntrinsics.DefaultValue<CancellationToken>());
    var payload = new byte[4];
    payload[0usize] = 1u8;
    payload[1usize] = 2u8;
    payload[2usize] = 3u8;
    payload[3usize] = 4u8;
    tlsClient.Write(ReadOnlySpan<byte>.FromArray(ref payload));
    var echo = new byte[4];
    let received = tlsClient.Read(Span<byte>.FromArray(ref echo));
    let joinStatus = serverThread.Join();
    return received == 4
        && echo[0usize] == 1u8
        && echo[1usize] == 2u8
        && echo[2usize] == 3u8
        && echo[3usize] == 4u8
        && joinStatus == ThreadStatus.Success;
}

testcase Tls12HandshakeLoopback()
{
    let port = 50444;
    var serverOpts = new TlsServerOptions();
    serverOpts.ServerName = "localhost";
    serverOpts.CertificateChain = TestCertificate();
    var protocols = new TlsProtocol[1];
    protocols[0] = TlsProtocol.Tls12;
    serverOpts.EnabledProtocols = protocols;
    var serverThread = SpawnEchoServer(port, serverOpts, 3);

    var clientSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let connectStatus = clientSocket.Connect(IPAddress.Parse("127.0.0.1"), port);
    if (connectStatus != SocketError.Success)
    {
        return false;
    }
    var clientStream = new NetworkStream(clientSocket, true);
    var tlsClient = new TlsStream(clientStream, false);
    var clientOpts = new TlsClientOptions();
    var clientProtocols = new TlsProtocol[1];
    clientProtocols[0] = TlsProtocol.Tls12;
    clientOpts.EnabledProtocols = clientProtocols;
    clientOpts.ServerName = "localhost";
    clientOpts.AllowUntrustedCertificates = true;
    tlsClient.AuthenticateAsClientAsync(clientOpts, CoreIntrinsics.DefaultValue<CancellationToken>());
    var payload = new byte[3];
    payload[0usize] = 9u8;
    payload[1usize] = 8u8;
    payload[2usize] = 7u8;
    tlsClient.Write(ReadOnlySpan<byte>.FromArray(ref payload));
    var echo = new byte[3];
    let received = tlsClient.Read(Span<byte>.FromArray(ref echo));
    let joinStatus = serverThread.Join();
    return received == 3
        && echo[0usize] == 9u8
        && echo[1usize] == 8u8
        && echo[2usize] == 7u8
        && joinStatus == ThreadStatus.Success;
}

testcase TlsCertificateValidationSuccess()
{
    let port = 50445;
    var root = TestCertificate();
    let rootPath = "tls_root.cert";
    WriteFile(rootPath, root);

    var serverOpts = new TlsServerOptions();
    serverOpts.ServerName = "example.test";
    serverOpts.CertificateChain = root;
    var serverThread = SpawnEchoServer(port, serverOpts, 1);

    var clientSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let connectStatus = clientSocket.Connect(IPAddress.Parse("127.0.0.1"), port);
    if (connectStatus != SocketError.Success)
    {
        return false;
    }
    var clientStream = new NetworkStream(clientSocket, true);
    var tlsClient = new TlsStream(clientStream, false);
    var clientOpts = new TlsClientOptions();
    clientOpts.ServerName = "example.test";
    clientOpts.AllowUntrustedCertificates = false;
    var roots = new string[1];
    roots[0] = rootPath;
    clientOpts.TrustedRootFiles = roots;
    tlsClient.AuthenticateAsClientAsync(clientOpts, CoreIntrinsics.DefaultValue<CancellationToken>());
    var ping = new byte[1];
    ping[0usize] = 42u8;
    tlsClient.Write(ReadOnlySpan<byte>.FromArray(ref ping));
    let joinStatus = serverThread.Join();
    return joinStatus == ThreadStatus.Success;
}

testcase TlsCertificateValidationFailsForWrongHost()
{
    let port = 50446;
    var root = TestCertificate();
    let rootPath = "tls_root_wrong.cert";
    WriteFile(rootPath, root);

    var serverOpts = new TlsServerOptions();
    serverOpts.ServerName = "server.internal";
    serverOpts.CertificateChain = root;
    var serverThread = SpawnEchoServer(port, serverOpts, 0);

    var clientSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let connectStatus = clientSocket.Connect(IPAddress.Parse("127.0.0.1"), port);
    if (connectStatus != SocketError.Success)
    {
        return false;
    }
    var clientStream = new NetworkStream(clientSocket, true);
    var tlsClient = new TlsStream(clientStream, false);
    var clientOpts = new TlsClientOptions();
    clientOpts.ServerName = "wrong.host";
    clientOpts.AllowUntrustedCertificates = false;
    var roots = new string[1];
    roots[0] = rootPath;
    clientOpts.TrustedRootFiles = roots;
    var failed = false;
    try
    {
        tlsClient.AuthenticateAsClientAsync(clientOpts, CoreIntrinsics.DefaultValue<CancellationToken>());
    }
    catch (TlsCertificateException)
    {
        failed = true;
    }
    let joinStatus = serverThread.Join();
    return failed && joinStatus == ThreadStatus.Success;
}

testcase TlsProtocolMismatchFails()
{
    let port = 50447;
    var serverOpts = new TlsServerOptions();
    var protocols = new TlsProtocol[1];
    protocols[0] = TlsProtocol.Tls12;
    serverOpts.EnabledProtocols = protocols;
    serverOpts.ServerName = "nomatch.test";
    serverOpts.CertificateChain = TestCertificate();
    var serverThread = SpawnEchoServer(port, serverOpts, 0);

    var clientSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let connectStatus = clientSocket.Connect(IPAddress.Parse("127.0.0.1"), port);
    if (connectStatus != SocketError.Success)
    {
        return false;
    }
    var clientStream = new NetworkStream(clientSocket, true);
    var tlsClient = new TlsStream(clientStream, false);
    var clientOpts = new TlsClientOptions();
    var clientProtocols = new TlsProtocol[1];
    clientProtocols[0] = TlsProtocol.Tls13;
    clientOpts.EnabledProtocols = clientProtocols;
    clientOpts.ServerName = "nomatch.test";
    clientOpts.AllowUntrustedCertificates = true;
    var failed = false;
    try
    {
        tlsClient.AuthenticateAsClientAsync(clientOpts, CoreIntrinsics.DefaultValue<CancellationToken>());
    }
    catch (TlsHandshakeException)
    {
        failed = true;
    }
    let joinStatus = serverThread.Join();
    return failed && joinStatus == ThreadStatus.Success;
}

testcase TlsTamperedCiphertextFails()
{
    let port = 50448;
    var tamperRunner = new TamperServerThread(port);
    var serverThread = ThreadBuilder.Spawn(ThreadStartFactory.From(tamperRunner));

    var clientSocket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
    let connectStatus = clientSocket.Connect(IPAddress.Parse("127.0.0.1"), port);
    if (connectStatus != SocketError.Success)
    {
        return false;
    }
    var clientStream = new NetworkStream(clientSocket, true);
    var tlsClient = new TlsStream(clientStream, false);
    var clientOpts = new TlsClientOptions();
    clientOpts.ServerName = "tamper.test";
    clientOpts.AllowUntrustedCertificates = true;
    tlsClient.AuthenticateAsClientAsync(clientOpts, CoreIntrinsics.DefaultValue<CancellationToken>());
    var payload = new byte[2];
    payload[0usize] = 5u8;
    payload[1usize] = 6u8;
    var failed = false;
    try
    {
        tlsClient.Write(ReadOnlySpan<byte>.FromArray(ref payload));
        var buffer = new byte[2];
        let got = tlsClient.Read(Span<byte>.FromArray(ref buffer));
        if (got == 0)
        {
            failed = true;
        }
    }
    catch (TlsException)
    {
        failed = true;
    }
    let joinStatus = serverThread.Join();
    return tamperRunner.Tampered && joinStatus == ThreadStatus.Success && failed;
}

private static byte[] TestCertificate()
{
    var cert = new byte[8];
    cert[0usize] = 0x01u8;
    cert[1usize] = 0x23u8;
    cert[2usize] = 0x45u8;
    cert[3usize] = 0x67u8;
    cert[4usize] = 0x89u8;
    cert[5usize] = 0xABu8;
    cert[6usize] = 0xCDu8;
    cert[7usize] = 0xEFu8;
    return cert;
}

private static void WriteFile(string path, byte[] data)
{
    var stream = new FileStream(path, FileMode.Create, FileAccess.Write, FileShare.None);
    stream.Write(ReadOnlySpan<byte>.FromArray(ref data));
    stream.Flush();
    stream.Dispose();
}

private static Thread SpawnEchoServer(int port, TlsServerOptions opts, int expectedRead)
{
    var runner = new EchoServerThread(port, opts, expectedRead);
    return ThreadBuilder.Spawn(ThreadStartFactory.From(runner));
}

private sealed class EchoServerThread : ThreadStart
{
    private int _port;
    private TlsServerOptions _opts;
    private int _expectedRead;

    public init(int port, TlsServerOptions opts, int expectedRead)
    {
        _port = port;
        _opts = opts;
        _expectedRead = expectedRead;
    }

    public void Run()
    {
        var listener = (Socket)null;
        var accepted = (Socket)null;
        var netStream = (NetworkStream)null;
        var tls = (TlsStream)null;
        try
        {
            listener = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
            let bindStatus = listener.Bind(IPAddress.Parse("127.0.0.1"), _port);
            if (bindStatus != SocketError.Success)
            {
                return;
            }
            listener.Listen(1);
            accepted = listener.Accept();
            netStream = new NetworkStream(accepted, true);
            tls = new TlsStream(netStream, false);
            tls.AuthenticateAsServerAsync(_opts, CoreIntrinsics.DefaultValue<CancellationToken>());
            if (_expectedRead > 0)
            {
                var buffer = new byte[_expectedRead];
                let read = tls.Read(Span<byte>.FromArray(ref buffer));
                if (read > 0)
                {
                    tls.Write(
                        ReadOnlySpan<byte>.FromArray(ref buffer).Slice(0usize, NumericUnchecked.ToUSize(read))
                    );
                    tls.Flush();
                }
            }
        }
        catch (Std.Exception)
        { }
        finally
        {
            if (tls != null)
            {
                tls.Dispose();
            }
            else if (netStream != null)
            {
                netStream.Dispose();
            }
            if (accepted != null)
            {
                accepted.Close();
            }
            if (listener != null)
            {
                listener.Close();
            }
        }
    }
}

private sealed class TamperServerThread : ThreadStart
{
    private int _port;
    private bool _tampered;

    public bool Tampered => _tampered;

    public init(int port)
    {
        _port = port;
        _tampered = false;
    }

    public void Run()
    {
        var listener = (Socket)null;
        var accepted = (Socket)null;
        var netStream = (NetworkStream)null;
        var corrupt = (CorruptingStream)null;
        var tls = (TlsStream)null;
        try
        {
            listener = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
            let bindStatus = listener.Bind(IPAddress.Parse("127.0.0.1"), _port);
            if (bindStatus != SocketError.Success)
            {
                return;
            }
            listener.Listen(1);
            accepted = listener.Accept();
            netStream = new NetworkStream(accepted, true);
            corrupt = new CorruptingStream(netStream);
            tls = new TlsStream(corrupt, false);
            var opts = new TlsServerOptions();
            opts.ServerName = "tamper.test";
            opts.CertificateChain = TestCertificate();
            tls.AuthenticateAsServerAsync(opts, CoreIntrinsics.DefaultValue<CancellationToken>());
            corrupt.CorruptNext = true;
            var buffer = new byte[2];
            let _ = tls.Read(Span<byte>.FromArray(ref buffer));
        }
        catch (TlsAlertException)
        {
            _tampered = true;
        }
        catch (Std.Exception)
        { }
        finally
        {
            if (tls != null)
            {
                tls.Dispose();
            }
            else if (corrupt != null)
            {
                corrupt.Dispose();
            }
            if (accepted != null)
            {
                accepted.Close();
            }
            if (listener != null)
            {
                listener.Close();
            }
        }
    }
}
