namespace Std.Net.Http;
/// <summary>
/// Raised when an HTTP request fails due to transport or protocol errors.
/// </summary>
public class HttpRequestException : Std.Exception
{
    public HttpStatusCode ?StatusCode;
    public init() : super() {
    }
    public init(string message) : super(message) {
    }
    public init(string message, HttpStatusCode statusCode) : super(message) {
        StatusCode = statusCode;
    }
}
