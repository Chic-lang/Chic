namespace Std.Net.Http;
/// <summary>
/// HTTP status codes supported by the bootstrap client.
/// </summary>
public enum HttpStatusCode
{
    Continue = 100, SwitchingProtocols = 101, OK = 200, Created = 201, Accepted = 202, NoContent = 204, PartialContent = 206, MultipleChoices = 300, MovedPermanently = 301, Found = 302, NotModified = 304, TemporaryRedirect = 307, PermanentRedirect = 308, BadRequest = 400, Unauthorized = 401, Forbidden = 403, NotFound = 404, MethodNotAllowed = 405, RequestTimeout = 408, Conflict = 409, Gone = 410, PayloadTooLarge = 413, UnsupportedMediaType = 415, TooManyRequests = 429, InternalServerError = 500, NotImplemented = 501, BadGateway = 502, ServiceUnavailable = 503, GatewayTimeout = 504,
}
