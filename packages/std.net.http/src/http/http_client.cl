namespace Std.Net.Http;
import Std.Async;
import Std.Datetime;
import Std.Core;
import Std.Strings;
import Std.Span;
import Std.Numeric;
import Std.IO.Compression;
/// <summary>
/// High-level HTTP client compatible with the bootstrap transport pipeline.
/// </summary>
public sealed class HttpClient : HttpMessageInvoker
{
    private Std.Uri ?_baseAddress;
    private HttpRequestHeaders _defaultRequestHeaders;
    private Std.Version _defaultRequestVersion;
    private HttpVersionPolicy _defaultVersionPolicy;
    private long _maxResponseContentBufferSize;
    private Duration _timeout;
    private Std.Net.IWebProxy ?_defaultProxy;
    private Std.Async.CancellationTokenSource _pendingCts;
    private HttpClientHandler ?_clientHandler;
    public init() : self(new HttpClientHandler(), true) {
    }
    public init(HttpClientHandler handler) : self(handler, true) {
    }
    public init(HttpMessageHandler handler) : self(handler, true) {
    }
    public init(HttpClientHandler handler, bool disposeHandler) : base(handler, disposeHandler) {
        _clientHandler = handler;
        InitDefaults();
    }
    public init(HttpMessageHandler handler, bool disposeHandler) : base(handler, disposeHandler) {
        _clientHandler = null;
        InitDefaults();
    }
    private void InitDefaults() {
        _defaultRequestHeaders = new HttpRequestHeaders();
        _defaultRequestVersion = new Std.Version(1, 1);
        _defaultVersionPolicy = HttpVersionPolicy.RequestVersionOrLower;
        _maxResponseContentBufferSize = 1024 * 1024;
        // 1 MB default
        _timeout = Duration.FromSeconds(100);
        _defaultProxy = null;
        _pendingCts = Std.Async.CancellationTokenSource.Create();
    }
    public Std.Uri ?BaseAddress {
        /// <summary>Base address used to resolve relative request URIs.</summary>
        get {
            return _baseAddress;
        }
        set {
            _baseAddress = value;
        }
    }
    public Std.Net.IWebProxy ?DefaultProxy {
        /// <summary>Optional proxy configuration (not currently honored by the transport).</summary>
        get {
            return _defaultProxy;
        }
        set {
            _defaultProxy = value;
        }
    }
    /// <summary>Headers applied to every request unless overridden.</summary>
    public HttpRequestHeaders DefaultRequestHeaders => _defaultRequestHeaders;
    public Std.Version DefaultRequestVersion {
        /// <summary>Default HTTP version for implicitly created requests.</summary>
        get {
            return _defaultRequestVersion;
        }
        set {
            _defaultRequestVersion = value;
        }
    }
    public HttpVersionPolicy DefaultVersionPolicy {
        /// <summary>Default policy controlling version negotiation.</summary>
        get {
            return _defaultVersionPolicy;
        }
        set {
            _defaultVersionPolicy = value;
        }
    }
    public long MaxResponseContentBufferSize {
        /// <summary>Maximum buffered response size; -1 disables the limit.</summary>
        get {
            return _maxResponseContentBufferSize;
        }
        set {
            if (value <- 1)
            {
                throw new Std.ArgumentOutOfRangeException("MaxResponseContentBufferSize");
            }
            _maxResponseContentBufferSize = value;
        }
    }
    public Duration Timeout {
        /// <summary>Total timeout for a request; Duration.Infinite disables.</summary>
        get {
            return _timeout;
        }
        set {
            if (value.Ticks <- 1)
            {
                throw new Std.ArgumentOutOfRangeException("Timeout");
            }
            _timeout = value;
            // Reset pending CTS to enforce new timeout across pooled sockets.
            _pendingCts.Cancel();
            _pendingCts = Std.Async.CancellationTokenSource.Create();
        }
    }
    public void CancelPendingRequests() {
        _pendingCts.Cancel();
        _pendingCts = Std.Async.CancellationTokenSource.Create();
    }
    private HttpRequestMessage PrepareRequest(HttpRequestMessage request) {
        if (request.RequestUri == null)
        {
            if (_baseAddress != null)
            {
                request.RequestUri = _baseAddress;
            }
            else
            {
                throw new HttpRequestException("RequestUri must be specified");
            }
        }
        else if (_baseAddress != null && !request.RequestUri.IsAbsoluteUri)
        {
            request.RequestUri = new Std.Uri(_baseAddress, request.RequestUri);
        }
        else if (!request.RequestUri.IsAbsoluteUri)
        {
            throw new HttpRequestException("Relative URIs require a BaseAddress");
        }
        if (request.Version.Major == 0 && request.Version.Minor == 0)
        {
            request.Version = _defaultRequestVersion;
        }
        if (request.VersionPolicy == HttpVersionPolicy.RequestVersionOrLower && _defaultVersionPolicy != HttpVersionPolicy.RequestVersionOrLower)
        {
            request.VersionPolicy = _defaultVersionPolicy;
        }
        // Apply default headers where missing.
        var iter = _defaultRequestHeaders.Iterate();
        while (iter.Next (out var name, out var value)) {
            if (!request.Headers.Contains (name))
            {
                request.Headers.Set(name, value);
            }
        }
        return request;
    }
    private HttpResponseMessage SendCore(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        ApplyHandlerConfiguration();
        let prepared = PrepareRequest(request);
        let response = base.Send(prepared, completionOption, cancellationToken);
        ApplyDecompression(response);
        return response;
    }
    private Task <HttpResponseMessage >SendCoreAsync(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        ApplyHandlerConfiguration();
        let prepared = PrepareRequest(request);
        let task = base.SendAsync(prepared, completionOption, cancellationToken);
        let response = TaskRuntime.GetResult(task);
        ApplyDecompression(response);
        return TaskRuntime.FromResult(response);
    }
    private void ApplyHandlerConfiguration() {
        let concrete = _clientHandler;
        if (concrete == null)
        {
            return;
        }
        concrete.Timeout = _timeout;
        concrete.MaxResponseContentBufferSize = _maxResponseContentBufferSize;
        concrete.SetGlobalToken(_pendingCts.Token());
    }
    private void ApplyDecompression(HttpResponseMessage response) {
        if (response == null || response.Content == null)
        {
            return;
        }
        if (!response.Content.Headers.TryGetValue ("Content-Encoding", out var encoding) || encoding == null) {
            return;
        }
        let compressed = response.Content.ReadAsByteArray();
        var output = new byte[compressed.Length * 4 + 64];
        var written = 0;
        var handled = false;
        if (ContainsTokenIgnoreCase (encoding, "gzip"))
        {
            handled = TryExpandGzip(compressed, ref output, out written);
        }
        else if (ContainsTokenIgnoreCase (encoding, "deflate"))
        {
            handled = TryExpandDeflate(compressed, ref output, out written);
        }
        if (!handled)
        {
            return;
        }
        var payload = output;
        if (written != payload.Length)
        {
            var trimmed = new byte[written];
            if (written >0)
            {
                Span <byte >.FromArray(ref trimmed).Slice(0usize, NumericUnchecked.ToUSize(written)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref payload).Slice(0usize,
                NumericUnchecked.ToUSize(written)));
            }
            payload = trimmed;
        }
        response.Content = new ByteArrayContent(payload);
        response.Content.Headers.Remove("Content-Encoding");
        response.Content.Headers.Set("Content-Length", written.ToString());
    }
    private static bool ContainsTokenIgnoreCase(string value, string token) {
        if (value == null || token == null)
        {
            return false;
        }
        let hay = value.AsSpan();
        let needle = token.AsSpan();
        if (needle.Length == 0usize || hay.Length <needle.Length)
        {
            return false;
        }
        let limit = hay.Length - needle.Length;
        var i = 0usize;
        while (i <= limit)
        {
            var match = true;
            var j = 0usize;
            while (j <needle.Length)
            {
                if (ToLowerAscii (hay[i + j]) != ToLowerAscii (needle[j]))
                {
                    match = false;
                    break;
                }
                j += 1usize;
            }
            if (match)
            {
                return true;
            }
            i += 1usize;
        }
        return false;
    }
    private static char ToLowerAscii(char value) {
        if (value >= 'A' && value <= 'Z')
        {
            return(char)(value + 32);
        }
        return value;
    }
    private static bool TryExpandGzip(byte[] data, ref byte[] output, out int written) {
        written = 0;
        while (true)
        {
            if (GZip.TryDecompress (ReadOnlySpan <byte >.FromArray (ref data), Span <byte >.FromArray(ref output), out written)) {
                return true;
            }
            var larger = new byte[output.Length * 2 + 64];
            output = larger;
            if (output.Length >data.Length * 32 + 1024)
            {
                return false;
            }
        }
    }
    private static bool TryExpandDeflate(byte[] data, ref byte[] output, out int written) {
        written = 0;
        while (true)
        {
            if (Deflate.TryDecompress (ReadOnlySpan <byte >.FromArray (ref data), Span <byte >.FromArray(ref output), out written)) {
                return true;
            }
            var larger = new byte[output.Length * 2 + 64];
            output = larger;
            if (output.Length >data.Length * 32 + 1024)
            {
                return false;
            }
        }
    }
    // Sync send overloads
    public HttpResponseMessage Send(HttpRequestMessage request) => SendCore(request, HttpCompletionOption.ResponseContentRead,
    CoreIntrinsics.DefaultValue <CancellationToken >());
    public HttpResponseMessage Send(HttpRequestMessage request, CancellationToken cancellationToken) => SendCore(request,
    HttpCompletionOption.ResponseContentRead, cancellationToken);
    public HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption) => SendCore(request,
    completionOption, CoreIntrinsics.DefaultValue <CancellationToken >());
    public HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) => SendCore(request,
    completionOption, cancellationToken);
    // Async send overloads
    public Task <HttpResponseMessage >SendAsync(HttpRequestMessage request) => SendCoreAsync(request, HttpCompletionOption.ResponseContentRead,
    CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) => SendCoreAsync(request,
    HttpCompletionOption.ResponseContentRead, cancellationToken);
    public Task <HttpResponseMessage >SendAsync(HttpRequestMessage request, HttpCompletionOption completionOption) => SendCoreAsync(request,
    completionOption, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >SendAsync(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) => SendCoreAsync(request,
    completionOption, cancellationToken);
    // Convenience helpers (string overloads)
    public Task <HttpResponseMessage >GetAsync(string uri) => GetAsync(new Std.Uri(uri), HttpCompletionOption.ResponseContentRead,
    CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >GetAsync(string uri, CancellationToken ct) => GetAsync(new Std.Uri(uri), HttpCompletionOption.ResponseContentRead,
    ct);
    public Task <HttpResponseMessage >GetAsync(string uri, HttpCompletionOption option) => GetAsync(new Std.Uri(uri), option,
    CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >GetAsync(string uri, HttpCompletionOption option, CancellationToken ct) => GetAsync(new Std.Uri(uri),
    option, ct);
    public Task <HttpResponseMessage >PostAsync(string uri, HttpContent content) => PostAsync(new Std.Uri(uri), content,
    CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >PostAsync(string uri, HttpContent content, CancellationToken ct) => PostAsync(new Std.Uri(uri),
    content, ct);
    public Task <HttpResponseMessage >PutAsync(string uri, HttpContent content) => PutAsync(new Std.Uri(uri), content, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >PutAsync(string uri, HttpContent content, CancellationToken ct) => PutAsync(new Std.Uri(uri),
    content, ct);
    public Task <HttpResponseMessage >DeleteAsync(string uri) => DeleteAsync(new Std.Uri(uri), CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >DeleteAsync(string uri, CancellationToken ct) => DeleteAsync(new Std.Uri(uri), ct);
    /// <summary>Sends a HEAD request to the specified URI.</summary>
    public Task <HttpResponseMessage >HeadAsync(string uri) => HeadAsync(new Std.Uri(uri), CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Sends a HEAD request with cancellation.</summary>
    public Task <HttpResponseMessage >HeadAsync(string uri, CancellationToken ct) => HeadAsync(new Std.Uri(uri), ct);
    /// <summary>Sends an OPTIONS request to the specified URI.</summary>
    public Task <HttpResponseMessage >OptionsAsync(string uri) => OptionsAsync(new Std.Uri(uri), CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Sends an OPTIONS request with cancellation.</summary>
    public Task <HttpResponseMessage >OptionsAsync(string uri, CancellationToken ct) => OptionsAsync(new Std.Uri(uri), ct);
    public Task <HttpResponseMessage >PatchAsync(string uri, HttpContent content) => PatchAsync(new Std.Uri(uri), content,
    CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >PatchAsync(string uri, HttpContent content, CancellationToken ct) => PatchAsync(new Std.Uri(uri),
    content, ct);
    // Convenience helpers (Uri overloads)
    public Task <HttpResponseMessage >GetAsync(Std.Uri uri) => GetAsync(uri, HttpCompletionOption.ResponseContentRead, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >GetAsync(Std.Uri uri, CancellationToken ct) => GetAsync(uri, HttpCompletionOption.ResponseContentRead,
    ct);
    public Task <HttpResponseMessage >GetAsync(Std.Uri uri, HttpCompletionOption option) => GetAsync(uri, option, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >GetAsync(Std.Uri uri, HttpCompletionOption option, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Get, uri);
        return SendCoreAsync(request, option, ct);
    }
    public Task <HttpResponseMessage >PostAsync(Std.Uri uri, HttpContent content) => PostAsync(uri, content, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >PostAsync(Std.Uri uri, HttpContent content, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Post, uri);
        request.Content = content;
        return SendCoreAsync(request, HttpCompletionOption.ResponseContentRead, ct);
    }
    public Task <HttpResponseMessage >PutAsync(Std.Uri uri, HttpContent content) => PutAsync(uri, content, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >PutAsync(Std.Uri uri, HttpContent content, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Put, uri);
        request.Content = content;
        return SendCoreAsync(request, HttpCompletionOption.ResponseContentRead, ct);
    }
    public Task <HttpResponseMessage >DeleteAsync(Std.Uri uri) => DeleteAsync(uri, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >DeleteAsync(Std.Uri uri, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Delete, uri);
        return SendCoreAsync(request, HttpCompletionOption.ResponseContentRead, ct);
    }
    /// <summary>Sends a HEAD request to a Uri.</summary>
    public Task <HttpResponseMessage >HeadAsync(Std.Uri uri) => HeadAsync(uri, CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Sends a HEAD request with cancellation.</summary>
    public Task <HttpResponseMessage >HeadAsync(Std.Uri uri, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Head, uri);
        return SendCoreAsync(request, HttpCompletionOption.ResponseHeadersRead, ct);
    }
    /// <summary>Sends an OPTIONS request to a Uri.</summary>
    public Task <HttpResponseMessage >OptionsAsync(Std.Uri uri) => OptionsAsync(uri, CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Sends an OPTIONS request with cancellation.</summary>
    public Task <HttpResponseMessage >OptionsAsync(Std.Uri uri, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Options, uri);
        return SendCoreAsync(request, HttpCompletionOption.ResponseContentRead, ct);
    }
    public Task <HttpResponseMessage >PatchAsync(Std.Uri uri, HttpContent content) => PatchAsync(uri, content, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <HttpResponseMessage >PatchAsync(Std.Uri uri, HttpContent content, CancellationToken ct) {
        var request = new HttpRequestMessage(HttpMethod.Patch, uri);
        request.Content = content;
        return SendCoreAsync(request, HttpCompletionOption.ResponseContentRead, ct);
    }
    // Convenience data helpers
    public Task <string >GetStringAsync(string uri) => GetStringAsync(new Std.Uri(uri), CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <string >GetStringAsync(string uri, CancellationToken ct) => GetStringAsync(new Std.Uri(uri), ct);
    public Task <string >GetStringAsync(Std.Uri uri) => GetStringAsync(uri, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <string >GetStringAsync(Std.Uri uri, CancellationToken ct) {
        let responseTask = GetAsync(uri, HttpCompletionOption.ResponseContentRead, ct);
        let response = TaskRuntime.GetResult(responseTask);
        if (response.Content == null)
        {
            return TaskRuntime.FromResult(Std.Runtime.StringRuntime.Create());
        }
        let text = response.Content.ReadAsString();
        return TaskRuntime.FromResult(text);
    }
    public Task <byte[] >GetByteArrayAsync(string uri) => GetByteArrayAsync(new Std.Uri(uri), CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <byte[] >GetByteArrayAsync(string uri, CancellationToken ct) => GetByteArrayAsync(new Std.Uri(uri), ct);
    public Task <byte[] >GetByteArrayAsync(Std.Uri uri) => GetByteArrayAsync(uri, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <byte[] >GetByteArrayAsync(Std.Uri uri, CancellationToken ct) {
        let responseTask = GetAsync(uri, HttpCompletionOption.ResponseContentRead, ct);
        let response = TaskRuntime.GetResult(responseTask);
        if (response.Content == null)
        {
            let empty = 0;
            return TaskRuntime.FromResult(new byte[empty]);
        }
        let bytes = response.Content.ReadAsByteArray();
        return TaskRuntime.FromResult(bytes);
    }
    // Stream convenience (returns buffered content as byte array wrapped in a simple stream substitute)
    public Task <Std.Net.Http.Internal.BufferedStream >GetStreamAsync(string uri) => GetStreamAsync(new Std.Uri(uri), CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <Std.Net.Http.Internal.BufferedStream >GetStreamAsync(string uri, CancellationToken ct) => GetStreamAsync(new Std.Uri(uri),
    ct);
    public Task <Std.Net.Http.Internal.BufferedStream >GetStreamAsync(Std.Uri uri) => GetStreamAsync(uri, CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task <Std.Net.Http.Internal.BufferedStream >GetStreamAsync(Std.Uri uri, CancellationToken ct) {
        let responseTask = GetAsync(uri, HttpCompletionOption.ResponseContentRead, ct);
        let response = TaskRuntime.GetResult(responseTask);
        if (response.Content == null)
        {
            let emptyLength = 0;
            let empty = new byte[emptyLength];
            return TaskRuntime.FromResult(new Std.Net.Http.Internal.BufferedStream(empty));
        }
        let bytes = response.Content.ReadAsByteArray();
        return TaskRuntime.FromResult(new Std.Net.Http.Internal.BufferedStream(bytes));
    }
}
