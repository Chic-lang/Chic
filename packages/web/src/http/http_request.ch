namespace Chic.Web;
import Std.IO;
import Std.Net.Http;
/// <summary>Incoming HTTP request metadata and body.</summary>
public sealed class HttpRequest
{
    private string _method;
    private string _path;
    private string _queryString;
    private HttpHeaders _headers;
    private MemoryStream _body;
    private QueryCollection _query;
    private RouteValues _routeValues;
    public init(string method, string path, string queryString, HttpHeaders headers, MemoryStream body) {
        _method = method;
        if (_method == null)
        {
            _method = "";
        }
        _path = path;
        if (_path == null || _path.Length == 0)
        {
            _path = "/";
        }
        _queryString = queryString;
        if (_queryString == null)
        {
            _queryString = "";
        }
        _headers = headers;
        if (_headers == null)
        {
            _headers = new HttpHeaders();
        }
        _body = body;
        if (_body == null)
        {
            _body = new MemoryStream();
        }
        _query = QueryCollection.Parse(_queryString);
        _routeValues = new RouteValues();
    }
    public string Method => _method;
    public string Path => _path;
    public string QueryString => _queryString;
    public HttpHeaders Headers => _headers;
    public Stream Body => _body;
    public QueryCollection Query => _query;
    public RouteValues RouteValues => _routeValues;
    internal void SetRouteValues(RouteValues values) {
        if (values == null)
        {
            _routeValues = new RouteValues();
            return;
        }
        _routeValues = values;
    }
}
