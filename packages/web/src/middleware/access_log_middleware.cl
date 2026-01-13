namespace Chic.Web;
import Std.Async;
/// <summary>Writes a basic access log entry after handling the request.</summary>
public sealed class AccessLogMiddleware
{
    private RequestDelegate _next;
    public init(RequestDelegate next) {
        _next = next;
    }
    public Task Invoke(HttpContext context) {
        let task = _next(context);
        Std.Async.Runtime.BlockOn(task);
        let statusText = context.Response.StatusCode.ToString();
        Std.Console.Console.WriteLine(context.Request.Method + " " + context.Request.Path + " => " + statusText);
        return TaskRuntime.CompletedTask();
    }
}
