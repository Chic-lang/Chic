namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>AES-GCM authenticated encryption built in Chic.</summary>
public sealed class AesGcm
{
    private const int BlockSize = 16;
    private readonly byte[] _sBox;
    private readonly byte[] _encRoundKeys;
    private readonly int _rounds;
    private readonly byte[] _hashSubKey;
    public init(ReadOnlySpan <byte >key) {
        ValidateKey(key);
        _sBox = InitSBox();
        var rounds = 0;
        _encRoundKeys = BuildEncRoundKeys(key, out rounds);
        _rounds = rounds;
        _hashSubKey = new byte[NumericUnchecked.ToUSize(BlockSize)];
        var zero = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        ClearSpan(zero);
        EncryptBlock(zero.AsReadOnly(), Span <byte >.FromArray(ref _hashSubKey));
    }
    public void Encrypt(ReadOnlySpan <byte >nonce, ReadOnlySpan <byte >plaintext, Span <byte >ciphertext, Span <byte >tag) {
        Encrypt(nonce, plaintext, ciphertext, tag, ReadOnlySpan <byte >.Empty);
    }
    public void Encrypt(ReadOnlySpan <byte >nonce, ReadOnlySpan <byte >plaintext, Span <byte >ciphertext, Span <byte >tag,
    ReadOnlySpan <byte >associatedData) {
        if (ciphertext.Length <plaintext.Length)
        {
            throw new Std.ArgumentException("ciphertext buffer too small");
        }
        if (tag.Length <BlockSize)
        {
            throw new Std.ArgumentException("tag buffer too small");
        }
        var j0 = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        BuildJ0(nonce, j0);
        var counter = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        counter.CopyFrom(j0);
        IncrementCounter(counter);
        var keystream = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        var offset = 0usize;
        while (offset <plaintext.Length)
        {
            EncryptBlock(counter.AsReadOnly(), keystream);
            let remaining = plaintext.Length - offset;
            let block = remaining >NumericUnchecked.ToUSize(BlockSize) ?NumericUnchecked.ToUSize(BlockSize) : remaining;
            XorInto(keystream, plaintext.Slice(offset, block), ciphertext.Slice(offset, block));
            IncrementCounter(counter);
            offset += block;
        }
        var s = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        ComputeAuthentication(associatedData, ciphertext.Slice(0usize, plaintext.Length), s);
        var tagBlock = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        EncryptBlock(j0.AsReadOnly(), tagBlock);
        XorBlocks(tagBlock, s);
        tag.Slice(0usize, NumericUnchecked.ToUSize(BlockSize)).CopyFrom(tagBlock);
    }
    public void Decrypt(ReadOnlySpan <byte >nonce, ReadOnlySpan <byte >ciphertext, ReadOnlySpan <byte >tag, Span <byte >plaintext) {
        Decrypt(nonce, ciphertext, tag, plaintext, ReadOnlySpan <byte >.Empty);
    }
    public void Decrypt(ReadOnlySpan <byte >nonce, ReadOnlySpan <byte >ciphertext, ReadOnlySpan <byte >tag, Span <byte >plaintext,
    ReadOnlySpan <byte >associatedData) {
        if (plaintext.Length <ciphertext.Length)
        {
            throw new Std.ArgumentException("plaintext buffer too small");
        }
        if (tag.Length <BlockSize)
        {
            throw new Std.ArgumentException("authentication tag missing");
        }
        var j0 = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        BuildJ0(nonce, j0);
        var s = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        ComputeAuthentication(associatedData, ciphertext, s);
        var tagBlock = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        EncryptBlock(j0.AsReadOnly(), tagBlock);
        XorBlocks(tagBlock, s);
        let providedTag = tag.Slice(0usize, NumericUnchecked.ToUSize(BlockSize));
        if (! ConstantTimeEquals (tagBlock, providedTag))
        {
            throw new Std.InvalidOperationException("authentication failed");
        }
        var counter = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        counter.CopyFrom(j0);
        IncrementCounter(counter);
        var keystream = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        var offset = 0usize;
        while (offset <ciphertext.Length)
        {
            EncryptBlock(counter.AsReadOnly(), keystream);
            let remaining = ciphertext.Length - offset;
            let block = remaining >NumericUnchecked.ToUSize(BlockSize) ?NumericUnchecked.ToUSize(BlockSize) : remaining;
            XorInto(keystream, ciphertext.Slice(offset, block), plaintext.Slice(offset, block));
            IncrementCounter(counter);
            offset += block;
        }
    }
    private void ComputeAuthentication(ReadOnlySpan <byte >associatedData, ReadOnlySpan <byte >cipherText, Span <byte >output) {
        var state = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        ClearSpan(state);
        GHashUpdate(state, associatedData);
        GHashUpdate(state, cipherText);
        var lengths = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        WriteUInt64BigEndian(NumericUnchecked.ToUInt64(associatedData.Length) * 8ul, lengths.Slice(0usize, 8usize));
        WriteUInt64BigEndian(NumericUnchecked.ToUInt64(cipherText.Length) * 8ul, lengths.Slice(8usize, 8usize));
        XorBlocks(state, lengths);
        MultiplyH(state, _hashSubKey, state);
        output.CopyFrom(state);
    }
    private void GHashUpdate(Span <byte >state, ReadOnlySpan <byte >data) {
        if (data.Length == 0usize)
        {
            return;
        }
        var offset = 0usize;
        var block = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        while (offset <data.Length)
        {
            ClearSpan(block);
            let remaining = data.Length - offset;
            let take = remaining >NumericUnchecked.ToUSize(BlockSize) ?NumericUnchecked.ToUSize(BlockSize) : remaining;
            block.Slice(0usize, take).CopyFrom(data.Slice(offset, take));
            XorBlocks(state, block);
            MultiplyH(state, _hashSubKey, state);
            offset += take;
        }
    }
    private void BuildJ0(ReadOnlySpan <byte >nonce, Span <byte >j0) {
        ClearSpan(j0);
        if (nonce.Length == 12usize)
        {
            j0.Slice(0usize, nonce.Length).CopyFrom(nonce);
            j0[12usize] = 0u8;
            j0[13usize] = 0u8;
            j0[14usize] = 0u8;
            j0[15usize] = 1u8;
            return;
        }
        GHashUpdate(j0, nonce);
        var lengths = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(BlockSize));
        WriteUInt64BigEndian(0ul, lengths.Slice(0usize, 8usize));
        WriteUInt64BigEndian(NumericUnchecked.ToUInt64(nonce.Length) * 8ul, lengths.Slice(8usize, 8usize));
        XorBlocks(j0, lengths);
        MultiplyH(j0, _hashSubKey, j0);
    }
    private void IncrementCounter(Span <byte >counter) {
        var idx = NumericUnchecked.ToInt32(BlockSize) - 1;
        var carry = true;
        while (idx >= NumericUnchecked.ToInt32 (BlockSize - 4) && carry)
        {
            let value = (byte)(counter[NumericUnchecked.ToUSize(idx)] + 1u8);
            carry = value == 0u8;
            counter[NumericUnchecked.ToUSize(idx)] = value;
            idx -= 1;
        }
    }
    private static void XorInto(ReadOnlySpan <byte >left, ReadOnlySpan <byte >right, Span <byte >destination) {
        var idx = 0usize;
        while (idx <right.Length)
        {
            destination[idx] = (byte)(right[idx] ^ left[idx]);
            idx += 1usize;
        }
    }
    private static void ClearSpan(Span <byte >span) {
        var idx = 0usize;
        while (idx <span.Length)
        {
            span[idx] = 0u8;
            idx += 1usize;
        }
    }
    private static bool ConstantTimeEquals(ReadOnlySpan <byte >left, ReadOnlySpan <byte >right) {
        if (left.Length != right.Length)
        {
            return false;
        }
        var diff = 0u8;
        var idx = 0usize;
        while (idx <left.Length)
        {
            diff = (byte)(diff | (left[idx] ^ right[idx]));
            idx += 1usize;
        }
        return diff == 0u8;
    }
    private void MultiplyH(ReadOnlySpan <byte >x, ReadOnlySpan <byte >y, Span <byte >output) {
        var zh = 0ul;
        var zl = 0ul;
        let yh = ReadUInt64BigEndian(y.Slice(0usize, 8usize));
        let yl = ReadUInt64BigEndian(y.Slice(8usize, 8usize));
        var vh = yh;
        var vl = yl;
        var byteIdx = 0usize;
        while (byteIdx <NumericUnchecked.ToUSize (BlockSize))
        {
            var bit = 0usize;
            let b = x[byteIdx];
            while (bit <8usize)
            {
                let mask = 0x80u8 >> NumericUnchecked.ToUInt32(bit);
                let xi = (b & mask) != 0u8;
                if (xi)
                {
                    zh = zh ^ vh;
                    zl = zl ^ vl;
                }
                let lsb = (vl & 1ul) != 0ul;
                vl = (vh << 63) | (vl >> 1);
                vh = (vh >> 1);
                if (lsb)
                {
                    vh = vh ^ 0xE100000000000000ul;
                }
                bit += 1usize;
            }
            byteIdx += 1usize;
        }
        WriteUInt64BigEndian(zh, output.Slice(0usize, 8usize));
        WriteUInt64BigEndian(zl, output.Slice(8usize, 8usize));
    }
    private static ulong ReadUInt64BigEndian(ReadOnlySpan <byte >source) {
        return(NumericUnchecked.ToUInt64(source[0usize]) << 56) | (NumericUnchecked.ToUInt64(source[1usize]) << 48) | (NumericUnchecked.ToUInt64(source[2usize]) << 40) | (NumericUnchecked.ToUInt64(source[3usize]) << 32) | (NumericUnchecked.ToUInt64(source[4usize]) << 24) | (NumericUnchecked.ToUInt64(source[5usize]) << 16) | (NumericUnchecked.ToUInt64(source[6usize]) << 8) | NumericUnchecked.ToUInt64(source[7usize]);
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
    private static void XorBlocks(Span <byte >state, ReadOnlySpan <byte >block) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = (byte)(state[idx] ^ block[idx]);
            idx += 1usize;
        }
    }
    private static byte[] InitSBox() {
        return new byte[] {
            0x63u8, 0x7cu8, 0x77u8, 0x7bu8, 0xf2u8, 0x6bu8, 0x6fu8, 0xc5u8, 0x30u8, 0x01u8, 0x67u8, 0x2bu8, 0xfeu8, 0xd7u8, 0xabu8, 0x76u8, 0xcau8, 0x82u8, 0xc9u8, 0x7du8, 0xfau8, 0x59u8, 0x47u8, 0xf0u8, 0xadu8, 0xd4u8, 0xa2u8, 0xafu8, 0x9cu8, 0xa4u8, 0x72u8, 0xc0u8, 0xb7u8, 0xfdu8, 0x93u8, 0x26u8, 0x36u8, 0x3fu8, 0xf7u8, 0xccu8, 0x34u8, 0xa5u8, 0xe5u8, 0xf1u8, 0x71u8, 0xd8u8, 0x31u8, 0x15u8, 0x04u8, 0xc7u8, 0x23u8, 0xc3u8, 0x18u8, 0x96u8, 0x05u8, 0x9au8, 0x07u8, 0x12u8, 0x80u8, 0xe2u8, 0xebu8, 0x27u8, 0xb2u8, 0x75u8, 0x09u8, 0x83u8, 0x2cu8, 0x1au8, 0x1bu8, 0x6eu8, 0x5au8, 0xa0u8, 0x52u8, 0x3bu8, 0xd6u8, 0xb3u8, 0x29u8, 0xe3u8, 0x2fu8, 0x84u8, 0x53u8, 0xd1u8, 0x00u8, 0xedu8, 0x20u8, 0xfcu8, 0xb1u8, 0x5bu8, 0x6au8, 0xcbu8, 0xbeu8, 0x39u8, 0x4au8, 0x4cu8, 0x58u8, 0xcfu8, 0xd0u8, 0xefu8, 0xaau8, 0xfbu8, 0x43u8, 0x4du8, 0x33u8, 0x85u8, 0x45u8, 0xf9u8, 0x02u8, 0x7fu8, 0x50u8, 0x3cu8, 0x9fu8, 0xa8u8, 0x51u8, 0xa3u8, 0x40u8, 0x8fu8, 0x92u8, 0x9du8, 0x38u8, 0xf5u8, 0xbcu8, 0xb6u8, 0xdau8, 0x21u8, 0x10u8, 0xffu8, 0xf3u8, 0xd2u8, 0xcdu8, 0x0cu8, 0x13u8, 0xecu8, 0x5fu8, 0x97u8, 0x44u8, 0x17u8, 0xc4u8, 0xa7u8, 0x7eu8, 0x3du8, 0x64u8, 0x5du8, 0x19u8, 0x73u8, 0x60u8, 0x81u8, 0x4fu8, 0xdcu8, 0x22u8, 0x2au8, 0x90u8, 0x88u8, 0x46u8, 0xeeu8, 0xb8u8, 0x14u8, 0xdeu8, 0x5eu8, 0x0bu8, 0xdbu8, 0xe0u8, 0x32u8, 0x3au8, 0x0au8, 0x49u8, 0x06u8, 0x24u8, 0x5cu8, 0xc2u8, 0xd3u8, 0xacu8, 0x62u8, 0x91u8, 0x95u8, 0xe4u8, 0x79u8, 0xe7u8, 0xc8u8, 0x37u8, 0x6du8, 0x8du8, 0xd5u8, 0x4eu8, 0xa9u8, 0x6cu8, 0x56u8, 0xf4u8, 0xeau8, 0x65u8, 0x7au8, 0xaeu8, 0x08u8, 0xbau8, 0x78u8, 0x25u8, 0x2eu8, 0x1cu8, 0xa6u8, 0xb4u8, 0xc6u8, 0xe8u8, 0xddu8, 0x74u8, 0x1fu8, 0x4bu8, 0xbdu8, 0x8bu8, 0x8au8, 0x70u8, 0x3eu8, 0xb5u8, 0x66u8, 0x48u8, 0x03u8, 0xf6u8, 0x0eu8, 0x61u8, 0x35u8, 0x57u8, 0xb9u8, 0x86u8, 0xc1u8, 0x1du8, 0x9eu8, 0xe1u8, 0xf8u8, 0x98u8, 0x11u8, 0x69u8, 0xd9u8, 0x8eu8, 0x94u8, 0x9bu8, 0x1eu8, 0x87u8, 0xe9u8, 0xceu8, 0x55u8, 0x28u8, 0xdfu8, 0x8cu8, 0xa1u8, 0x89u8, 0x0du8, 0xbfu8, 0xe6u8, 0x42u8, 0x68u8, 0x41u8, 0x99u8, 0x2du8, 0x0fu8, 0xb0u8, 0x54u8, 0xbbu8, 0x16u8,
        }
        ;
    }
    private byte[] BuildEncRoundKeys(ReadOnlySpan <byte >key, out int rounds) {
        let keyWords = NumericUnchecked.ToInt32(key.Length) / 4;
        rounds = keyWords + 6;
        let totalWords = 4 * (rounds + 1);
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
        let roundKeyBytes = (NumericUnchecked.ToUSize(rounds + 1) * NumericUnchecked.ToUSize(BlockSize));
        let roundKeyLength = NumericUnchecked.ToInt32(roundKeyBytes);
        var encRoundKeys = new byte[NumericUnchecked.ToUSize(roundKeyLength)];
        i = 0usize;
        while (i <NumericUnchecked.ToUSize (totalWords))
        {
            let word = w[i];
            let offset = i * 4usize;
            encRoundKeys[offset] = NumericUnchecked.ToByte((word >> 24) & 0xFFu);
            encRoundKeys[offset + 1usize] = NumericUnchecked.ToByte((word >> 16) & 0xFFu);
            encRoundKeys[offset + 2usize] = NumericUnchecked.ToByte((word >> 8) & 0xFFu);
            encRoundKeys[offset + 3usize] = NumericUnchecked.ToByte(word & 0xFFu);
            i += 1usize;
        }
        return encRoundKeys;
    }
    private void EncryptBlock(ReadOnlySpan <byte >input, Span <byte >output) {
        output.Slice(0usize, NumericUnchecked.ToUSize(BlockSize)).CopyFrom(input);
        AddRoundKey(output, _encRoundKeys, 0usize);
        var round = 1usize;
        while (round <NumericUnchecked.ToUSize (_rounds))
        {
            SubBytes(output);
            ShiftRows(output);
            MixColumns(output);
            AddRoundKey(output, _encRoundKeys, round * NumericUnchecked.ToUSize(BlockSize));
            round += 1usize;
        }
        SubBytes(output);
        ShiftRows(output);
        AddRoundKey(output, _encRoundKeys, NumericUnchecked.ToUSize(_rounds) * NumericUnchecked.ToUSize(BlockSize));
    }
    private void SubBytes(Span <byte >state) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = _sBox[state[idx]];
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
    private static void AddRoundKey(Span <byte >state, byte[] roundKeys, usize offset) {
        var idx = 0usize;
        while (idx <NumericUnchecked.ToUSize (BlockSize))
        {
            state[idx] = (byte)(state[idx] ^ roundKeys[offset + idx]);
            idx += 1usize;
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
    private static byte XTime(byte value) {
        let shifted = NumericUnchecked.ToUInt32(value) << 1;
        if ( (value & 0x80u8) != 0u8)
        {
            return NumericUnchecked.ToByte((shifted ^ 0x1Bu) & 0xFFu);
        }
        return NumericUnchecked.ToByte(shifted & 0xFFu);
    }
    private static void ValidateKey(ReadOnlySpan <byte >key) {
        if (key.Length != 16usize && key.Length != 24usize && key.Length != 32usize)
        {
            throw new Std.ArgumentException("AES-GCM key must be 16, 24, or 32 bytes");
        }
    }
}
