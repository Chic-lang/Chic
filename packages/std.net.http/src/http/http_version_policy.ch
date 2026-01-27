namespace Std.Net.Http;
/// <summary>
/// Controls how the HTTP client negotiates protocol versions.
/// </summary>
public enum HttpVersionPolicy
{
    RequestVersionOrLower = 0, RequestVersionOrHigher = 1, RequestVersionExact = 2,
}
