namespace Exec;

import Std.Security.Cryptography;
import Std.Strings;
import Std.Span;

public static class Program
{
    public static int Main()
    {
        if (!Sha256Vectors())
        {
            return 1;
        }
        if (!Sha512Vectors())
        {
            return 2;
        }
        return 0;
    }

    private static bool Sha256Vectors()
    {
        var sha = HashAlgorithmFactory.CreateSha256();

        if (!Matches(sha.ComputeHash("".AsUtf8Span()), Hex.Parse("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")))
        {
            return false;
        }
        if (!Matches(sha.ComputeHash("abc".AsUtf8Span()), Hex.Parse("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")))
        {
            return false;
        }
        let msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".AsUtf8Span();
        if (!Matches(sha.ComputeHash(msg), Hex.Parse("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")))
        {
            return false;
        }

        // Incremental path
        var incremental = HashAlgorithmFactory.CreateSha256();
        incremental.Append("abc".AsUtf8Span().Slice(0usize, 1usize));
        incremental.Append("abc".AsUtf8Span().Slice(1usize, 2usize));
        var output = Span<byte>.StackAlloc(32usize);
        let written = incremental.FinalizeHash(output);
        if (written != 32)
        {
            return false;
        }
        if (!Matches(output.Slice(0usize, 32usize), Hex.Parse("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")))
        {
            return false;
        }
        return true;
    }

    private static bool Sha512Vectors()
    {
        var sha = HashAlgorithmFactory.CreateSha512();
        if (!Matches(sha.ComputeHash("abc".AsUtf8Span()), Hex.Parse("ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f")))
        {
            return false;
        }
        var emptyDigest = Span<byte>.StackAlloc(64usize);
        sha.Append("".AsUtf8Span());
        let written = sha.FinalizeHash(emptyDigest);
        if (written != 64)
        {
            return false;
        }
        if (!Matches(emptyDigest.Slice(0usize, 64usize), Hex.Parse("cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e")))
        {
            return false;
        }
        return true;
    }

    private static bool Matches(byte[] actual, byte[] expected)
    {
        return Matches(ReadOnlySpan<byte>.FromArray(ref actual), expected);
    }

    private static bool Matches(ReadOnlySpan<byte> actual, byte[] expected)
    {
        let span = ReadOnlySpan<byte>.FromArray(ref expected);
        if (actual.Length != span.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx < actual.Length)
        {
            if (actual[idx] != span[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
}
