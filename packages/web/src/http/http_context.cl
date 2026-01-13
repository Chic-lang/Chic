namespace Chic.Web;
import Std.Async;
import Std.Collections;
/// <summary>Per-request context shared across middleware and handlers.</summary>
public sealed class HttpContext
{
    private HttpRequest _request;
    private HttpResponse _response;
    private HashMap <string, object >_items;
    private CancellationToken _aborted;
    public init(HttpRequest request, HttpResponse response, CancellationToken aborted) {
        _request = request;
        _response = response;
        _aborted = aborted;
        _items = new HashMap <string, object >();
    }
    public HttpRequest Request => _request;
    public HttpResponse Response => _response;
    public HashMap <string, object >Items => _items;
    public CancellationToken RequestAborted => _aborted;
}
