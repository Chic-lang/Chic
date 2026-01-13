namespace Std
{
    import Std.Strings;
    import Std.Span;
    /// <summary>Minimal IDN helper stub that preserves input while UTF-8 parsing stabilises.</summary>
    internal static class UriIdn
    {
        internal static bool TryGetAsciiHost(ReadOnlySpan <byte >hostSpan, out string asciiHost, out string error) {
            error = Std.Runtime.StringRuntime.Create();
            asciiHost = Utf8String.FromSpan(hostSpan);
            return true;
        }
        internal static bool TryGetUnicodeHost(string asciiHost, out string unicodeHost, out string error) {
            error = Std.Runtime.StringRuntime.Create();
            unicodeHost = asciiHost;
            return true;
        }
    }
}
