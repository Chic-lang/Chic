namespace Exec.ConvertBase64;

import Std;
import Std.Numeric;
import Std.Span;

public static class Base64Perf
{
    public static int Main()
    {
        var data = new byte[4096];
        FillDeterministic(Span<byte>.FromArray(ref data));

        var iteration = 0;
        while (iteration < 256)
        {
            let encoded = Std.Convert.ToBase64String(ReadOnlySpan<byte>.FromArray(ref data));
            let decoded = Std.Convert.FromBase64String(encoded);
            if (!Matches(ReadOnlySpan<byte>.FromArray(ref data), ReadOnlySpan<byte>.FromArray(ref decoded)))
            {
                return 1;
            }
            iteration += 1;
        }
        return 0;
    }

    private static void FillDeterministic(Span<byte> buffer)
    {
        var state = 0x9E3779B9u;
        var idx = 0usize;
        while (idx < buffer.Length)
        {
            state = (state * 1664525u) + 1013904223u;
            buffer[idx] = NumericUnchecked.ToByte((state >> 16) & 0xFFu32);
            idx += 1usize;
        }
    }

    private static bool Matches(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right)
    {
        if (left.Length != right.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx < left.Length)
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
