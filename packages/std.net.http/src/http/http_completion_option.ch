namespace Std.Net.Http;
/// <summary>Controls when an HTTP operation completes.</summary>
public enum HttpCompletionOption
{
    /// <summary>Complete after reading the full response content.</summary>
    ResponseContentRead = 0,
    /// <summary>Complete as soon as the response headers are available.</summary>
    ResponseHeadersRead = 1,
}
