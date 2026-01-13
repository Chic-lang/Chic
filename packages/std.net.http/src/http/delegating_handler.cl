namespace Std.Net.Http;
import Std.Async;
/// <summary>
/// Handler that delegates to an inner handler, enabling middleware composition.
/// </summary>
public abstract class DelegatingHandler : HttpMessageHandler
{
    public HttpMessageHandler ?InnerHandler {
        get;
        set;
    }
    public override HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        if (InnerHandler == null)
        {
            throw new Std.InvalidOperationException("DelegatingHandler requires InnerHandler");
        }
        return InnerHandler.Send(request, completionOption, cancellationToken);
    }
}
