namespace Exec;

import Std.Security.Cryptography;
import Std.Span;
import Std.Strings;
import Std.Numeric;

public static class AesVectors
{
    public static int Main()
    {
        if (!Vector256Cbc())
        {
            return 1;
        }
        if (!Pkcs7RoundTrip())
        {
            return 2;
        }
        return 0;
    }

    private static bool Vector256Cbc()
    {
        var key = Hex.Parse("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        var iv = Hex.Parse("000102030405060708090a0b0c0d0e0f");
        var plain = Hex.Parse(
            "6bc1bee22e409f96e93d7e117393172a" +
            "ae2d8a571e03ac9c9eb76fac45af8e51" +
            "30c81c46a35ce411e5fbc1191a0a52ef" +
            "f69f2445df4f9b17ad2b417be66c3710"
        );
        var expected = Hex.Parse(
            "f58c4c04d6e5f1ba779eabfb5f7bfbd6" +
            "9cfc4e967edb808d679f777bc6702c7d" +
            "39f23369a9d9bacfa530e26304231461" +
            "b2eb05e2c39be9fcda6c19078c6a9d1b"
        );

        var aes = new AesAlgorithm();
        aes.Key = ReadOnlySpan<byte>.FromArray(ref key);
        aes.IV = ReadOnlySpan<byte>.FromArray(ref iv);
        aes.Padding = PaddingMode.None;

        var encryptor = aes.CreateEncryptor();
        var ciphertext = new byte[expected.Length];
        let written = encryptor.TransformFinalBlock(ReadOnlySpan<byte>.FromArray(ref plain), Span<byte>.FromArray(ref ciphertext));
        if (written != expected.Length || !Matches(ciphertext, expected))
        {
            return false;
        }

        var decryptor = aes.CreateDecryptor();
        var decrypted = new byte[plain.Length];
        let decWritten = decryptor.TransformFinalBlock(ReadOnlySpan<byte>.FromArray(ref ciphertext), Span<byte>.FromArray(ref decrypted));
        if (decWritten != plain.Length || !Matches(decrypted, plain))
        {
            return false;
        }
        return true;
    }

    private static bool Pkcs7RoundTrip()
    {
        var key = Hex.Parse("000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f");
        var iv = Hex.Parse("0f0e0d0c0b0a09080706050403020100");
        var aes = new AesAlgorithm();
        aes.Key = ReadOnlySpan<byte>.FromArray(ref key);
        aes.IV = ReadOnlySpan<byte>.FromArray(ref iv);
        aes.Padding = PaddingMode.PKCS7;

        var encryptor = aes.CreateEncryptor();
        var plainText = "hello cbc pkcs7".AsUtf8Span();
        var ciphertext = new byte[64];
        let written = encryptor.TransformFinalBlock(plainText, Span<byte>.FromArray(ref ciphertext));

        var decryptor = aes.CreateDecryptor();
        var decrypted = new byte[64];
        let decWritten = decryptor.TransformFinalBlock(ReadOnlySpan<byte>.FromArray(ref ciphertext).Slice(0usize, NumericUnchecked.ToUSize(written)), Span<byte>.FromArray(ref decrypted));
        let decoded = decrypted.Slice(0usize, NumericUnchecked.ToUSize(decWritten));
        var plainArray = new byte[NumericUnchecked.ToInt32(plainText.Length)];
        Span<byte>.FromArray(ref plainArray).CopyFrom(plainText);
        return Matches(decoded, ReadOnlySpan<byte>.FromArray(ref plainArray));
    }

    private static bool Matches(byte[] left, byte[] right)
    {
        return Matches(ReadOnlySpan<byte>.FromArray(ref left), ReadOnlySpan<byte>.FromArray(ref right));
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
