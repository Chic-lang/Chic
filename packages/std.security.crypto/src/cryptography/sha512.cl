namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>SHA-512 implementation with span-first incremental API.</summary>
public sealed class SHA512 : HashAlgorithm
{
    private const int BlockSize = 128;
    private const int DigestSize = 64;
    private static ulong[] InitK() {
        return new ulong[] {
            0x428A2F98D728AE22ul, 0x7137449123EF65CDul, 0xB5C0FBCFEC4D3B2Ful, 0xE9B5DBA58189DBBCul, 0x3956C25BF348B538ul, 0x59F111F1B605D019ul, 0x923F82A4AF194F9Bul, 0xAB1C5ED5DA6D8118ul, 0xD807AA98A3030242ul, 0x12835B0145706FBEul, 0x243185BE4EE4B28Cul, 0x550C7DC3D5FFB4E2ul, 0x72BE5D74F27B896Ful, 0x80DEB1FE3B1696B1ul, 0x9BDC06A725C71235ul, 0xC19BF174CF692694ul, 0xE49B69C19EF14AD2ul, 0xEFBE4786384F25E3ul, 0x0FC19DC68B8CD5B5ul, 0x240CA1CC77AC9C65ul, 0x2DE92C6F592B0275ul, 0x4A7484AA6EA6E483ul, 0x5CB0A9DCBD41FBD4ul, 0x76F988DA831153B5ul, 0x983E5152EE66DFABul, 0xA831C66D2DB43210ul, 0xB00327C898FB213Ful, 0xBF597FC7BEEF0EE4ul, 0xC6E00BF33DA88FC2ul, 0xD5A79147930AA725ul, 0x06CA6351E003826Ful, 0x142929670A0E6E70ul, 0x27B70A8546D22FFcul, 0x2E1B21385C26C926ul, 0x4D2C6DFC5AC42AEDul, 0x53380D139D95B3DFul, 0x650A73548BAF63DEul, 0x766A0ABB3C77B2A8ul, 0x81C2C92E47EDAEE6ul, 0x92722C851482353Bul, 0xA2BFE8A14CF10364ul, 0xA81A664BBC423001ul, 0xC24B8B70D0F89791ul, 0xC76C51A30654BE30ul, 0xD192E819D6EF5218ul, 0xD69906245565A910ul, 0xF40E35855771202Aul, 0x106AA07032BBD1B8ul, 0x19A4C116B8D2D0C8ul, 0x1E376C085141AB53ul, 0x2748774CDF8EEB99ul, 0x34B0BCB5E19B48A8ul, 0x391C0CB3C5C95A63ul, 0x4ED8AA4AE3418ACBul, 0x5B9CCA4F7763E373ul, 0x682E6FF3D6B2B8A3ul, 0x748F82EE5DEFB2FCul, 0x78A5636F43172F60ul, 0x84C87814A1F0AB72ul, 0x8CC702081A6439ECul, 0x90BEFFFA23631E28ul, 0xA4506CEBDE82BDE9ul, 0xBEF9A3F7B2C67915ul, 0xC67178F2E372532Bul, 0xCA273ECEEA26619Cul, 0xD186B8C721C0C207ul, 0xEADA7DD6CDE0EB1Eul, 0xF57D4F7FEE6ED178ul, 0x06F067AA72176FBAl, 0x0A637DC5A2C898A6ul, 0x113F9804BEF90DAEul, 0x1B710B35131C471Bul, 0x28DB77F523047D84ul, 0x32CAAB7B40C72493ul, 0x3C9EBE0A15C9BEBCul, 0x431D67C49C100D4Cul, 0x4CC5D4BECB3E42B6ul, 0x597F299CFC657E2Aul, 0x5FCB6FAB3AD6FAECul, 0x6C44198C4A475817ul,
        }
        ;
    }
    private readonly ulong[] _state;
    private readonly byte[] _buffer;
    private int _bufferLength;
    private ulong _totalBytes;
    private readonly ulong[] _k;
    public init() {
        let stateLength = 8;
        _state = new ulong[stateLength];
        _buffer = new byte[BlockSize];
        _bufferLength = 0;
        _totalBytes = 0ul;
        _k = InitK();
        Reset();
    }
    public override int HashSizeBits => 512;
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
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        let bufferLen = NumericUnchecked.ToUSize(_bufferLength);
        let padZeroBytes = (blockSize - ((bufferLen + 17usize) % blockSize)) % blockSize;
        let totalPad = 1usize + padZeroBytes + 16usize;
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
            let lengthPos = finalLength - 16usize;
            WriteUInt128BigEndian(_totalBytes * 8ul, finalBuffer.Slice(lengthPos, 16usize));
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
        _state[0] = 0x6A09E667F3BCC908ul;
        _state[1] = 0xBB67AE8584CAA73Bul;
        _state[2] = 0x3C6EF372FE94F82Bul;
        _state[3] = 0xA54FF53A5F1D36F1ul;
        _state[4] = 0x510E527FADE682D1ul;
        _state[5] = 0x9B05688C2B3E6C1Ful;
        _state[6] = 0x1F83D9ABFB41BD6Bul;
        _state[7] = 0x5BE0CD19137E2179ul;
        _bufferLength = 0;
        _totalBytes = 0ul;
    }
    private void WriteDigest(Span <byte >destination) {
        var offset = 0usize;
        var idx = 0usize;
        while (idx <8usize)
        {
            WriteUInt64BigEndian(_state[idx], destination.Slice(offset, 8usize));
            offset += 8usize;
            idx += 1usize;
        }
    }
    private void ProcessBlock(ReadOnlySpan <byte >block) {
        var wArray = new ulong[80];
        var i = 0;
        while (i <16)
        {
            let byteIndex = NumericUnchecked.ToUSize(i) * 8usize;
            let value = (NumericUnchecked.ToUInt64(block[byteIndex]) << 56) | (NumericUnchecked.ToUInt64(block[byteIndex + 1usize]) << 48) | (NumericUnchecked.ToUInt64(block[byteIndex + 2usize]) << 40) | (NumericUnchecked.ToUInt64(block[byteIndex + 3usize]) << 32) | (NumericUnchecked.ToUInt64(block[byteIndex + 4usize]) << 24) | (NumericUnchecked.ToUInt64(block[byteIndex + 5usize]) << 16) | (NumericUnchecked.ToUInt64(block[byteIndex + 6usize]) << 8) | NumericUnchecked.ToUInt64(block[byteIndex + 7usize]);
            wArray[i] = value;
            i += 1;
        }
        while (i <80)
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
        while (i <80)
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
    private static ulong RotateRight(ulong value, int offset) {
        return NumericBitOperations.RotateRightUInt64(value, offset);
    }
    private static ulong Ch(ulong x, ulong y, ulong z) {
        return(x & y) ^ (~ x & z);
    }
    private static ulong Maj(ulong x, ulong y, ulong z) {
        return(x & y) ^ (x & z) ^ (y & z);
    }
    private static ulong BigSigma0(ulong x) {
        return RotateRight(x, 28) ^ RotateRight(x, 34) ^ RotateRight(x, 39);
    }
    private static ulong BigSigma1(ulong x) {
        return RotateRight(x, 14) ^ RotateRight(x, 18) ^ RotateRight(x, 41);
    }
    private static ulong SmallSigma0(ulong x) {
        return RotateRight(x, 1) ^ RotateRight(x, 8) ^ (x >> 7);
    }
    private static ulong SmallSigma1(ulong x) {
        return RotateRight(x, 19) ^ RotateRight(x, 61) ^ (x >> 6);
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
    private static void WriteUInt128BigEndian(ulong lowBits, Span <byte >destination) {
        // High 64 bits are zero for supported message sizes.
        WriteUInt64BigEndian(0ul, destination.Slice(0usize, 8usize));
        WriteUInt64BigEndian(lowBits, destination.Slice(8usize, 8usize));
    }
}
