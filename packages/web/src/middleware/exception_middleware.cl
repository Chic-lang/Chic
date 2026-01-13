namespace Chic.Web;
import Std.Async;
/// <summary>Catches unhandled exceptions and emits a 500 response.</summary>
public sealed class ExceptionMiddleware
{
    private RequestDelegate _next;
    public init(RequestDelegate next) {
        _next = next;
    }
    public Task Invoke(HttpContext context) {
        try {
            let task = _next(context);
            Std.Async.Runtime.BlockOn(task);
        }
        catch(Std.Exception ex) {
            if (context.Response.HasStarted)
            {
                return TaskRuntime.CompletedTask();
            }
            context.Response.StatusCode = 500;
            context.Response.Headers.Set("Content-Type", "text/plain");
            return context.Response.WriteStringAsync("Unhandled exception: " + ex.Message);
        }
        return TaskRuntime.CompletedTask();
    }
}
