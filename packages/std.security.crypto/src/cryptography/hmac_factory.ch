namespace Std.Security.Cryptography;
/// <summary>Factory helpers for HMAC instances keyed by hash algorithm name.</summary>
public static class HmacFactory
{
    public static Hmac Create(HashAlgorithmName hash, out int digestSize) {
        if (hash.Equals (in HashAlgorithmName.Sha256())) {
            digestSize = 32;
            return new HmacSha256();
        }
        if (hash.Equals (in HashAlgorithmName.Sha384())) {
            digestSize = 48;
            return new HmacSha384();
        }
        if (hash.Equals (in HashAlgorithmName.Sha512())) {
            digestSize = 64;
            return new HmacSha512();
        }
        throw new Std.NotSupportedException("Unsupported hash algorithm for HMAC");
    }
}
