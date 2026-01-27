namespace Exec;

import Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;

testcase AesGcmVectorWithoutAad()
{
    let key = Hex.Parse("00000000000000000000000000000000");
    let iv = Hex.Parse("000000000000000000000000");
    let plaintext = Hex.Parse("00000000000000000000000000000000");
    let expectedCipher = Hex.Parse("0388dace60b6a392f328c2b971b2fe78");
    let expectedTag = Hex.Parse("ab6e47d42cec13bdf53a67b21257bddf");

    var aes = new AesGcm(ReadOnlySpan<byte>.FromArray(ref key));

    var ciphertext = new byte[expectedCipher.Length];
    var tag = new byte[expectedTag.Length];
    aes.Encrypt(
        ReadOnlySpan<byte>.FromArray(ref iv),
        ReadOnlySpan<byte>.FromArray(ref plaintext),
        Span<byte>.FromArray(ref ciphertext),
        Span<byte>.FromArray(ref tag)
    );

    if (!Matches(ReadOnlySpan<byte>.FromArray(ref expectedCipher), ReadOnlySpan<byte>.FromArray(ref ciphertext)))
    {
        return false;
    }
    if (!Matches(ReadOnlySpan<byte>.FromArray(ref expectedTag), ReadOnlySpan<byte>.FromArray(ref tag)))
    {
        return false;
    }

    var decrypted = new byte[plaintext.Length];
    aes.Decrypt(
        ReadOnlySpan<byte>.FromArray(ref iv),
        ReadOnlySpan<byte>.FromArray(ref ciphertext),
        ReadOnlySpan<byte>.FromArray(ref tag),
        Span<byte>.FromArray(ref decrypted)
    );
    return Matches(ReadOnlySpan<byte>.FromArray(ref plaintext), ReadOnlySpan<byte>.FromArray(ref decrypted));
}

testcase AesGcmVectorWithAad()
{
    let key = Hex.Parse("feffe9928665731c6d6a8f9467308308");
    let iv = Hex.Parse("cafebabefacedbaddecaf888");
    let plaintext = Hex.Parse(
        "d9313225f88406e5a55909c5aff5269a" +
        "86a7a9531534f7da2e4c303d8a318a72" +
        "1c3c0c95956809532fcf0e2449a6b525" +
        "b16aedf5aa0de657ba637b39"
    );
    let associated = Hex.Parse("feedfacedeadbeeffeedfacedeadbeefabaddad2");
    let expectedCipher = Hex.Parse(
        "42831ec2217774244b7221b784d0d49c" +
        "e3aa212f2c02a4e035c17e2329aca12e" +
        "21d514b25466931c7d8f6a5aac84aa05" +
        "1ba30b396a0aac973d58e091"
    );
    let expectedTag = Hex.Parse("5bc94fbc3221a5db94fae95ae7121a47");

    var aes = new AesGcm(ReadOnlySpan<byte>.FromArray(ref key));
    var ciphertext = new byte[expectedCipher.Length];
    var tag = new byte[expectedTag.Length];
    aes.Encrypt(
        ReadOnlySpan<byte>.FromArray(ref iv),
        ReadOnlySpan<byte>.FromArray(ref plaintext),
        Span<byte>.FromArray(ref ciphertext),
        Span<byte>.FromArray(ref tag),
        ReadOnlySpan<byte>.FromArray(ref associated)
    );

    if (!Matches(ReadOnlySpan<byte>.FromArray(ref expectedCipher), ReadOnlySpan<byte>.FromArray(ref ciphertext)))
    {
        return false;
    }
    if (!Matches(ReadOnlySpan<byte>.FromArray(ref expectedTag), ReadOnlySpan<byte>.FromArray(ref tag)))
    {
        return false;
    }

    var decrypted = new byte[plaintext.Length];
    aes.Decrypt(
        ReadOnlySpan<byte>.FromArray(ref iv),
        ReadOnlySpan<byte>.FromArray(ref ciphertext),
        ReadOnlySpan<byte>.FromArray(ref tag),
        Span<byte>.FromArray(ref decrypted),
        ReadOnlySpan<byte>.FromArray(ref associated)
    );
    return Matches(ReadOnlySpan<byte>.FromArray(ref plaintext), ReadOnlySpan<byte>.FromArray(ref decrypted));
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
