namespace Exec;

import Std.Security.Cryptography;
import Std.Span;

public static class RngTest
{
    public static int Main()
    {
        var bytes = RandomNumberGenerator.GetBytes(32);
        if (IsAllZero(bytes))
        {
            return 1;
        }
        var second = RandomNumberGenerator.GetBytes(32);
        if (Matches(bytes, second))
        {
            return 2;
        }

        var buffer = new byte[16];
        RandomNumberGenerator.Fill(Span<byte>.FromArray(ref buffer));
        if (IsAllZero(buffer))
        {
            return 3;
        }
        return 0;
    }

    private static bool IsAllZero(byte[] data)
    {
        var idx = 0usize;
        while (idx < data.Length)
        {
            if (data[idx] != 0u8)
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }

    private static bool Matches(byte[] left, byte[] right)
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
