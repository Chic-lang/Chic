namespace Chic.Web;
import Std.Strings;
import Std.Span;
import Std.Numeric;
/// <summary>Represents a bound route entry for a specific HTTP method.</summary>
public sealed class RouteEndpoint
{
    private string _method;
    private RouteTemplate _template;
    private RequestDelegate _handler;
    public init(string method, RouteTemplate template, RequestDelegate handler) {
        _method = NormalizeMethod(method);
        _template = template;
        _handler = handler;
    }
    public RequestDelegate Handler => _handler;
    public bool TryMatch(HttpRequest request, out RouteValues values) {
        let candidateMethod = NormalizeMethod(request.Method);
        if (candidateMethod != _method)
        {
            values = new RouteValues();
            return false;
        }
        return _template.TryMatch(request.Path, out values);
    }
    private static string NormalizeMethod(string method) {
        if (method == null)
        {
            return "";
        }
        let utf8 = method.AsUtf8Span();
        var buf = new byte[utf8.Length];
        var idx = 0usize;
        while (idx <utf8.Length)
        {
            var b = utf8[idx];
            if (b >= NumericUnchecked.ToByte ('a') && b <= NumericUnchecked.ToByte ('z'))
            {
                b = NumericUnchecked.ToByte(b - NumericUnchecked.ToByte(32));
            }
            buf[idx] = b;
            idx += 1usize;
        }
        return Utf8String.FromSpan(ReadOnlySpan <byte >.FromArray(in buf));
    }
}
