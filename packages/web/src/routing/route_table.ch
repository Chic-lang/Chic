namespace Chic.Web;
import Std.Core;
/// <summary>In-memory route registry with insertion order matching.</summary>
public sealed class RouteTable
{
    private RouteEndpoint[] _endpoints;
    private int _count;
    public init() {
        _endpoints = new RouteEndpoint[4];
        _count = 0;
    }
    public void Add(string method, string pattern, RequestDelegate handler) {
        EnsureCapacity(_count + 1);
        _endpoints[_count] = new RouteEndpoint(method, new RouteTemplate(pattern), handler);
        _count += 1;
    }
    public bool TryFind(HttpRequest request, out RouteEndpoint endpoint, out RouteValues values) {
        var idx = 0;
        while (idx <_count)
        {
            let current = _endpoints[idx];
            if (current.TryMatch (request, out values)) {
                endpoint = current;
                return true;
            }
            idx += 1;
        }
        endpoint = CoreIntrinsics.DefaultValue <RouteEndpoint >();
        values = new RouteValues();
        return false;
    }
    private void EnsureCapacity(int requiredCount) {
        if (requiredCount <= _endpoints.Length)
        {
            return;
        }
        var newSize = _endpoints.Length * 2;
        if (newSize <requiredCount)
        {
            newSize = requiredCount;
        }
        var next = new RouteEndpoint[newSize];
        var idx = 0;
        while (idx <_endpoints.Length)
        {
            next[idx] = _endpoints[idx];
            idx += 1;
        }
        _endpoints = next;
    }
}
