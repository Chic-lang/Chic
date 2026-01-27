namespace Std.Security.Cryptography;
import Std.Span;
/// <summary>AES symmetric algorithm supporting CBC mode.</summary>
public sealed class AesAlgorithm : SymmetricAlgorithm
{
    public override ICryptoTransform CreateEncryptor() {
        EnsureMode();
        let key = KeyMaterial();
        let iv = IvMaterial();
        EnsureKeyAndIv(key, iv);
        return new AesCbcCryptoTransform(key, iv, Padding, true);
    }
    public override ICryptoTransform CreateDecryptor() {
        EnsureMode();
        let key = KeyMaterial();
        let iv = IvMaterial();
        EnsureKeyAndIv(key, iv);
        return new AesCbcCryptoTransform(key, iv, Padding, false);
    }
    protected override void ValidateKey(ReadOnlySpan <byte >key) {
        let length = key.Length;
        if (length != 16usize && length != 24usize && length != 32usize)
        {
            throw new Std.ArgumentException("AES key must be 16, 24, or 32 bytes");
        }
    }
    protected override void ValidateIV(ReadOnlySpan <byte >iv) {
        if (iv.Length != 16usize)
        {
            throw new Std.ArgumentException("AES IV must be 16 bytes");
        }
    }
    private void EnsureMode() {
        if (Mode != CipherMode.CBC)
        {
            throw new Std.NotSupportedException("Only CBC mode is supported");
        }
        if (Padding != PaddingMode.PKCS7 && Padding != PaddingMode.None)
        {
            throw new Std.NotSupportedException("Unsupported padding mode");
        }
    }
    private static void EnsureKeyAndIv(ReadOnlySpan <byte >key, ReadOnlySpan <byte >iv) {
        if (key.Length == 0usize)
        {
            throw new Std.InvalidOperationException("Key must be set before creating a transform");
        }
        if (iv.Length == 0usize)
        {
            throw new Std.InvalidOperationException("IV must be set before creating a transform");
        }
    }
}
