# chic.web overview

`chic.web` is a minimal API stack for Chic: a Kestrel-style host, middleware pipeline, and lightweight routing. The library lives outside `Std` and is 100% Chic-native.

## Quick start

```cl
namespace Demo;

import Chic.Web;
import Std.Async;

public class Program
{
    private static CancellationTokenSource _cts;

    public static int Main()
    {
        _cts = CancellationTokenSource.Create();
        var builder = WebApplication.CreateBuilder();
        builder.Urls = "http://127.0.0.1:5000";

        let app = builder.Build();
        app.MapGet("/", Hello);
        app.MapPost("/echo", Echo);

        let _ = app.RunAsync(_cts.Token());
        return 0;
    }

    private static Task<string> Hello(HttpContext ctx) =>
        TaskRuntime.FromResult("hello");

    private static Task<string> Echo(HttpContext ctx)
    {
        let data = ctx.Request.Body.ReadAllBytes();
        let text = Std.Strings.Utf8String.FromSpan(ReadOnlySpan<byte>.FromArray(ref data));
        return TaskRuntime.FromResult(text);
    }
}
```

## Pipeline and middleware

- The host builds a deterministic pipeline: exception handling → access logging → user middleware → routing.
- Add middleware with `app.Use(Middleware factory)` where the factory takes the next `RequestDelegate` and returns the wrapped delegate.
- Default middleware:
  - `ExceptionMiddleware`: catches unhandled exceptions and emits `500` with a text payload.
  - `AccessLogMiddleware`: writes `"{METHOD} {PATH} => {STATUS}"` to stdout after the request completes.

## Routing

- `MapGet/MapPost/MapPut/MapDelete/MapPatch` register literal routes and `{param}` segments (e.g., `/users/{id}`).
- Route parameters are exposed via `context.Request.RouteValues`.
- Query strings are parsed into `context.Request.Query` for simple lookup.

## Responses

- Set the status with `context.Response.StatusCode` and add headers through `context.Response.Headers`.
- Write bodies with `WriteStringAsync` or `WriteJsonAsync<T>` (JSON uses `Std.Text.Json`).
- Bodies stream through the response `Stream`; `Content-Length` is set automatically when not provided.

## Running and shutdown

- `RunAsync(token)` blocks the calling thread until the cancellation token is signaled or the listener fails.
- For graceful shutdown, keep a `CancellationTokenSource` and cancel it from an endpoint (e.g., `/shutdown`) or an external signal.
