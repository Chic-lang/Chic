namespace Exec;

import Std.Security.Cryptography;
import Std.Strings;
import Std.Span;
import Std.Numeric;

testcase Sha384HashesMatchVectors()
{
    if (!VerifyHash("abc", "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"))
    {
        return false;
    }

    let longMessage =
        "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn" +
        "hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
    if (!VerifyHash(longMessage, "09330c33f71147e83d192fc782cd1b4753111b173b3b05d22fa08086e3b0f712fcc7c71a557e2db966c3e9fa91746039"))
    {
        return false;
    }

    return true;
}

private static bool VerifyHash(string text, string expectedHex)
{
    var sha = HashAlgorithmFactory.CreateSha384();
    var digest = sha.ComputeHash(text.AsUtf8Span());
    let expected = Hex.Parse(expectedHex);
    return Matches(ReadOnlySpan<byte>.FromArray(ref digest), ReadOnlySpan<byte>.FromArray(ref expected));
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
