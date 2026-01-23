namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>HMAC with SHA-512.</summary>
public sealed class HmacSha512 : Hmac
{
    private const int BlockSize = 128;
    private const int DigestSize = 64;
    private readonly SHA512 _hash;
    private byte[] _key;
    private byte[] _innerPad;
    private byte[] _outerPad;
    private bool _initialised;
    public init() {
        _hash = new SHA512();
        let empty = 0;
        _key = new byte[empty];
        _innerPad = new byte[BlockSize];
        _outerPad = new byte[BlockSize];
        _initialised = false;
    }
    public override void SetKey(ReadOnlySpan <byte >key) {
        var effectiveKey = key;
        if (key.Length >BlockSize)
        {
            var hashed = _hash.ComputeHash(key);
            effectiveKey = ReadOnlySpan <byte >.FromArray(ref hashed);
        }
        _key = new byte[BlockSize];
        var keySpan = Span <byte >.FromArray(ref _key);
        if (effectiveKey.Length >0usize)
        {
            keySpan.Slice(0usize, effectiveKey.Length).CopyFrom(effectiveKey);
        }
        _innerPad = new byte[BlockSize];
        _outerPad = new byte[BlockSize];
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            let b = idx <effectiveKey.Length ?effectiveKey[idx] : 0u8;
            _innerPad[idx] = (byte)(b ^ 0x36u8);
            _outerPad[idx] = (byte)(b ^ 0x5cu8);
            idx += 1usize;
        }
        _initialised = true;
        Reset();
    }
    public override void Append(ReadOnlySpan <byte >data) {
        EnsureKey();
        if (data.Length == 0usize)
        {
            return;
        }
        _hash.Append(data);
    }
    public override int FinalizeHash(Span <byte >destination) {
        EnsureKey();
        var innerDigest = new byte[DigestSize];
        let innerWritten = 0;
        {
            let innerSpan = Span <byte >.FromArray(ref innerDigest);
            innerWritten = _hash.FinalizeHash(innerSpan);
        }
        _hash.Reset();
        _hash.Append(ReadOnlySpan <byte >.FromArray(ref _outerPad));
        _hash.Append(ReadOnlySpan <byte >.FromArray(ref innerDigest).Slice(0usize, NumericUnchecked.ToUSize(innerWritten)));
        let written = _hash.FinalizeHash(destination);
        Reset();
        return written;
    }
    public override void Reset() {
        EnsureKey();
        _hash.Reset();
        _hash.Append(ReadOnlySpan <byte >.FromArray(ref _innerPad));
    }
    private void EnsureKey() {
        if (!_initialised)
        {
            throw new Std.InvalidOperationException("HMAC key must be set before use");
        }
    }
}
