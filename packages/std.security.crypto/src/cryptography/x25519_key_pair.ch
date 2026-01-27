namespace Std.Security.Cryptography;
/// <summary>Represents an X25519 key pair.</summary>
public struct X25519KeyPair
{
    public byte[] PublicKey;
    public byte[] PrivateKey;
    public init(byte[] publicKey, byte[] privateKey) {
        PublicKey = publicKey;
        PrivateKey = privateKey;
    }
}
