namespace Std.Net.Http;
import Std.Async;
/// <summary>
/// Base invoker that routes requests through a configured message handler.
/// </summary>
public abstract class HttpMessageInvoker
{
    private readonly HttpMessageHandler _handler;
    private readonly bool _disposeHandler;
    private bool _disposed;
    protected init(HttpMessageHandler handler, bool disposeHandler) {
        _handler = handler;
        _disposeHandler = disposeHandler;
        _disposed = false;
    }
    protected HttpMessageHandler Handler => _handler;
    public HttpResponseMessage Send(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        EnsureNotDisposed();
        return _handler.Send(request, completionOption, cancellationToken);
    }
    public Task <HttpResponseMessage >SendAsync(HttpRequestMessage request, HttpCompletionOption completionOption, CancellationToken cancellationToken) {
        EnsureNotDisposed();
        return _handler.SendAsync(request, completionOption, cancellationToken);
    }
    private void EnsureNotDisposed() {
        if (_disposed)
        {
            throw new Std.InvalidOperationException(Std.Runtime.StringRuntime.FromStr("HttpMessageInvoker has been disposed"));
        }
    }
    public virtual void Dispose() {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        if (_disposeHandler)
        {
            _handler.Dispose();
        }
    }
}
