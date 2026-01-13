namespace Std.Security.Cryptography;
import Std.Numeric;
import Std.Span;
/// <summary>Factory helpers for AES symmetric encryption.</summary>
public static class Aes
{
    public static SymmetricAlgorithm Create() {
        return new AesAlgorithm();
    }
    public static byte[] GenerateKey(int sizeBytes = 32) {
        if (sizeBytes != 16 && sizeBytes != 24 && sizeBytes != 32)
        {
            throw new Std.ArgumentException("AES key size must be 16, 24, or 32 bytes");
        }
        var key = new byte[sizeBytes];
        Std.Security.Cryptography.RandomNumberGenerator.Fill(Span <byte >.FromArray(ref key));
        return key;
    }
    public static byte[] GenerateIV() {
        let blockSize = 16;
        var iv = new byte[blockSize];
        Std.Security.Cryptography.RandomNumberGenerator.Fill(Span <byte >.FromArray(ref iv));
        return iv;
    }
}
