namespace Std.Security.Cryptography;
import Std.Span;
import Std.Runtime.Collections;
import Std.Numeric;
/// <summary>Cryptographically secure random number generator backed by the host OS.</summary>
public sealed class RandomNumberGenerator
{
    private static class RuntimeExports
    {
        @extern("C") public static extern bool chic_rt_random_fill(* mut @expose_address byte buffer, usize length);
    }
    public static void Fill(Span <byte >data) {
        if (data.Length == 0usize)
        {
            return;
        }
        let raw = data.Raw;
        let ok = RuntimeExports.chic_rt_random_fill(raw.Data.Pointer, data.Length);
        if (!ok)
        {
            throw new Std.InvalidOperationException("cryptographic RNG unavailable");
        }
    }
    public static byte[] GetBytes(int count) {
        if (count <0)
        {
            throw new Std.ArgumentOutOfRangeException("count");
        }
        if (count == 0)
        {
            let empty = 0;
            return new byte[empty];
        }
        var buffer = new byte[count];
        let span = Span <byte >.FromArray(ref buffer);
        Fill(span);
        return buffer;
    }
}
