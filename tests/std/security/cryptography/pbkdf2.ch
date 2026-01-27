namespace Exec;

import Std.Security.Cryptography;
import Std.Strings;
import Std.Span;
import Std.Numeric;

public static class Pbkdf2Vectors
{
    public static int Main()
    {
        if (!Vector(HashAlgorithmName.Sha256(), 1, "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"))
        {
            return 1;
        }
        if (!Vector(HashAlgorithmName.Sha256(), 2, "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"))
        {
            return 2;
        }
        return 0;
    }

    private static bool Vector(HashAlgorithmName hash, int iterations, string expectedHex)
    {
        let derived = Rfc2898DeriveBytes.Pbkdf2("password".AsUtf8Span(), "salt".AsUtf8Span(), iterations, 32, hash);
        let expected = Hex.Parse(expectedHex);
        return Matches(ReadOnlySpan<byte>.FromArray(ref derived), ReadOnlySpan<byte>.FromArray(ref expected));
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
