namespace Std.Security.Cryptography;
/// <summary>Factory helpers for common hash algorithms.</summary>
public static class HashAlgorithmFactory
{
    public static HashAlgorithm CreateSha256() {
        return new SHA256();
    }
    public static HashAlgorithm CreateSha384() {
        return new SHA384();
    }
    public static HashAlgorithm CreateSha512() {
        return new SHA512();
    }
}
