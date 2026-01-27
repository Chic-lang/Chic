namespace Std.Net.Http;
import Std.Async;
/// <summary>
/// Core abstraction for HTTP message handlers (transport or delegating).
/// </summary>
public abstract class HttpMessageHandler
{
    public virtual HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        throw new Std.InvalidOperationException("HttpMessageHandler.Send not implemented");
    }
    public virtual Task <HttpResponseMessage >SendAsync(HttpRequestMessage request, HttpCompletionOption completionOption,
    CancellationToken cancellationToken) {
        let response = Send(request, completionOption, cancellationToken);
        return TaskRuntime.FromResult(response);
    }
    public virtual void Dispose() {
    }
}
