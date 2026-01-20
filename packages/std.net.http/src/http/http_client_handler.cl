namespace Std.Net.Http;
import Std.Async;
import Std.Core;
import Std.Datetime;
import Std.IO;
import Std.Net;
import Std.Net.Sockets;
import Std.Security.Tls;
import Std.Security.Certs;
import Std.Span;
import Std.Numeric;
import Std.Collections;
import Std.Platform.Time;
internal struct PooledConnection
{
    public Std.Net.Sockets.Socket Socket;
    public NetworkStream ?PlainStream;
    public TlsStream ?SecureStream;
    public bool IsTls;
    public init(Std.Net.Sockets.Socket socket, NetworkStream stream, bool isTls) {
        Socket = socket;
        PlainStream = stream;
        SecureStream = null;
        IsTls = isTls;
    }
    public init(Std.Net.Sockets.Socket socket, TlsStream stream) {
        Socket = socket;
        PlainStream = null;
        SecureStream = stream;
        IsTls = true;
    }
    public int Read(Span <byte >buffer) {
        if (IsTls)
        {
            let tls = SecureStream;
            if (tls == null)
            {
                return 0;
            }
            return tls.Read(buffer);
        }
        let plain = PlainStream;
        if (plain == null)
        {
            return 0;
        }
        return plain.Read(buffer);
    }
    public void Write(ReadOnlySpan <byte >buffer) {
        if (IsTls)
        {
            let tls = SecureStream;
            if (tls == null)
            {
                return;
            }
            tls.Write(buffer);
            return;
        }
        let plain = PlainStream;
        if (plain == null)
        {
            return;
        }
        plain.Write(buffer);
    }
    public void Flush() {
        if (IsTls)
        {
            let tls = SecureStream;
            if (tls == null)
            {
                return;
            }
            tls.Flush();
            return;
        }
        let plain = PlainStream;
        if (plain == null)
        {
            return;
        }
        plain.Flush();
    }
    public void DisposeStream() {
        if (IsTls)
        {
            let tls = SecureStream;
            if (tls != null)
            {
                tls.Dispose();
            }
            return;
        }
        let plain = PlainStream;
        if (plain != null)
        {
            plain.Dispose();
        }
    }
}
public class HttpClientHandler : HttpMessageHandler
{
    private HashMap <string, PooledConnection >_pool;
    private CancellationToken _globalToken;
    public Duration Timeout {
        get;
        set;
    }
    public long MaxResponseContentBufferSize {
        get;
        set;
    }
    public bool AllowUntrustedCertificates {
        get;
        set;
    }
    public string[] TrustedRootCertificates {
        get;
        set;
    }
    public TlsProtocol[] EnabledProtocols {
        get;
        set;
    }
    public string[] ApplicationProtocols {
        get;
        set;
    }
    public init() {
        _pool = new HashMap <string, PooledConnection >();
        _globalToken = CoreIntrinsics.DefaultValue <CancellationToken >();
        Timeout = Duration.FromSeconds(100);
        MaxResponseContentBufferSize = 1024 * 1024;
        AllowUntrustedCertificates = false;
        TrustedRootCertificates = new string[0];
        EnabledProtocols = DefaultProtocols();
        ApplicationProtocols = DefaultAlpn();
    }
    protected override Task <HttpResponseMessage >SendAsync(HttpRequestMessage request, HttpCompletionOption completion,
    CancellationToken ct) {
        if (request == null || request.RequestUri == null)
        {
            throw new HttpRequestException("RequestUri must be provided");
        }
        if (request.Version.Major >1 && request.VersionPolicy != HttpVersionPolicy.RequestVersionOrLower)
        {
            throw new HttpRequestException("HTTP/2 and HTTP/3 are not implemented yet");
        }
        let startNs = Time.MonotonicNanoseconds();
        let timeoutNs = Timeout.Ticks >0 ?NumericUnchecked.ToUInt64(Timeout.Ticks) * 100ul : 0ul;
        CheckCancellation(ct, startNs, timeoutNs);
        let uri = request.RequestUri;
        let scheme = uri.Scheme;
        let host = uri.Host;
        let port = uri.Port;
        let isHttps = scheme != null && EqualsIgnoreCase(scheme, "https");
        let path = uri.PathAndQuery;
        if (path == null || path.Length == 0)
        {
            path = "/";
        }
        let hostHeader = BuildHostHeader(uri);
        let poolKey = BuildPoolKey(scheme, host, port);
        if (! TryRentConnection (poolKey, out var connection)) {
            connection = OpenConnection(host, port, isHttps, ct, startNs, timeoutNs);
        }
        try {
            let content = request.Content != null ?request.Content.GetBytes() : new byte[0];
            WriteRequest(connection, request, hostHeader, path, content, startNs, timeoutNs, ct);
            let allowReuse = ! HasToken(request.Headers, "Connection", "close");
            let response = ReadResponse(connection, completion, content.Length, allowReuse, poolKey, startNs, timeoutNs,
            ct);
            return TaskRuntime.FromResult(response);
        }
        catch(Std.TaskCanceledException) {
            CloseConnection(connection);
            throw;
        }
        catch(Std.Exception) {
            CloseConnection(connection);
            throw;
        }
    }
    internal void SetGlobalToken(CancellationToken token) {
        _globalToken = token;
    }
    internal void ReturnSocket(string poolKey, Std.Net.Sockets.Socket socket) {
        if (socket == null || ! socket.IsValid)
        {
            return;
        }
        var stream = new NetworkStream(socket, true);
        StoreConnection(poolKey, new PooledConnection(socket, stream, false));
    }
    private PooledConnection OpenConnection(string host, int port, bool useTls, CancellationToken ct, ulong startNs, ulong timeoutNs) {
        CheckCancellation(ct, startNs, timeoutNs);
        var socket = new Std.Net.Sockets.Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
        let addresses = Dns.GetHostAddresses(host);
        if (addresses == null || addresses.Length == 0)
        {
            throw new HttpRequestException("DNS resolution failed");
        }
        let status = socket.Connect(addresses[0], port);
        if (status != SocketError.Success)
        {
            socket.Close();
            throw new HttpRequestException("Unable to connect to host");
        }
        var stream = new NetworkStream(socket, false);
        if (useTls)
        {
            var tls = new TlsStream(stream, false);
            try {
                var opts = new TlsClientOptions();
                opts.ServerName = host;
                opts.AllowUntrustedCertificates = AllowUntrustedCertificates;
                opts.TrustedRootFiles = TrustedRootCertificates;
                opts.EnabledProtocols = EnabledProtocols;
                opts.ApplicationProtocols = ApplicationProtocols;
                tls.AuthenticateAsClientAsync(opts, ct);
                let alpn = tls.ApplicationProtocol;
                if (alpn != null && alpn.Length >0 && ! EqualsIgnoreCase (alpn, "http/1.1"))
                {
                    throw new HttpRequestException("Negotiated unsupported protocol: " + alpn);
                }
            }
            catch(Std.Exception) {
                socket.Close();
                throw;
            }
            return new PooledConnection(socket, tls);
        }
        return new PooledConnection(socket, stream, useTls);
    }
    private void WriteRequest(PooledConnection connection, HttpRequestMessage request, string hostHeader, string path, byte[] content,
    ulong startNs, ulong timeoutNs, CancellationToken ct) {
        CheckCancellation(ct, startNs, timeoutNs);
        let keepAlive = ! request.Headers.Contains("Connection") || ! HasToken(request.Headers, "Connection", "close");
        var line = request.Method.Method + " " + path + " HTTP/1.1\r\n";
        WriteAscii(connection, line);
        WriteAscii(connection, "Host: ");
        WriteAscii(connection, hostHeader);
        WriteAscii(connection, "\r\n");
        WriteAscii(connection, "Connection: ");
        WriteAscii(connection, keepAlive ?"keep-alive" : "close");
        WriteAscii(connection, "\r\n");
        var hasContentLengthHeader = request.Headers.Contains("Content-Length");
        if (!hasContentLengthHeader && request.Content != null)
        {
            let contentHeaders = request.Content.Headers;
            hasContentLengthHeader = contentHeaders.Contains("Content-Length");
        }
        if (content.Length >0 && ! hasContentLengthHeader)
        {
            WriteAscii(connection, "Content-Length: ");
            WriteAscii(connection, content.Length.ToString());
            WriteAscii(connection, "\r\n");
        }
        WriteHeaders(connection, request.Headers, new string[] {
            "host", "connection", "content-length"
        }
        );
        if (request.Content != null)
        {
            WriteHeaders(connection, request.Content.Headers, new string[] {
                "content-length"
            }
            );
        }
        WriteAscii(connection, "\r\n");
        if (content.Length >0)
        {
            CheckCancellation(ct, startNs, timeoutNs);
            connection.Write(ReadOnlySpan <byte >.FromArray(ref content));
        }
        connection.Flush();
    }
    private HttpResponseMessage ReadResponse(PooledConnection connection, HttpCompletionOption completion, int requestContentLength, bool allowReuse,
    string poolKey, ulong startNs, ulong timeoutNs, CancellationToken ct) {
        var statusLine = ReadLine(connection, startNs, timeoutNs, ct);
        if (statusLine.Length == 0)
        {
            throw new HttpRequestException("empty response");
        }
        ParseStatusLine(statusLine, out var version, out var statusCode, out var reason);
        var headers = new HttpResponseHeaders();
        var connectionClose = ! allowReuse || (version.Major == 1 && version.Minor == 0);
        var contentLength = 0L;
        var hasContentLength = false;
        while (true)
        {
            let line = ReadLine(connection, startNs, timeoutNs, ct);
            if (line.Length == 0)
            {
                break;
            }
            let colon = line.IndexOf(":");
            if (colon <= 0)
            {
                continue;
            }
            let name = line.Substring(0, colon);
            let value = Trim(line.Substring(colon + 1));
            headers.Set(name, value);
            if (EqualsIgnoreCase (name, "connection"))
            {
                if (ContainsToken (value, "close"))
                {
                    connectionClose = true;
                }
            }
            if (EqualsIgnoreCase (name, "content-length"))
            {
                hasContentLength = true;
                contentLength = ParseLong(value);
            }
        }
        var body = new byte[0];
        if (hasContentLength && contentLength >0)
        {
            EnforceBufferLimit(contentLength);
            body = ReadFixed(connection, contentLength, startNs, timeoutNs, ct);
        }
        else if (hasContentLength && contentLength == 0)
        {
            body = new byte[0];
        }
        else
        {
            body = ReadToEnd(connection, startNs, timeoutNs, ct);
            EnforceBufferLimit(NumericUnchecked.ToInt64(body.Length));
            connectionClose = true;
        }
        var response = new HttpResponseMessage();
        response.StatusCode = (HttpStatusCode) statusCode;
        response.ReasonPhrase = reason;
        response.Headers = headers;
        response.Version = version;
        response.Content = new ByteArrayContent(body);
        response.Content.Headers.Set("Content-Length", body.Length.ToString());
        if (! connectionClose && ! _globalToken.IsCancellationRequested ())
        {
            StoreConnection(poolKey, connection);
        }
        else
        {
            CloseConnection(connection);
        }
        return response;
    }
    private void StoreConnection(string poolKey, PooledConnection connection) {
        if (poolKey == null || connection.Socket == null || ! connection.Socket.IsValid)
        {
            return;
        }
        _pool.Insert(poolKey, connection, out var previous);
        if (previous.IsSome (out var old)) {
            CloseConnection(old);
        }
    }
    private bool TryRentConnection(string poolKey, out PooledConnection connection) {
        connection = CoreIntrinsics.DefaultValue <PooledConnection >();
        let entry = _pool.Get(poolKey);
        if (entry.IsSome (out var pooled)) {
            _pool.Remove(poolKey);
            if (pooled.Socket != null && pooled.Socket.IsValid)
            {
                connection = pooled;
                return true;
            }
        }
        return false;
    }
    private void CloseConnection(PooledConnection connection) {
        connection.DisposeStream();
        if (connection.Socket != null)
        {
            connection.Socket.Close();
        }
    }
    private string ReadLine(PooledConnection connection, ulong startNs, ulong timeoutNs, CancellationToken ct) {
        var buffer = new byte[256];
        var length = 0usize;
        var seenCr = false;
        var temp = new byte[1];
        while (true)
        {
            CheckCancellation(ct, startNs, timeoutNs);
            let read = connection.Read(Span <byte >.FromArray(ref temp));
            if (read == 0)
            {
                break;
            }
            let b = temp[0];
            if (seenCr && b == 10u8)
            {
                break;
            }
            if (b == 13u8)
            {
                seenCr = true;
                continue;
            }
            if (length >= buffer.Length)
            {
                let larger = new byte[buffer.Length * 2];
                ReadOnlySpan <byte >.FromArray(ref buffer).CopyTo(Span <byte >.FromArray(ref larger));
                buffer = larger;
            }
            buffer[length] = b;
            length += 1usize;
            seenCr = false;
        }
        if (length == 0usize)
        {
            return "";
        }
        return Utf8String.FromSpan(ReadOnlySpan <byte >.FromArray(ref buffer).Slice(0usize, length));
    }
    private byte[] ReadFixed(PooledConnection connection, long length, ulong startNs, ulong timeoutNs, CancellationToken ct) {
        if (length <0)
        {
            throw new HttpRequestException("invalid content length");
        }
        var remaining = length;
        var buffer = new byte[length];
        var offset = 0usize;
        while (remaining >0)
        {
            CheckCancellation(ct, startNs, timeoutNs);
            let toRead = remaining >8192 ?8192 : NumericUnchecked.ToInt32(remaining);
            let span = Span <byte >.FromArray(ref buffer).Slice(offset, NumericUnchecked.ToUSize(toRead));
            let read = connection.Read(span);
            if (read == 0)
            {
                throw new HttpRequestException("incomplete HTTP response body");
            }
            remaining -= read;
            offset += NumericUnchecked.ToUSize(read);
        }
        return buffer;
    }
    private byte[] ReadToEnd(PooledConnection connection, ulong startNs, ulong timeoutNs, CancellationToken ct) {
        var chunk = new byte[1024];
        var output = new byte[0];
        var total = 0usize;
        while (true)
        {
            CheckCancellation(ct, startNs, timeoutNs);
            let read = connection.Read(Span <byte >.FromArray(ref chunk));
            if (read == 0)
            {
                break;
            }
            let nextLength = total + NumericUnchecked.ToUSize(read);
            var next = new byte[nextLength];
            if (total >0usize)
            {
                ReadOnlySpan <byte >.FromArray(ref output).CopyTo(Span <byte >.FromArray(ref next));
            }
            ReadOnlySpan <byte >.FromArray(ref chunk).Slice(0usize, NumericUnchecked.ToUSize(read)).CopyTo(Span <byte >.FromArray(ref next).Slice(total,
            NumericUnchecked.ToUSize(read)));
            output = next;
            total = nextLength;
            EnforceBufferLimit(NumericUnchecked.ToInt64(total));
        }
        return output;
    }
    private void ParseStatusLine(string line, out Std.Version version, out int statusCode, out string reason) {
        version = new Std.Version(1, 1);
        statusCode = 0;
        reason = "";
        let firstSpace = line.IndexOf(" ");
        if (firstSpace <= 0)
        {
            throw new HttpRequestException("invalid HTTP status line");
        }
        let secondSpace = line.IndexOf(" ", firstSpace + 1);
        let proto = line.Substring(0, firstSpace);
        if (proto.StartsWith ("HTTP/"))
        {
            let verText = proto.Substring(5);
            let dot = verText.IndexOf(".");
            if (dot >0)
            {
                let majorText = verText.Substring(0, dot);
                let minorText = verText.Substring(dot + 1);
                version = new Std.Version(ParseInt(majorText), ParseInt(minorText));
            }
        }
        if (secondSpace > firstSpace + 1)
        {
            statusCode = ParseInt(line.Substring(firstSpace + 1, secondSpace - firstSpace - 1));
            if (secondSpace + 1 < line.Length)
            {
                reason = line.Substring(secondSpace + 1);
            }
            return;
        }
        statusCode = ParseInt(line.Substring(firstSpace + 1));
    }
    private static string Trim(string value) {
        if (value == null)
        {
            return "";
        }
        var start = 0;
        var end = value.Length - 1;
        while (start <= end && IsSpace (value[start]))
        {
            start += 1;
        }
        while (end >= start && IsSpace (value[end]))
        {
            end -= 1;
        }
        if (start >end)
        {
            return "";
        }
        return value.Substring(start, end - start + 1);
    }
    private static bool IsSpace(char value) {
        return value == ' ' || value == '\t' || value == '\r' || value == '\n';
    }
    private static bool EqualsIgnoreCase(string left, string right) {
        if (left == null || right == null)
        {
            return false;
        }
        if (left.Length != right.Length)
        {
            return false;
        }
        let a = left.AsUtf8Span();
        let b = right.AsUtf8Span();
        var idx = 0usize;
        while (idx <a.Length)
        {
            var la = ToLower(a[idx]);
            var lb = ToLower(b[idx]);
            if (la != lb)
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
    private static bool ContainsToken(string value, string token) {
        if (value == null || token == null)
        {
            return false;
        }
        let hay = value.AsUtf8Span();
        let needle = token.AsUtf8Span();
        var idx = 0usize;
        while (idx + needle.Length <= hay.Length)
        {
            let slice = hay.Slice(idx, needle.Length);
            if (EqualsIgnoreCase (Utf8String.FromSpan (slice), token))
            {
                return true;
            }
            idx += 1usize;
        }
        return false;
    }
    private static byte ToLower(byte value) {
        if (value >= NumericUnchecked.ToByte ('A') && value <= NumericUnchecked.ToByte ('Z'))
        {
            return NumericUnchecked.ToByte(value + NumericUnchecked.ToByte(32));
        }
        return value;
    }
    private static int ParseInt(string value) {
        var result = 0;
        var idx = 0;
        while (idx <value.Length)
        {
            let ch = value[idx];
            if (ch <'0' || ch >'9')
            {
                break;
            }
            result = result * 10 + NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(ch) - NumericUnchecked.ToUInt32('0'));
            idx += 1;
        }
        return result;
    }
    private static long ParseLong(string value) {
        var result = 0L;
        var idx = 0;
        while (idx <value.Length)
        {
            let ch = value[idx];
            if (ch <'0' || ch >'9')
            {
                break;
            }
            result = result * 10L + NumericUnchecked.ToInt64(NumericUnchecked.ToUInt32(ch) - NumericUnchecked.ToUInt32('0'));
            idx += 1;
        }
        return result;
    }
    private static string BuildHostHeader(Std.Uri uri) {
        let host = uri.Host;
        if (! uri.IsDefaultPort)
        {
            return host + ":" + uri.Port.ToString();
        }
        return host;
    }
    private static string BuildPoolKey(string scheme, string host, int port) {
        return scheme + "://" + host + ":" + port.ToString();
    }
    private static void WriteAscii(PooledConnection connection, string text) {
        let utf8 = text.AsUtf8Span();
        connection.Write(utf8);
    }
    private void WriteHeaders(PooledConnection connection, HttpHeaders headers, string[] skip) {
        if (headers == null)
        {
            return;
        }
        let iter = headers.Iterate();
        while (iter.Next (out var name, out var value)) {
            if (ShouldSkip (name, skip))
            {
                continue;
            }
            WriteAscii(connection, name);
            WriteAscii(connection, ": ");
            WriteAscii(connection, value);
            WriteAscii(connection, "\r\n");
        }
    }
    private static bool ShouldSkip(string name, string[] skip) {
        if (skip == null || name == null)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <skip.Length)
        {
            if (EqualsIgnoreCase (name, skip[idx]))
            {
                return true;
            }
            idx += 1usize;
        }
        return false;
    }
    private void CheckCancellation(CancellationToken token, ulong startNs, ulong timeoutNs) {
        if (_globalToken.IsCancellationRequested () || token.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Request canceled");
        }
        if (timeoutNs >0ul)
        {
            let elapsed = Time.MonotonicNanoseconds() - startNs;
            if (elapsed >timeoutNs)
            {
                throw new Std.TaskCanceledException("Request timed out");
            }
        }
    }
    private void EnforceBufferLimit(long length) {
        if (MaxResponseContentBufferSize <0)
        {
            return;
        }
        if (length >MaxResponseContentBufferSize)
        {
            throw new HttpRequestException("response content exceeded buffer limit");
        }
    }
    private static TlsProtocol[] DefaultProtocols() {
        var protocols = new TlsProtocol[2];
        protocols[0] = TlsProtocol.Tls13;
        protocols[1] = TlsProtocol.Tls12;
        return protocols;
    }
    private static string[] DefaultAlpn() {
        var protocols = new string[1];
        protocols[0] = "http/1.1";
        return protocols;
    }
    private static bool HasToken(HttpHeaders headers, string name, string token) {
        if (headers == null || name == null || token == null)
        {
            return false;
        }
        if (! headers.TryGetValue (name, out var value)) {
            return false;
        }
        return ContainsToken(value, token);
    }
}
