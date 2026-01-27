namespace Std.Compiler.Lsp.Server;
public struct NotificationRouter
{
    private NotificationRoute[] _routes;
    private int _count;
    public init() {
        _routes = new NotificationRoute[8];
        _count = 0;
    }
    public void Register(ref this, string method, NotificationHandler handler) {
        if (_count >= _routes.Length)
        {
            let resized = new NotificationRoute[_routes.Length * 2];
            var idx = 0;
            while (idx <_routes.Length)
            {
                resized[idx] = _routes[idx];
                idx += 1;
            }
            _routes = resized;
        }
        _routes[_count] = new NotificationRoute(method, handler);
        _count += 1;
    }
    public bool Dispatch(ref this, string method, string payload) {
        var idx = 0;
        while (idx <_count)
        {
            let route = _routes[idx];
            if (route.Method == method)
            {
                let handler = route.Handler;
                handler(method, payload);
                return true;
            }
            idx += 1;
        }
        return false;
    }
}
