namespace Chic.Web;
import Std.Async;
/// <summary>Routes the request to the matching endpoint or returns 404.</summary>
public sealed class RoutingMiddleware
{
    private RouteTable _routes;
    public init(RouteTable routes) {
        _routes = routes;
    }
    public Task Invoke(HttpContext context) {
        if (_routes.TryFind (context.Request, out var endpoint, out var values)) {
            context.Request.SetRouteValues(values);
            return endpoint.Handler.Invoke(context);
        }
        context.Response.StatusCode = 404;
        context.Response.Headers.Set("Content-Type", "text/plain");
        return context.Response.WriteStringAsync("Not Found");
    }
}
