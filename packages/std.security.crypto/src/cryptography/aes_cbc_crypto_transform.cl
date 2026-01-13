namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>AES CBC transform with PKCS7 or unpadded operation.</summary>
public sealed class AesCbcCryptoTransform : ICryptoTransform
{
    private const int BlockSize = 16;
    private static byte[] InitSBox() {
        return new byte[] {
            0x63u8, 0x7cu8, 0x77u8, 0x7bu8, 0xf2u8, 0x6bu8, 0x6fu8, 0xc5u8, 0x30u8, 0x01u8, 0x67u8, 0x2bu8, 0xfeu8, 0xd7u8, 0xabu8, 0x76u8, 0xcau8, 0x82u8, 0xc9u8, 0x7du8, 0xfau8, 0x59u8, 0x47u8, 0xf0u8, 0xadu8, 0xd4u8, 0xa2u8, 0xafu8, 0x9cu8, 0xa4u8, 0x72u8, 0xc0u8, 0xb7u8, 0xfdu8, 0x93u8, 0x26u8, 0x36u8, 0x3fu8, 0xf7u8, 0xccu8, 0x34u8, 0xa5u8, 0xe5u8, 0xf1u8, 0x71u8, 0xd8u8, 0x31u8, 0x15u8, 0x04u8, 0xc7u8, 0x23u8, 0xc3u8, 0x18u8, 0x96u8, 0x05u8, 0x9au8, 0x07u8, 0x12u8, 0x80u8, 0xe2u8, 0xebu8, 0x27u8, 0xb2u8, 0x75u8, 0x09u8, 0x83u8, 0x2cu8, 0x1au8, 0x1bu8, 0x6eu8, 0x5au8, 0xa0u8, 0x52u8, 0x3bu8, 0xd6u8, 0xb3u8, 0x29u8, 0xe3u8, 0x2fu8, 0x84u8, 0x53u8, 0xd1u8, 0x00u8, 0xedu8, 0x20u8, 0xfcu8, 0xb1u8, 0x5bu8, 0x6au8, 0xcbu8, 0xbeu8, 0x39u8, 0x4au8, 0x4cu8, 0x58u8, 0xcfu8, 0xd0u8, 0xefu8, 0xaau8, 0xfbu8, 0x43u8, 0x4du8, 0x33u8, 0x85u8, 0x45u8, 0xf9u8, 0x02u8, 0x7fu8, 0x50u8, 0x3cu8, 0x9fu8, 0xa8u8, 0x51u8, 0xa3u8, 0x40u8, 0x8fu8, 0x92u8, 0x9du8, 0x38u8, 0xf5u8, 0xbcu8, 0xb6u8, 0xdau8, 0x21u8, 0x10u8, 0xffu8, 0xf3u8, 0xd2u8, 0xcdu8, 0x0cu8, 0x13u8, 0xecu8, 0x5fu8, 0x97u8, 0x44u8, 0x17u8, 0xc4u8, 0xa7u8, 0x7eu8, 0x3du8, 0x64u8, 0x5du8, 0x19u8, 0x73u8, 0x60u8, 0x81u8, 0x4fu8, 0xdcu8, 0x22u8, 0x2au8, 0x90u8, 0x88u8, 0x46u8, 0xeeu8, 0xb8u8, 0x14u8, 0xdeu8, 0x5eu8, 0x0bu8, 0xdbu8, 0xe0u8, 0x32u8, 0x3au8, 0x0au8, 0x49u8, 0x06u8, 0x24u8, 0x5cu8, 0xc2u8, 0xd3u8, 0xacu8, 0x62u8, 0x91u8, 0x95u8, 0xe4u8, 0x79u8, 0xe7u8, 0xc8u8, 0x37u8, 0x6du8, 0x8du8, 0xd5u8, 0x4eu8, 0xa9u8, 0x6cu8, 0x56u8, 0xf4u8, 0xeau8, 0x65u8, 0x7au8, 0xaeu8, 0x08u8, 0xbau8, 0x78u8, 0x25u8, 0x2eu8, 0x1cu8, 0xa6u8, 0xb4u8, 0xc6u8, 0xe8u8, 0xddu8, 0x74u8, 0x1fu8, 0x4bu8, 0xbdu8, 0x8bu8, 0x8au8, 0x70u8, 0x3eu8, 0xb5u8, 0x66u8, 0x48u8, 0x03u8, 0xf6u8, 0x0eu8, 0x61u8, 0x35u8, 0x57u8, 0xb9u8, 0x86u8, 0xc1u8, 0x1du8, 0x9eu8, 0xe1u8, 0xf8u8, 0x98u8, 0x11u8, 0x69u8, 0xd9u8, 0x8eu8, 0x94u8, 0x9bu8, 0x1eu8, 0x87u8, 0xe9u8, 0xceu8, 0x55u8, 0x28u8, 0xdfu8, 0x8cu8, 0xa1u8, 0x89u8, 0x0du8, 0xbfu8, 0xe6u8, 0x42u8, 0x68u8, 0x41u8, 0x99u8, 0x2du8, 0x0fu8, 0xb0u8, 0x54u8, 0xbbu8, 0x16u8,
        }
        ;
    }
    private static byte[] InitInvSBox() {
        return new byte[] {
            0x52u8, 0x09u8, 0x6au8, 0xd5u8, 0x30u8, 0x36u8, 0xa5u8, 0x38u8, 0xbfu8, 0x40u8, 0xa3u8, 0x9eu8, 0x81u8, 0xf3u8, 0xd7u8, 0xfbu8, 0x7cu8, 0xe3u8, 0x39u8, 0x82u8, 0x9bu8, 0x2fu8, 0xffu8, 0x87u8, 0x34u8, 0x8eu8, 0x43u8, 0x44u8, 0xc4u8, 0xdeu8, 0xe9u8, 0xcbu8, 0x54u8, 0x7bu8, 0x94u8, 0x32u8, 0xa6u8, 0xc2u8, 0x23u8, 0x3du8, 0xeeu8, 0x4cu8, 0x95u8, 0x0bu8, 0x42u8, 0xfau8, 0xc3u8, 0x4eu8, 0x08u8, 0x2eu8, 0xa1u8, 0x66u8, 0x28u8, 0xd9u8, 0x24u8, 0xb2u8, 0x76u8, 0x5bu8, 0xa2u8, 0x49u8, 0x6du8, 0x8bu8, 0xd1u8, 0x25u8, 0x72u8, 0xf8u8, 0xf6u8, 0x64u8, 0x86u8, 0x68u8, 0x98u8, 0x16u8, 0xd4u8, 0xa4u8, 0x5cu8, 0xccu8, 0x5du8, 0x65u8, 0xb6u8, 0x92u8, 0x6cu8, 0x70u8, 0x48u8, 0x50u8, 0xfdu8, 0xedu8, 0xb9u8, 0xdau8, 0x5eu8, 0x15u8, 0x46u8, 0x57u8, 0xa7u8, 0x8du8, 0x9du8, 0x84u8, 0x90u8, 0xd8u8, 0xabu8, 0x00u8, 0x8cu8, 0xbcu8, 0xd3u8, 0x0au8, 0xf7u8, 0xe4u8, 0x58u8, 0x05u8, 0xb8u8, 0xb3u8, 0x45u8, 0x06u8, 0xd0u8, 0x2cu8, 0x1eu8, 0x8fu8, 0xcau8, 0x3fu8, 0x0fu8, 0x02u8, 0xc1u8, 0xafu8, 0xbdu8, 0x03u8, 0x01u8, 0x13u8, 0x8au8, 0x6bu8, 0x3au8, 0x91u8, 0x11u8, 0x41u8, 0x4fu8, 0x67u8, 0xdcu8, 0xeau8, 0x97u8, 0xf2u8, 0xcfu8, 0xceu8, 0xf0u8, 0xb4u8, 0xe6u8, 0x73u8, 0x96u8, 0xacu8, 0x74u8, 0x22u8, 0xe7u8, 0xadu8, 0x35u8, 0x85u8, 0xe2u8, 0xf9u8, 0x37u8, 0xe8u8, 0x1cu8, 0x75u8, 0xdfu8, 0x6eu8, 0x47u8, 0xf1u8, 0x1au8, 0x71u8, 0x1du8, 0x29u8, 0xc5u8, 0x89u8, 0x6fu8, 0xb7u8, 0x62u8, 0x0eu8, 0xaau8, 0x18u8, 0xbeu8, 0x1bu8, 0xfcu8, 0x56u8, 0x3eu8, 0x4bu8, 0xc6u8, 0xd2u8, 0x79u8, 0x20u8, 0x9au8, 0xdcu8, 0xc0u8, 0xfeu8, 0x78u8, 0xcdu8, 0x5au8, 0xf4u8, 0x1fu8, 0xddu8, 0xa8u8, 0x33u8, 0x88u8, 0x07u8, 0xc7u8, 0x31u8, 0xb1u8, 0x12u8, 0x10u8, 0x59u8, 0x27u8, 0x80u8, 0xecu8, 0x5fu8, 0x60u8, 0x51u8, 0x7fu8, 0xa9u8, 0x19u8, 0xb5u8, 0x4au8, 0x0du8, 0x2du8, 0xe5u8, 0x7au8, 0x9fu8, 0x93u8, 0xc9u8, 0x9cu8, 0xefu8, 0xa0u8, 0xe0u8, 0x3bu8, 0x4du8, 0xaeu8, 0x2au8, 0xf5u8, 0xb0u8, 0xc8u8, 0xebu8, 0xbbu8, 0x3cu8, 0x83u8, 0x53u8, 0x99u8, 0x61u8, 0x17u8, 0x2bu8, 0x04u8, 0x7eu8, 0xbau8, 0x77u8, 0xd6u8, 0x26u8, 0xe1u8, 0x69u8, 0x14u8, 0x63u8, 0x55u8, 0x21u8, 0x0cu8, 0x7du8,
        }
        ;
    }
    private byte[] _encRoundKeys;
    private byte[] _decRoundKeys;
    private int _rounds;
    private readonly bool _encrypting;
    private readonly PaddingMode _padding;
    private readonly byte[] _iv;
    private byte[] _currentVector;
    private readonly byte[] _buffer;
    private int _bufferLength;
    private readonly byte[] _sBox;
    private readonly byte[] _invSBox;
    public init(ReadOnlySpan <byte >key, ReadOnlySpan <byte >iv, PaddingMode padding, bool encrypting) {
        _padding = padding;
        _encrypting = encrypting;
        _buffer = new byte[NumericUnchecked.ToUSize(BlockSize)];
        _bufferLength = 0;
        _iv = new byte[NumericUnchecked.ToUSize(BlockSize)];
        _currentVector = new byte[NumericUnchecked.ToUSize(BlockSize)];
        _sBox = InitSBox();
        _invSBox = InitInvSBox();
        Span <byte >.FromArray(ref _iv).CopyFrom(iv);
        Span <byte >.FromArray(ref _currentVector).CopyFrom(iv);
        let empty = 0usize;
        _encRoundKeys = new byte[empty];
        _decRoundKeys = new byte[empty];
        _rounds = 0;
        BuildKeySchedule(key);
    }
    public int InputBlockSize => BlockSize;
    public int OutputBlockSize => BlockSize;
    public bool CanTransformMultipleBlocks => true;
    public bool CanReuseTransform => true;
    public int TransformBlock(ReadOnlySpan <byte >input, Span <byte >output) {
        return _encrypting ?TransformBlockEncrypt(input, output) : TransformBlockDecrypt(input, output);
    }
    public int TransformFinalBlock(ReadOnlySpan <byte >input, Span <byte >output) {
        return _encrypting ?TransformFinalEncrypt(input, output) : TransformFinalDecrypt(input, output);
    }
    public void Reset() {
        Span <byte >.FromArray(ref _currentVector).CopyFrom(ReadOnlySpan <byte >.FromArray(ref _iv));
        _bufferLength = 0;
    }
    private int TransformBlockEncrypt(ReadOnlySpan <byte >input, Span <byte >output) {
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        let available = NumericUnchecked.ToUSize(_bufferLength) + input.Length;
        let fullBytes = (available / blockSize) * blockSize;
        if (fullBytes == 0usize)
        {
            if (input.Length >0usize)
            {
                Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_bufferLength), input.Length).CopyFrom(input);
                _bufferLength += NumericUnchecked.ToInt32(input.Length);
            }
            return 0;
        }
        if (output.Length <NumericUnchecked.ToInt32 (fullBytes))
        {
            throw new Std.ArgumentException("output too small");
        }
        var bytesWritten = 0usize;
        var inputOffset = 0usize;
        if (_bufferLength >0)
        {
            let needed = blockSize - NumericUnchecked.ToUSize(_bufferLength);
            Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_bufferLength), needed).CopyFrom(input.Slice(0usize,
            needed));
            EncryptBlockWithCbc(ReadOnlySpan <byte >.FromArray(ref _buffer), output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
            inputOffset = needed;
            _bufferLength = 0;
        }
        while (bytesWritten <fullBytes && inputOffset + blockSize <= input.Length)
        {
            let block = input.Slice(inputOffset, blockSize);
            EncryptBlockWithCbc(block, output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
            inputOffset += blockSize;
        }
        let remaining = input.Length - inputOffset;
        if (remaining >0usize)
        {
            Span <byte >.FromArray(ref _buffer).Slice(0usize, remaining).CopyFrom(input.Slice(inputOffset, remaining));
            _bufferLength = NumericUnchecked.ToInt32(remaining);
        }
        return NumericUnchecked.ToInt32(bytesWritten);
    }
    private int TransformBlockDecrypt(ReadOnlySpan <byte >input, Span <byte >output) {
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        let available = NumericUnchecked.ToUSize(_bufferLength) + input.Length;
        let fullBytes = (available / blockSize) * blockSize;
        if (fullBytes == 0usize)
        {
            if (input.Length >0usize)
            {
                Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_bufferLength), input.Length).CopyFrom(input);
                _bufferLength += NumericUnchecked.ToInt32(input.Length);
            }
            return 0;
        }
        if (output.Length <NumericUnchecked.ToInt32 (fullBytes))
        {
            throw new Std.ArgumentException("output too small");
        }
        var bytesWritten = 0usize;
        var inputOffset = 0usize;
        if (_bufferLength >0)
        {
            let needed = blockSize - NumericUnchecked.ToUSize(_bufferLength);
            Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_bufferLength), needed).CopyFrom(input.Slice(0usize,
            needed));
            DecryptBlockWithCbc(ReadOnlySpan <byte >.FromArray(ref _buffer), output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
            inputOffset = needed;
            _bufferLength = 0;
        }
        while (bytesWritten <fullBytes && inputOffset + blockSize <= input.Length)
        {
            let block = input.Slice(inputOffset, blockSize);
            DecryptBlockWithCbc(block, output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
            inputOffset += blockSize;
        }
        let remaining = input.Length - inputOffset;
        if (remaining >0usize)
        {
            Span <byte >.FromArray(ref _buffer).Slice(0usize, remaining).CopyFrom(input.Slice(inputOffset, remaining));
            _bufferLength = NumericUnchecked.ToInt32(remaining);
        }
        return NumericUnchecked.ToInt32(bytesWritten);
    }
    private int TransformFinalEncrypt(ReadOnlySpan <byte >input, Span <byte >output) {
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        let total = NumericUnchecked.ToUSize(_bufferLength) + input.Length;
        let remainder = total % blockSize;
        let paddingSize = _padding == PaddingMode.None ?0usize : blockSize - remainder;
        if (_padding == PaddingMode.None && remainder != 0usize)
        {
            throw new Std.InvalidOperationException("input length not aligned with block size");
        }
        if (_padding == PaddingMode.PKCS7 && paddingSize == 0usize)
        {
            paddingSize = blockSize;
        }
        let finalLength = total + paddingSize;
        if (output.Length <NumericUnchecked.ToInt32 (finalLength))
        {
            throw new Std.ArgumentException("output too small");
        }
        var plain = new byte[total];
        if (_bufferLength >0)
        {
            Span <byte >.FromArray(ref plain).Slice(0usize, NumericUnchecked.ToUSize(_bufferLength)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref _buffer).Slice(0usize,
            NumericUnchecked.ToUSize(_bufferLength)));
        }
        if (input.Length >0usize)
        {
            Span <byte >.FromArray(ref plain).Slice(NumericUnchecked.ToUSize(_bufferLength), input.Length).CopyFrom(input);
        }
        var bytesWritten = 0usize;
        var processed = 0usize;
        while (processed + blockSize <= total)
        {
            let block = ReadOnlySpan <byte >.FromArray(ref plain).Slice(processed, blockSize);
            EncryptBlockWithCbc(block, output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
            processed += blockSize;
        }
        if (_padding == PaddingMode.PKCS7)
        {
            var finalBlock = Span <byte >.StackAlloc(blockSize);
            let tailLength = total - processed;
            if (tailLength >0usize)
            {
                finalBlock.Slice(0usize, tailLength).CopyFrom(ReadOnlySpan <byte >.FromArray(ref plain).Slice(processed,
                tailLength));
            }
            let padValue = NumericUnchecked.ToByte(paddingSize);
            var idx = tailLength;
            while (idx <blockSize)
            {
                finalBlock[idx] = padValue;
                idx += 1usize;
            }
            EncryptBlockWithCbc(finalBlock, output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
        }
        _bufferLength = 0;
        Reset();
        return NumericUnchecked.ToInt32(bytesWritten);
    }
    private int TransformFinalDecrypt(ReadOnlySpan <byte >input, Span <byte >output) {
        let blockSize = NumericUnchecked.ToUSize(BlockSize);
        let total = NumericUnchecked.ToUSize(_bufferLength) + input.Length;
        if (total == 0usize)
        {
            return 0;
        }
        if (total % blockSize != 0usize)
        {
            throw new Std.InvalidOperationException("input length not aligned with block size");
        }
        var cipher = new byte[total];
        if (_bufferLength >0)
        {
            Span <byte >.FromArray(ref cipher).Slice(0usize, NumericUnchecked.ToUSize(_bufferLength)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref _buffer).Slice(0usize,
            NumericUnchecked.ToUSize(_bufferLength)));
        }
        if (input.Length >0usize)
        {
            Span <byte >.FromArray(ref cipher).Slice(NumericUnchecked.ToUSize(_bufferLength), input.Length).CopyFrom(input);
        }
        if (output.Length <NumericUnchecked.ToInt32 (total))
        {
            throw new Std.ArgumentException("output too small");
        }
        var bytesWritten = 0usize;
        var processed = 0usize;
        while (processed <total)
        {
            let block = ReadOnlySpan <byte >.FromArray(ref cipher).Slice(processed, blockSize);
            DecryptBlockWithCbc(block, output.Slice(bytesWritten, blockSize));
            bytesWritten += blockSize;
            processed += blockSize;
        }
        var resultLength = bytesWritten;
        if (_padding == PaddingMode.PKCS7)
        {
            if (bytesWritten == 0usize)
            {
                throw new Std.InvalidOperationException("no data to unpad");
            }
            let lastByte = output[bytesWritten - 1usize];
            let pad = NumericUnchecked.ToInt32(lastByte);
            if (pad <= 0 || pad >BlockSize)
            {
                throw new Std.InvalidOperationException("invalid padding");
            }
            var padCheck = 0usize;
            while (padCheck <NumericUnchecked.ToUSize (pad))
            {
                if (output[bytesWritten - 1usize - padCheck] != lastByte)
                {
                    throw new Std.InvalidOperationException("invalid padding");
                }
                padCheck += 1usize;
            }
            resultLength = bytesWritten - NumericUnchecked.ToUSize(pad);
        }
        _bufferLength = 0;
        Reset();
        return NumericUnchecked.ToInt32(resultLength);
    }
    private void BuildKeySchedule(ReadOnlySpan <byte >key) {
        let keyWords = NumericUnchecked.ToInt32(key.Length) / 4;
        _rounds = keyWords + 6;
        let totalWords = 4 * (_rounds + 1);
        var w = new uint[totalWords];
        var i = 0usize;
        while (i <NumericUnchecked.ToUSize (keyWords))
        {
            let idx = i * 4usize;
            let word = (NumericUnchecked.ToUInt32(key[idx]) << 24) | (NumericUnchecked.ToUInt32(key[idx + 1usize]) << 16) | (NumericUnchecked.ToUInt32(key[idx + 2usize]) << 8) | NumericUnchecked.ToUInt32(key[idx + 3usize]);
            w[i] = word;
            i += 1usize;
        }
        var rcon = 0x01u;
        while (i <NumericUnchecked.ToUSize (totalWords))
        {
            var temp = w[i - 1usize];
            if ( (i % NumericUnchecked.ToUSize (keyWords)) == 0usize)
            {
                temp = SubWord(RotWord(temp)) ^ (NumericUnchecked.ToUInt32(rcon) << 24);
                rcon = NextRcon(rcon);
            }
            else if (keyWords >6 && (i % NumericUnchecked.ToUSize (keyWords)) == 4usize)
            {
                temp = SubWord(temp);
            }
            w[i] = w[i - NumericUnchecked.ToUSize(keyWords)] ^ temp;
            i += 1usize;
        }
        let roundKeyBytes = (NumericUnchecked.ToUSize(_rounds + 1) * NumericUnchecked.ToUSize(BlockSize));
        let roundKeyLength = NumericUnchecked.ToInt32(roundKeyBytes);
        _encRoundKeys = new byte[NumericUnchecked.ToUSize(roundKeyLength)];
        i = 0usize;
        while (i <NumericUnchecked.ToUSize (totalWords))
        {
            let word = w[i];
            let offset = i * 4usize;
            _encRoundKeys[offset] = NumericUnchecked.ToByte((word >> 24) & 0xFFu);
            _encRoundKeys[offset + 1usize] = NumericUnchecked.ToByte((word >> 16) & 0xFFu);
            _encRoundKeys[offset + 2usize] = NumericUnchecked.ToByte((word >> 8) & 0xFFu);
            _encRoundKeys[offset + 3usize] = NumericUnchecked.ToByte(word & 0xFFu);
            i += 1usize;
        }
        var decWords = new uint[totalWords];
        var roundIndex = 0usize;
        let roundCount = NumericUnchecked.ToUSize(_rounds + 1);
        while (roundIndex <roundCount)
        {
            let encOffset = roundIndex * 4usize;
            let decOffset = (roundCount - 1usize - roundIndex) * 4usize;
            decWords[decOffset] = w[encOffset];
            decWords[decOffset + 1usize] = w[encOffset + 1usize];
            decWords[decOffset + 2usize] = w[encOffset + 2usize];
            decWords[decOffset + 3usize] = w[encOffset + 3usize];
            roundIndex += 1usize;
        }
        i = 4usize;
        while (i <NumericUnchecked.ToUSize (totalWords - 4))
        {
            decWords[i] = InvMixColumnsWord(decWords[i]);
            i += 1usize;
        }
        _decRoundKeys = new byte[NumericUnchecked.ToUSize(roundKeyLength)];
        i = 0usize;
        while (i <NumericUnchecked.ToUSize (totalWords))
        {
            let word = decWords[i];
            let offset = i * 4usize;
            _decRoundKeys[offset] = NumericUnchecked.ToByte((word >> 24) & 0xFFu);
            _decRoundKeys[offset + 1usize] = NumericUnchecked.ToByte((word >> 16) & 0xFFu);
            _decRoundKeys[offset + 2usize] = NumericUnchecked.ToByte((word >> 8) & 0xFFu);
            _decRoundKeys[offset + 3usize] = NumericUnchecked.ToByte(word & 0xFFu);
            i += 1usize;
        }
    }
    private void EncryptBlockWithCbc(ReadOnlySpan <byte >input, Span <byte >output) {
        var state = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        state.CopyFrom(input);
        XorWithVector(state, ReadOnlySpan <byte >.FromArray(ref _currentVector));
        EncryptBlock(state);
        output.Slice(0usize, NumericUnchecked.ToUSize(BlockSize)).CopyFrom(state);
        Span <byte >.FromArray(ref _currentVector).CopyFrom(output.Slice(0usize, NumericUnchecked.ToUSize(BlockSize)));
    }
    private void DecryptBlockWithCbc(ReadOnlySpan <byte >input, Span <byte >output) {
        var state = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        state.CopyFrom(input);
        var cipherCopy = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        cipherCopy.CopyFrom(input);
        DecryptBlock(state);
        XorWithVector(state, ReadOnlySpan <byte >.FromArray(ref _currentVector));
        output.Slice(0usize, NumericUnchecked.ToUSize(BlockSize)).CopyFrom(state);
        Span <byte >.FromArray(ref _currentVector).CopyFrom(cipherCopy);
    }
    private void EncryptBlock(Span <byte >state) {
        AddRoundKey(state, _encRoundKeys, 0usize);
        var round = 1usize;
        while (round <NumericUnchecked.ToUSize (_rounds))
        {
            SubBytes(state);
            ShiftRows(state);
            MixColumns(state);
            AddRoundKey(state, _encRoundKeys, round * NumericUnchecked.ToUSize(BlockSize));
            round += 1usize;
        }
        SubBytes(state);
        ShiftRows(state);
        AddRoundKey(state, _encRoundKeys, NumericUnchecked.ToUSize(_rounds) * NumericUnchecked.ToUSize(BlockSize));
    }
    private void DecryptBlock(Span <byte >state) {
        AddRoundKey(state, _decRoundKeys, 0usize);
        var round = 1usize;
        while (round <NumericUnchecked.ToUSize (_rounds))
        {
            InvShiftRows(state);
            InvSubBytes(state);
            AddRoundKey(state, _decRoundKeys, round * NumericUnchecked.ToUSize(BlockSize));
            InvMixColumns(state);
            round += 1usize;
        }
        InvShiftRows(state);
        InvSubBytes(state);
        AddRoundKey(state, _decRoundKeys, NumericUnchecked.ToUSize(_rounds) * NumericUnchecked.ToUSize(BlockSize));
    }
    private static void AddRoundKey(Span <byte >state, byte[] roundKeys, usize offset) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = (byte)(state[idx] ^ roundKeys[offset + idx]);
            idx += 1usize;
        }
    }
    private void SubBytes(Span <byte >state) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = _sBox[state[idx]];
            idx += 1usize;
        }
    }
    private void InvSubBytes(Span <byte >state) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = _invSBox[state[idx]];
            idx += 1usize;
        }
    }
    private static void ShiftRows(Span <byte >state) {
        let t1 = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = t1;
        let t2 = state[2];
        let t6 = state[6];
        state[2] = state[10];
        state[6] = state[14];
        state[10] = t2;
        state[14] = t6;
        let t3 = state[3];
        state[3] = state[15];
        state[15] = state[11];
        state[11] = state[7];
        state[7] = t3;
    }
    private static void InvShiftRows(Span <byte >state) {
        let t1 = state[13];
        state[13] = state[9];
        state[9] = state[5];
        state[5] = state[1];
        state[1] = t1;
        let t2 = state[2];
        let t6 = state[6];
        state[2] = state[10];
        state[6] = state[14];
        state[10] = t2;
        state[14] = t6;
        let t3 = state[3];
        state[3] = state[7];
        state[7] = state[11];
        state[11] = state[15];
        state[15] = t3;
    }
    private static void MixColumns(Span <byte >state) {
        var col = 0usize;
        while (col <4usize)
        {
            let i = col * 4usize;
            let s0 = state[i];
            let s1 = state[i + 1usize];
            let s2 = state[i + 2usize];
            let s3 = state[i + 3usize];
            let t = (byte)(s0 ^ s1 ^ s2 ^ s3);
            let u = s0;
            state[i] = (byte)(s0 ^ t ^ XTime((byte)(s0 ^ s1)));
            state[i + 1usize] = (byte)(s1 ^ t ^ XTime((byte)(s1 ^ s2)));
            state[i + 2usize] = (byte)(s2 ^ t ^ XTime((byte)(s2 ^ s3)));
            state[i + 3usize] = (byte)(s3 ^ t ^ XTime((byte)(s3 ^ u)));
            col += 1usize;
        }
    }
    private static void InvMixColumns(Span <byte >state) {
        var col = 0usize;
        while (col <4usize)
        {
            let i = col * 4usize;
            let s0 = state[i];
            let s1 = state[i + 1usize];
            let s2 = state[i + 2usize];
            let s3 = state[i + 3usize];
            state[i] = (byte)(Multiply(0x0eu8, s0) ^ Multiply(0x0bu8, s1) ^ Multiply(0x0du8, s2) ^ Multiply(0x09u8, s3));
            state[i + 1usize] = (byte)(Multiply(0x09u8, s0) ^ Multiply(0x0eu8, s1) ^ Multiply(0x0bu8, s2) ^ Multiply(0x0du8,
            s3));
            state[i + 2usize] = (byte)(Multiply(0x0du8, s0) ^ Multiply(0x09u8, s1) ^ Multiply(0x0eu8, s2) ^ Multiply(0x0bu8,
            s3));
            state[i + 3usize] = (byte)(Multiply(0x0bu8, s0) ^ Multiply(0x0du8, s1) ^ Multiply(0x09u8, s2) ^ Multiply(0x0eu8,
            s3));
            col += 1usize;
        }
    }
    private uint SubWord(uint word) {
        let b0 = _sBox[(word >> 24) & 0xFFu];
        let b1 = _sBox[(word >> 16) & 0xFFu];
        let b2 = _sBox[(word >> 8) & 0xFFu];
        let b3 = _sBox[word & 0xFFu];
        let result = (NumericUnchecked.ToUInt32(b0) << 24) | (NumericUnchecked.ToUInt32(b1) << 16) | (NumericUnchecked.ToUInt32(b2) << 8) | NumericUnchecked.ToUInt32(b3);
        return result;
    }
    private static uint RotWord(uint word) {
        return(word << 8) | (word >> 24);
    }
    private static byte NextRcon(byte current) {
        var value = NumericUnchecked.ToUInt32(current) << 1;
        if ( (value & 0x100u) != 0u)
        {
            value = value ^ 0x11Bu;
        }
        return NumericUnchecked.ToByte(value & 0xFFu);
    }
    private static uint InvMixColumnsWord(uint word) {
        let b0 = NumericUnchecked.ToByte((word >> 24) & 0xFFu);
        let b1 = NumericUnchecked.ToByte((word >> 16) & 0xFFu);
        let b2 = NumericUnchecked.ToByte((word >> 8) & 0xFFu);
        let b3 = NumericUnchecked.ToByte(word & 0xFFu);
        let r0 = NumericUnchecked.ToUInt32(Multiply(0x0eu8, b0) ^ Multiply(0x0bu8, b1) ^ Multiply(0x0du8, b2) ^ Multiply(0x09u8,
        b3));
        let r1 = NumericUnchecked.ToUInt32(Multiply(0x09u8, b0) ^ Multiply(0x0eu8, b1) ^ Multiply(0x0bu8, b2) ^ Multiply(0x0du8,
        b3));
        let r2 = NumericUnchecked.ToUInt32(Multiply(0x0du8, b0) ^ Multiply(0x09u8, b1) ^ Multiply(0x0eu8, b2) ^ Multiply(0x0bu8,
        b3));
        let r3 = NumericUnchecked.ToUInt32(Multiply(0x0bu8, b0) ^ Multiply(0x0du8, b1) ^ Multiply(0x09u8, b2) ^ Multiply(0x0eu8,
        b3));
        return(r0 << 24) | (r1 << 16) | (r2 << 8) | r3;
    }
    private static void XorWithVector(Span <byte >state, ReadOnlySpan <byte >vector) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = (byte)(state[idx] ^ vector[idx]);
            idx += 1usize;
        }
    }
    private static byte XTime(byte value) {
        let shifted = NumericUnchecked.ToUInt32(value) << 1;
        if ( (value & 0x80u8) != 0u8)
        {
            return NumericUnchecked.ToByte((shifted ^ 0x1Bu) & 0xFFu);
        }
        return NumericUnchecked.ToByte(shifted & 0xFFu);
    }
    private static byte Multiply(byte a, byte b) {
        var result = 0u8;
        var value = a;
        var factor = b;
        var i = 0usize;
        while (i <8usize)
        {
            if ( (factor & 1u8) != 0u8)
            {
                result = (byte)(result ^ value);
            }
            let high = (value & 0x80u8) != 0u8;
            value = (byte)((value << 1) & 0xFFu8);
            if (high)
            {
                value = (byte)(value ^ 0x1Bu8);
            }
            factor = (byte)(factor >> 1);
            i += 1usize;
        }
        return result;
    }
}
