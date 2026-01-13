namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>SHA-256 implementation with incremental, span-first API.</summary>
public sealed class SHA256 : HashAlgorithm
{
    private const int BlockSize = 64;
    private const int DigestSize = 32;
    private static uint[] InitK() {
        return new uint[] {
            0x428A2F98u, 0x71374491u, 0xB5C0FBCFu, 0xE9B5DBA5u, 0x3956C25Bu, 0x59F111F1u, 0x923F82A4u, 0xAB1C5ED5u, 0xD807AA98u, 0x12835B01u, 0x243185BEu, 0x550C7DC3u, 0x72BE5D74u, 0x80DEB1FEu, 0x9BDC06A7u, 0xC19BF174u, 0xE49B69C1u, 0xEFBE4786u, 0x0FC19DC6u, 0x240CA1CCu, 0x2DE92C6Fu, 0x4A7484AAu, 0x5CB0A9DCu, 0x76F988DAu, 0x983E5152u, 0xA831C66Du, 0xB00327C8u, 0xBF597FC7u, 0xC6E00BF3u, 0xD5A79147u, 0x06CA6351u, 0x14292967u, 0x27B70A85u, 0x2E1B2138u, 0x4D2C6DFCu, 0x53380D13u, 0x650A7354u, 0x766A0ABBu, 0x81C2C92Eu, 0x92722C85u, 0xA2BFE8A1u, 0xA81A664Bu, 0xC24B8B70u, 0xC76C51A3u, 0xD192E819u, 0xD6990624u, 0xF40E3585u, 0x106AA070u, 0x19A4C116u, 0x1E376C08u, 0x2748774Cu, 0x34B0BCB5u, 0x391C0CB3u, 0x4ED8AA4Au, 0x5B9CCA4Fu, 0x682E6FF3u, 0x748F82EEu, 0x78A5636Fu, 0x84C87814u, 0x8CC70208u, 0x90BEFFFAu, 0xA4506CEBu, 0xBEF9A3F7u, 0xC67178F2u,
        }
        ;
    }
    private readonly uint[] _state;
    private readonly byte[] _buffer;
    private int _bufferLength;
    private ulong _totalBytes;
    private readonly uint[] _k;
    public init() {
        let stateLength = 8;
        _state = new uint[stateLength];
        _buffer = new byte[BlockSize];
        _bufferLength = 0;
        _totalBytes = 0ul;
        _k = InitK();
        Reset();
    }
    public override int HashSizeBits => 256;
    public override void Append(ReadOnlySpan <byte >data) {
        if (data.Length == 0usize)
        {
            return;
        }
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        var offset = 0usize;
        if (_bufferLength >0)
        {
            let available = blockSize - NumericUnchecked.ToUSize(_bufferLength);
            if (data.Length <available)
            {
                Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_bufferLength), data.Length).CopyFrom(data);
                _bufferLength += NumericUnchecked.ToInt32(data.Length);
                _totalBytes += NumericUnchecked.ToUInt64(data.Length);
                return;
            }
            Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_bufferLength), available).CopyFrom(data.Slice(0usize,
            available));
            _bufferLength = 0;
            _totalBytes += NumericUnchecked.ToUInt64(available);
            offset = available;
            ProcessBlock(ReadOnlySpan <byte >.FromArray(ref _buffer));
        }
        while (data.Length - offset >= blockSize)
        {
            let block = data.Slice(offset, blockSize);
            ProcessBlock(block);
            offset += blockSize;
            _totalBytes += NumericUnchecked.ToUInt64(blockSize);
        }
        let remaining = data.Length - offset;
        if (remaining >0usize)
        {
            Span <byte >.FromArray(ref _buffer).Slice(0usize, remaining).CopyFrom(data.Slice(offset, remaining));
            _bufferLength = NumericUnchecked.ToInt32(remaining);
            _totalBytes += NumericUnchecked.ToUInt64(remaining);
        }
    }
    public override int FinalizeHash(Span <byte >destination) {
        if (destination.Length <DigestSize)
        {
            throw new Std.ArgumentException("destination too small");
        }
        let bitLength = _totalBytes * 8ul;
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        let bufferLen = NumericUnchecked.ToUSize(_bufferLength);
        let padZeroBytes = (blockSize - ((bufferLen + 9usize) % blockSize)) % blockSize;
        let totalPad = 1usize + padZeroBytes + 8usize;
        let finalLength = bufferLen + totalPad;
        var finalArray = new byte[NumericUnchecked.ToInt32(finalLength)];
        {
            var finalBuffer = Span <byte >.FromArray(ref finalArray);
            if (bufferLen >0usize)
            {
                finalBuffer.Slice(0usize, bufferLen).CopyFrom(ReadOnlySpan <byte >.FromArray(ref _buffer).Slice(0usize, bufferLen));
            }
            let padIndex = NumericUnchecked.ToInt32(bufferLen);
            finalArray[padIndex] = 0x80u8;
            let zeroBytes = NumericUnchecked.ToInt32(padZeroBytes);
            var zeroIdx = 0;
            while (zeroIdx <zeroBytes)
            {
                finalArray[padIndex + 1 + zeroIdx] = 0u8;
                zeroIdx += 1;
            }
            let lengthPos = finalLength - 8usize;
            WriteUInt64BigEndian(bitLength, finalBuffer.Slice(lengthPos, 8usize));
        }
        var processed = 0usize;
        while (processed <finalLength)
        {
            let block = ReadOnlySpan <byte >.FromArray(ref finalArray).Slice(processed, blockSize);
            ProcessBlock(block);
            processed += blockSize;
        }
        WriteDigest(destination);
        Reset();
        return DigestSize;
    }
    public override void Reset() {
        _state[0] = 0x6A09E667u;
        _state[1] = 0xBB67AE85u;
        _state[2] = 0x3C6EF372u;
        _state[3] = 0xA54FF53Au;
        _state[4] = 0x510E527Fu;
        _state[5] = 0x9B05688Cu;
        _state[6] = 0x1F83D9ABu;
        _state[7] = 0x5BE0CD19u;
        _bufferLength = 0;
        _totalBytes = 0ul;
    }
    private void WriteDigest(Span <byte >destination) {
        var offset = 0usize;
        var idx = 0usize;
        while (idx <8usize)
        {
            WriteUInt32BigEndian(_state[idx], destination.Slice(offset, 4usize));
            offset += 4usize;
            idx += 1usize;
        }
    }
    private void ProcessBlock(ReadOnlySpan <byte >block) {
        var wArray = new uint[64];
        var i = 0;
        while (i <16)
        {
            let byteIndex = NumericUnchecked.ToUSize(i) * 4usize;
            let value = (NumericUnchecked.ToUInt32(block[byteIndex]) << 24) | (NumericUnchecked.ToUInt32(block[byteIndex + 1usize]) << 16) | (NumericUnchecked.ToUInt32(block[byteIndex + 2usize]) << 8) | NumericUnchecked.ToUInt32(block[byteIndex + 3usize]);
            wArray[i] = value;
            i += 1;
        }
        while (i <64)
        {
            wArray[i] = SmallSigma1(wArray[i - 2]) + wArray[i - 7] + SmallSigma0(wArray[i - 15]) + wArray[i - 16];
            i += 1;
        }
        var a = _state[0];
        var b = _state[1];
        var c = _state[2];
        var d = _state[3];
        var e = _state[4];
        var f = _state[5];
        var g = _state[6];
        var h = _state[7];
        i = 0;
        while (i <64)
        {
            let t1 = h + BigSigma1(e) + Ch(e, f, g) + _k[i] + wArray[i];
            let t2 = BigSigma0(a) + Maj(a, b, c);
            h = g;
            g = f;
            f = e;
            e = d + t1;
            d = c;
            c = b;
            b = a;
            a = t1 + t2;
            i += 1;
        }
        _state[0] = _state[0] + a;
        _state[1] = _state[1] + b;
        _state[2] = _state[2] + c;
        _state[3] = _state[3] + d;
        _state[4] = _state[4] + e;
        _state[5] = _state[5] + f;
        _state[6] = _state[6] + g;
        _state[7] = _state[7] + h;
    }
    private static uint RotateRight(uint value, int offset) {
        return NumericBitOperations.RotateRightUInt32(value, offset);
    }
    private static uint Ch(uint x, uint y, uint z) {
        return(x & y) ^ (~ x & z);
    }
    private static uint Maj(uint x, uint y, uint z) {
        return(x & y) ^ (x & z) ^ (y & z);
    }
    private static uint BigSigma0(uint x) {
        return RotateRight(x, 2) ^ RotateRight(x, 13) ^ RotateRight(x, 22);
    }
    private static uint BigSigma1(uint x) {
        return RotateRight(x, 6) ^ RotateRight(x, 11) ^ RotateRight(x, 25);
    }
    private static uint SmallSigma0(uint x) {
        return RotateRight(x, 7) ^ RotateRight(x, 18) ^ (x >> 3);
    }
    private static uint SmallSigma1(uint x) {
        return RotateRight(x, 17) ^ RotateRight(x, 19) ^ (x >> 10);
    }
    private static void WriteUInt32BigEndian(uint value, Span <byte >destination) {
        destination[0] = NumericUnchecked.ToByte((value >> 24) & 0xFFu);
        destination[1] = NumericUnchecked.ToByte((value >> 16) & 0xFFu);
        destination[2] = NumericUnchecked.ToByte((value >> 8) & 0xFFu);
        destination[3] = NumericUnchecked.ToByte(value & 0xFFu);
    }
    private static void WriteUInt64BigEndian(ulong value, Span <byte >destination) {
        destination[0] = NumericUnchecked.ToByte((value >> 56) & 0xFFul);
        destination[1] = NumericUnchecked.ToByte((value >> 48) & 0xFFul);
        destination[2] = NumericUnchecked.ToByte((value >> 40) & 0xFFul);
        destination[3] = NumericUnchecked.ToByte((value >> 32) & 0xFFul);
        destination[4] = NumericUnchecked.ToByte((value >> 24) & 0xFFul);
        destination[5] = NumericUnchecked.ToByte((value >> 16) & 0xFFul);
        destination[6] = NumericUnchecked.ToByte((value >> 8) & 0xFFul);
        destination[7] = NumericUnchecked.ToByte(value & 0xFFul);
    }
}
