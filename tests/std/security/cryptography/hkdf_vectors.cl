namespace Exec;

import Std.Security.Cryptography;
import Std.Strings;
import Std.Span;
import Std.Numeric;

testcase HkdfSha256Case1()
{
    let ikm = Hex.Parse("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
    let salt = Hex.Parse("000102030405060708090a0b0c");
    let info = Hex.Parse("f0f1f2f3f4f5f6f7f8f9");
    let expectedPrk = Hex.Parse("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5");
    let expectedOkm = Hex.Parse("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865");

    var prk = new byte[NumericUnchecked.ToInt32(expectedPrk.Length)];
    let prkWritten = HKDF.Extract(
        HashAlgorithmName.Sha256(),
        ReadOnlySpan<byte>.FromArray(ref ikm),
        ReadOnlySpan<byte>.FromArray(ref salt),
        Span<byte>.FromArray(ref prk)
    );
    if (prkWritten != NumericUnchecked.ToInt32(expectedPrk.Length))
    {
        return false;
    }
    if (!Matches(ReadOnlySpan<byte>.FromArray(ref prk), ReadOnlySpan<byte>.FromArray(ref expectedPrk)))
    {
        return false;
    }

    var okm = new byte[NumericUnchecked.ToInt32(expectedOkm.Length)];
    let okmWritten = HKDF.Expand(
        HashAlgorithmName.Sha256(),
        ReadOnlySpan<byte>.FromArray(ref prk).Slice(0usize, NumericUnchecked.ToUSize(prkWritten)),
        ReadOnlySpan<byte>.FromArray(ref info),
        Span<byte>.FromArray(ref okm)
    );
    if (okmWritten != NumericUnchecked.ToInt32(expectedOkm.Length))
    {
        return false;
    }
    return Matches(ReadOnlySpan<byte>.FromArray(ref okm), ReadOnlySpan<byte>.FromArray(ref expectedOkm));
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
