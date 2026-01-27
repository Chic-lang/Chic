namespace Std.Security.Cryptography;
/// <summary>Identifies a hash algorithm by name.</summary>
public struct HashAlgorithmName
{
    public string Name;
    public init(string name) {
        if (name == null)
        {
            Name = "";
        }
        else
        {
            Name = name;
        }
    }
    public static HashAlgorithmName Sha256() {
        return new HashAlgorithmName("SHA256");
    }
    public static HashAlgorithmName Sha384() {
        return new HashAlgorithmName("SHA384");
    }
    public static HashAlgorithmName Sha512() {
        return new HashAlgorithmName("SHA512");
    }
    public override string ToString() {
        return Name;
    }
    public bool Equals(in HashAlgorithmName other) {
        return Name == other.Name;
    }
}
