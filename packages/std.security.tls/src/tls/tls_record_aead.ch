namespace Std.Security.Tls;
import Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>AES-GCM record protection used by modern TLS cipher suites.</summary>
public sealed class TlsRecordAead
{
    private const int NonceSize = 12;
    private const int TagSize = 16;
    private const int HeaderSize = 5;
    private const ushort LegacyVersion = 0x0303;
    private readonly AesGcm _aead;
    private readonly byte[] _iv;
    private readonly TlsCipherSuite _suite;
    public init(TlsCipherSuite suite, ReadOnlySpan <byte >key, ReadOnlySpan <byte >iv) {
        if (iv.Length != NonceSize)
        {
            throw new Std.ArgumentException("IV must be 12 bytes");
        }
        _suite = suite;
        _aead = new AesGcm(key);
        _iv = new byte[NonceSize];
        Span <byte >.FromArray(ref _iv).CopyFrom(iv);
    }
    public int EncryptRecord(ulong sequenceNumber, TlsContentType contentType, ReadOnlySpan <byte >plaintext, Span <byte >destination) {
        let requiredLength = HeaderSize + plaintext.Length + TagSize;
        if (destination.Length <requiredLength)
        {
            throw new Std.ArgumentException("destination too small for record");
        }
        var header = destination.Slice(0usize, HeaderSize);
        WriteHeader(contentType, NumericUnchecked.ToUInt16(plaintext.Length + TagSize), header);
        var nonce = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(NonceSize));
        BuildNonce(sequenceNumber, nonce);
        var cipherSpan = destination.Slice(NumericUnchecked.ToUSize(HeaderSize), plaintext.Length);
        var tagSpan = destination.Slice(NumericUnchecked.ToUSize(HeaderSize) + plaintext.Length, NumericUnchecked.ToUSize(TagSize));
        _aead.Encrypt(nonce.AsReadOnly(), plaintext, cipherSpan, tagSpan, header.AsReadOnly());
        return requiredLength;
    }
    public int DecryptRecord(ulong sequenceNumber, ReadOnlySpan <byte >record, Span <byte >plaintext, out TlsContentType contentType) {
        if (record.Length <HeaderSize + TagSize)
        {
            throw new Std.ArgumentException("record too small");
        }
        let length = NumericUnchecked.ToUInt16((NumericUnchecked.ToUInt32(record[3]) << 8) | NumericUnchecked.ToUInt32(record[4]));
        if (length + HeaderSize >record.Length)
        {
            throw new Std.ArgumentException("record length mismatch");
        }
        contentType = (TlsContentType) record[0];
        let cipherLength = NumericUnchecked.ToInt32(length - TagSize);
        if (plaintext.Length <cipherLength)
        {
            throw new Std.ArgumentException("plaintext buffer too small");
        }
        var header = record.Slice(0usize, HeaderSize);
        var cipherSpan = record.Slice(NumericUnchecked.ToUSize(HeaderSize), NumericUnchecked.ToUSize(cipherLength));
        var tagSpan = record.Slice(NumericUnchecked.ToUSize(HeaderSize + cipherLength), NumericUnchecked.ToUSize(TagSize));
        var nonce = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(NonceSize));
        BuildNonce(sequenceNumber, nonce);
        try {
            _aead.Decrypt(nonce.AsReadOnly(), cipherSpan, tagSpan, plaintext, header);
        }
        catch(Std.InvalidOperationException ex) {
            throw new TlsAlertException("record authentication failed: " + ex.Message);
        }
        return cipherLength;
    }
    private void BuildNonce(ulong sequenceNumber, Span <byte >nonce) {
        nonce.CopyFrom(ReadOnlySpan <byte >.FromArray(ref _iv));
        var seq = Span <byte >.StackAlloc(8usize);
        WriteUInt64BigEndian(sequenceNumber, seq);
        var idx = 0usize;
        while (idx <8usize)
        {
            let nIdx = NonceSize - 8usize + idx;
            nonce[nIdx] = (byte)(nonce[nIdx] ^ seq[idx]);
            idx += 1usize;
        }
    }
    private static void WriteHeader(TlsContentType contentType, ushort length, Span <byte >destination) {
        destination[0] = (byte) contentType;
        destination[1] = NumericUnchecked.ToByte((LegacyVersion >> 8) & 0xFF);
        destination[2] = NumericUnchecked.ToByte(LegacyVersion & 0xFF);
        destination[3] = NumericUnchecked.ToByte((length >> 8) & 0xFF);
        destination[4] = NumericUnchecked.ToByte(length & 0xFF);
    }
    private static void WriteUInt64BigEndian(ulong value, Span <byte >destination) {
        destination[0] = NumericUnchecked.ToByte((value >> 56) & 0xFFul);
        destination[1usize] = NumericUnchecked.ToByte((value >> 48) & 0xFFul);
        destination[2usize] = NumericUnchecked.ToByte((value >> 40) & 0xFFul);
        destination[3usize] = NumericUnchecked.ToByte((value >> 32) & 0xFFul);
        destination[4usize] = NumericUnchecked.ToByte((value >> 24) & 0xFFul);
        destination[5usize] = NumericUnchecked.ToByte((value >> 16) & 0xFFul);
        destination[6usize] = NumericUnchecked.ToByte((value >> 8) & 0xFFul);
        destination[7usize] = NumericUnchecked.ToByte(value & 0xFFul);
    }
}
