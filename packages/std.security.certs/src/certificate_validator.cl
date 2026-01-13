namespace Std.Security.Certs;
import Std.IO;
import Std.Span;
import Std.Numeric;
/// <summary>Lightweight certificate helpers for native TLS.</summary>
public static class CertificateValidator
{
    public static bool IsTrusted(ReadOnlySpan <byte >certificate, string[] trustedRoots) {
        if (trustedRoots == null || trustedRoots.Length == 0usize)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <trustedRoots.Length)
        {
            let path = trustedRoots[idx];
            if (path != null && path.Length >0)
            {
                if (TryLoadFile (path, out var data)) {
                    let span = ReadOnlySpan <byte >.FromArray(ref data);
                    if (Matches (certificate, span))
                    {
                        return true;
                    }
                }
            }
            idx += 1usize;
        }
        return false;
    }
    public static bool MatchesHost(string expectedHost, string presentedHost) {
        if (expectedHost == null || expectedHost.Length == 0)
        {
            return true;
        }
        if (presentedHost == null || presentedHost.Length == 0)
        {
            return false;
        }
        let lhs = NormalizeHost(expectedHost);
        let rhs = NormalizeHost(presentedHost);
        return lhs == rhs;
    }
    private static string NormalizeHost(string value) {
        var span = value.AsUtf8Span();
        var buffer = new byte[span.Length];
        var idx = 0usize;
        while (idx <span.Length)
        {
            var b = span[idx];
            if (b >= NumericUnchecked.ToByte ('A') && b <= NumericUnchecked.ToByte ('Z'))
            {
                b = NumericUnchecked.ToByte(b + NumericUnchecked.ToByte(32));
            }
            buffer[idx] = b;
            idx += 1usize;
        }
        return Utf8String.FromSpan(ReadOnlySpan <byte >.FromArray(ref buffer));
    }
    private static bool TryLoadFile(string path, out byte[] data) {
        try {
            var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
            let length = NumericUnchecked.ToInt32(stream.Length);
            data = new byte[length];
            var span = Span <byte >.FromArray(ref data);
            var read = stream.Read(span);
            stream.Dispose();
            return read == length;
        }
        catch(Std.Exception) {
            data = new byte[0];
            return false;
        }
    }
    private static bool Matches(ReadOnlySpan <byte >left, ReadOnlySpan <byte >right) {
        if (left.Length != right.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <left.Length)
        {
            if (left[idx] != right[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
}
