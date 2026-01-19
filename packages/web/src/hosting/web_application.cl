namespace Chic.Web;
import Std.Async;
import Std.Core;
import Std.Numeric;
import Std.Strings;
import Std.Security.Tls;
/// <summary>Minimal pipeline host for chic.web applications.</summary>
public sealed class WebApplication
{
    private string _urls;
    private HttpProtocols _protocols;
    private Middleware[] _middleware;
    private int _middlewareCount;
    private RouteTable _routes;
    private bool _useTls;
    private TlsServerOptions ?_tlsOptions;
    public init(string urls, HttpProtocols protocols, TlsServerOptions ?tlsOptions) {
        _urls = urls;
        if (_urls == null || _urls.Length == 0)
        {
            _urls = "http://127.0.0.1:5000";
        }
        _protocols = protocols;
        _tlsOptions = tlsOptions;
        _middleware = new Middleware[4];
        _middlewareCount = 0;
        _routes = new RouteTable();
        // Default middleware ordering: exception handling then access log.
        Use(CreateExceptionMiddleware);
        Use(CreateAccessLogMiddleware);
    }
    public static WebApplicationBuilder CreateBuilder() {
        return new WebApplicationBuilder();
    }
    public HttpProtocols Protocols => _protocols;
    public void Use(Middleware middleware) {
        if (middleware == null)
        {
            return;
        }
        EnsureCapacity(_middlewareCount + 1);
        _middleware[_middlewareCount] = middleware;
        _middlewareCount += 1;
    }
    public void MapGet(string pattern, RequestDelegate handler) => AddRoute("GET", pattern, handler);
    public void MapPost(string pattern, RequestDelegate handler) => AddRoute("POST", pattern, handler);
    public void MapPut(string pattern, RequestDelegate handler) => AddRoute("PUT", pattern, handler);
    public void MapDelete(string pattern, RequestDelegate handler) => AddRoute("DELETE", pattern, handler);
    public void MapPatch(string pattern, RequestDelegate handler) => AddRoute("PATCH", pattern, handler);
    public Task RunAsync() => RunAsync(CoreIntrinsics.DefaultValue <CancellationToken >());
    public Task RunAsync(CancellationToken ct) {
        var pipeline = BuildPipeline();
        CreateServer().Run(pipeline, ct);
        return TaskRuntime.CompletedTask();
    }
    internal RouteTable Routes => _routes;
    private void AddRoute(string method, string pattern, RequestDelegate handler) {
        if (pattern == null)
        {
            return;
        }
        _routes.Add(method, pattern, handler);
    }
    private RequestDelegate BuildPipeline() {
        var router = new RoutingMiddleware(_routes);
        var pipeline = router.Invoke;
        var idx = _middlewareCount - 1;
        while (idx >= 0)
        {
            let middleware = _middleware[idx];
            pipeline = middleware(pipeline);
            idx -= 1;
        }
        return pipeline;
    }
    private HttpServer CreateServer() {
        ParseUrl(_urls, out var host, out var port, out var useTls);
        _useTls = useTls;
        var options = _tlsOptions;
        if (_useTls && options == null)
        {
            options = new TlsServerOptions();
        }
        if (options != null)
        {
            var serverName = options.ServerName;
            if (serverName == null || serverName.Length == 0)
            {
                options.ServerName = host;
            }
        }
        return new HttpServer(host, port, _protocols, _useTls, options);
    }
    private static RequestDelegate CreateExceptionMiddleware(RequestDelegate next) {
        var middleware = new ExceptionMiddleware(next);
        return middleware.Invoke;
    }
    private static RequestDelegate CreateAccessLogMiddleware(RequestDelegate next) {
        var middleware = new AccessLogMiddleware(next);
        return middleware.Invoke;
    }
    private void EnsureCapacity(int requiredCount) {
        if (requiredCount <= _middleware.Length)
        {
            return;
        }
        var newSize = _middleware.Length * 2;
        if (newSize <requiredCount)
        {
            newSize = requiredCount;
        }
        var next = new Middleware[newSize];
        var idx = 0;
        while (idx <_middleware.Length)
        {
            next[idx] = _middleware[idx];
            idx += 1;
        }
        _middleware = next;
    }
    private static void ParseUrl(string urlList, out string host, out int port, out bool useTls) {
        // Very small parser: pick the first entry, trim spaces, drop scheme, extract optional port.
        host = "127.0.0.1";
        port = 5000;
        useTls = false;
        var urls = urlList;
        if (urls == null || urls.Length == 0)
        {
            return;
        }
        let separator = urls.IndexOf(";");
        var first = urls;
        if (separator >= 0)
        {
            first = urls.Substring(0, separator);
        }
        if (first == null || first.Length == 0)
        {
            return;
        }
        var start = 0;
        var end = first.Length - 1;
        while (start <first.Length && IsSpace (first[start]))
        {
            start += 1;
        }
        while (end >= start && IsSpace (first[end]))
        {
            end -= 1;
        }
        if (start >end)
        {
            return;
        }
        var text = first.Substring(start, end - start + 1);
        let schemeMarker = "://";
        let schemeIdx = text.IndexOf(schemeMarker);
        if (schemeIdx >= 0)
        {
            let scheme = text.Substring(0, schemeIdx);
            useTls = scheme != null && EqualsIgnoreAsciiCase(scheme, "https");
            text = text.Substring(schemeIdx + schemeMarker.Length);
        }
        var colon = - 1;
        var idx = text.Length - 1;
        while (idx >= 0)
        {
            if (text[idx] == ':')
            {
                colon = idx;
                break;
            }
            idx -= 1;
        }
        if (colon >= 0)
        {
            let portText = text.Substring(colon + 1);
            port = ParsePort(portText, port);
            let hostText = text.Substring(0, colon);
            if (hostText.Length >0)
            {
                host = hostText;
            }
            return;
        }
        if (text.Length >0)
        {
            host = text;
        }
    }
    private static bool EqualsIgnoreAsciiCase(string left, string right) {
        if (left == null || right == null)
        {
            return false;
        }
        let leftBytes = left.AsUtf8Span();
        let rightBytes = right.AsUtf8Span();
        if (leftBytes.Length != rightBytes.Length)
        {
            return false;
        }
        var idx = 0usize;
        let upperA = NumericUnchecked.ToByte('A');
        let upperZ = NumericUnchecked.ToByte('Z');
        let delta = NumericUnchecked.ToByte(32);
        while (idx <leftBytes.Length)
        {
            var leftValue = leftBytes[idx];
            var rightValue = rightBytes[idx];
            if (leftValue >= upperA && leftValue <= upperZ)
            {
                leftValue = (byte)(leftValue + delta);
            }
            if (rightValue >= upperA && rightValue <= upperZ)
            {
                rightValue = (byte)(rightValue + delta);
            }
            if (leftValue != rightValue)
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
    private static int ParsePort(string text, int fallback) {
        if (text == null || text.Length == 0)
        {
            return fallback;
        }
        var value = 0;
        var idx = 0;
        while (idx <text.Length)
        {
            let ch = text[idx];
            if (ch <'0' || ch >'9')
            {
                return fallback;
            }
            value = value * 10 + NumericUnchecked.ToInt32(NumericUnchecked.ToUInt32(ch) - NumericUnchecked.ToUInt32('0'));
            idx += 1;
        }
        return value;
    }
    private static bool IsSpace(char value) {
        return value == ' ' || value == '\t' || value == '\r' || value == '\n';
    }
}
