namespace Std.Net.Http;
import Std.Strings;
/// <summary>
/// HTTP method identifier (e.g., GET, POST).
/// </summary>
public sealed class HttpMethod : IEquatable <HttpMethod >
{
    private readonly string _method;
    private static HttpMethod ?_get;
    private static HttpMethod ?_post;
    private static HttpMethod ?_put;
    private static HttpMethod ?_delete;
    private static HttpMethod ?_patch;
    private static HttpMethod ?_head;
    private static HttpMethod ?_options;
    public init(string method) {
        if (method == null)
        {
            throw new Std.ArgumentNullException("method");
        }
        _method = method;
    }
    public string Method => _method;
    public static HttpMethod Get {
        get {
            if (_get == null)
            {
                _get = new HttpMethod("GET");
            }
            return _get;
        }
    }
    public static HttpMethod Post {
        get {
            if (_post == null)
            {
                _post = new HttpMethod("POST");
            }
            return _post;
        }
    }
    public static HttpMethod Put {
        get {
            if (_put == null)
            {
                _put = new HttpMethod("PUT");
            }
            return _put;
        }
    }
    public static HttpMethod Delete {
        get {
            if (_delete == null)
            {
                _delete = new HttpMethod("DELETE");
            }
            return _delete;
        }
    }
    public static HttpMethod Patch {
        get {
            if (_patch == null)
            {
                _patch = new HttpMethod("PATCH");
            }
            return _patch;
        }
    }
    public static HttpMethod Head {
        get {
            if (_head == null)
            {
                _head = new HttpMethod("HEAD");
            }
            return _head;
        }
    }
    public static HttpMethod Options {
        get {
            if (_options == null)
            {
                _options = new HttpMethod("OPTIONS");
            }
            return _options;
        }
    }
    public bool Equals(HttpMethod other) {
        if (other == null)
        {
            return false;
        }
        return _method == other._method;
    }
    public string ToString() {
        return _method;
    }
}
