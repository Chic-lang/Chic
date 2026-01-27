namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>Base class for symmetric block algorithms.</summary>
public abstract class SymmetricAlgorithm
{
    private byte[] _key;
    private byte[] _iv;
    private CipherMode _mode;
    private PaddingMode _padding;
    protected init() {
        let empty = 0;
        _key = new byte[empty];
        _iv = new byte[empty];
        _mode = CipherMode.CBC;
        _padding = PaddingMode.PKCS7;
    }
    public virtual ReadOnlySpan <byte >Key {
        get {
            if (_key == null || _key.Length == 0)
            {
                return ReadOnlySpan <byte >.Empty;
            }
            return ReadOnlySpan <byte >.FromArray(ref _key).Slice(0usize, NumericUnchecked.ToUSize(_key.Length));
        }
        set {
            ValidateKey(value);
            _key = CopyToArray(value);
        }
    }
    public virtual ReadOnlySpan <byte >IV {
        get {
            if (_iv == null || _iv.Length == 0)
            {
                return ReadOnlySpan <byte >.Empty;
            }
            return ReadOnlySpan <byte >.FromArray(ref _iv).Slice(0usize, NumericUnchecked.ToUSize(_iv.Length));
        }
        set {
            ValidateIV(value);
            _iv = CopyToArray(value);
        }
    }
    public CipherMode Mode {
        get {
            return _mode;
        }
        set {
            _mode = value;
        }
    }
    public PaddingMode Padding {
        get {
            return _padding;
        }
        set {
            _padding = value;
        }
    }
    protected ReadOnlySpan <byte >KeyMaterial() {
        if (_key == null)
        {
            return ReadOnlySpan <byte >.Empty;
        }
        return ReadOnlySpan <byte >.FromArray(ref _key).Slice(0usize, NumericUnchecked.ToUSize(_key.Length));
    }
    protected ReadOnlySpan <byte >IvMaterial() {
        if (_iv == null)
        {
            return ReadOnlySpan <byte >.Empty;
        }
        return ReadOnlySpan <byte >.FromArray(ref _iv).Slice(0usize, NumericUnchecked.ToUSize(_iv.Length));
    }
    protected virtual void ValidateKey(ReadOnlySpan <byte >key) {
        if (key.Length == 0usize)
        {
            throw new Std.ArgumentException("Key must not be empty");
        }
    }
    protected virtual void ValidateIV(ReadOnlySpan <byte >iv) {
        if (iv.Length == 0usize)
        {
            throw new Std.ArgumentException("IV must not be empty");
        }
    }
    public abstract ICryptoTransform CreateEncryptor();
    public abstract ICryptoTransform CreateDecryptor();
    private static byte[] CopyToArray(ReadOnlySpan <byte >source) {
        var arr = new byte[NumericUnchecked.ToInt32(source.Length)];
        if (source.Length >0usize)
        {
            Span <byte >.FromArray(ref arr).CopyFrom(source);
        }
        return arr;
    }
}
