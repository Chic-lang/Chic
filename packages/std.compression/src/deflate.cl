namespace Std.IO.Compression;
import Std.Span;
import Std.IO;
import Std;
/// <summary>Span-based deflate codec.</summary>
public static class Deflate
{
    /// <summary>Attempts to compress into the destination span using a deterministic fixed-Huffman encoder.</summary>
    public static bool TryCompress(ReadOnlySpan <byte >src, Span <byte >dst, CompressionLevel level, out int bytesWritten) {
        bytesWritten = 0;
        if (level == CompressionLevel.NoCompression)
        {
            return DeflateEncoder.EmitStored(src, dst, out bytesWritten);
        }
        return DeflateEncoder.EmitFixed(src, dst, out bytesWritten);
    }
    /// <summary>Attempts to decompress a deflate payload into the destination span.</summary>
    public static bool TryDecompress(ReadOnlySpan <byte >src, Span <byte >dst, out int bytesWritten) {
        return DeflateDecoder.TryDecode(src, dst, out bytesWritten);
    }
}
/// <summary>Bit-level writer for deflate streams (LSB-first).</summary>
internal struct BitWriter
{
    private Span <byte >_buffer;
    private int _index;
    private uint _bitBuffer;
    private int _bitCount;
    private bool _overflowed;
    public init(Span <byte >destination) {
        _buffer = destination;
        _index = 0;
        _bitBuffer = 0;
        _bitCount = 0;
        _overflowed = false;
    }
    public bool Overflowed => _overflowed;
    public bool WriteBits(uint value, int count) {
        if (_overflowed)
        {
            return false;
        }
        _bitBuffer |= (value & ((1u << count) - 1u)) << _bitCount;
        _bitCount += count;
        while (_bitCount >= 8)
        {
            if (_index >= _buffer.Length)
            {
                _overflowed = true;
                return false;
            }
            _buffer[_index] = CompressionCast.ToByte(_bitBuffer & 0xFFu);
            _index += 1;
            _bitBuffer >>= 8;
            _bitCount -= 8;
        }
        return true;
    }
    public bool AlignToByte() {
        if (_bitCount >0)
        {
            return WriteBits(0u, 8 - _bitCount);
        }
        return true;
    }
    public bool WriteByte(byte value) {
        if (!AlignToByte ())
        {
            return false;
        }
        if (_index >= _buffer.Length)
        {
            _overflowed = true;
            return false;
        }
        _buffer[_index] = value;
        _index += 1;
        return true;
    }
    public bool WriteBytes(ReadOnlySpan <byte >data) {
        if (!AlignToByte ())
        {
            return false;
        }
        if (_index + data.Length >_buffer.Length)
        {
            _overflowed = true;
            return false;
        }
        _buffer.Slice(CompressionCast.ToUSize(_index), data.Length).CopyFrom(data);
        _index += data.Length;
        return true;
    }
    public int BytesWritten {
        get {
            let rounded = _bitCount == 0 ?0 : 1;
            return _index + rounded;
        }
    }
}
internal static class DeflateEncoder
{
    public static bool EmitStored(ReadOnlySpan <byte >src, Span <byte >dst, out int written) {
        written = 0;
        var writer = new BitWriter(dst);
        var remaining = src.Length;
        var offset = 0usize;
        while (remaining >0)
        {
            let chunk = remaining >65535 ?65535 : remaining;
            let isLast = remaining <= 65535;
            if (!writer.WriteBits (isLast ?1u : 0u, 1))
            {
                return false;
            }
            if (!writer.WriteBits (0u, 2))
            {
                return false;
            }
            if (!writer.AlignToByte ())
            {
                return false;
            }
            let len = CompressionCast.ToUInt16(chunk);
            let nlen = CompressionCast.ToUInt16(~ len);
            if (!writer.WriteByte (CompressionCast.ToByte (len & 0xFFu)) || !writer.WriteByte (CompressionCast.ToByte (len >> 8)) || !writer.WriteByte (CompressionCast.ToByte (nlen & 0xFFu)) || !writer.WriteByte (CompressionCast.ToByte (nlen >> 8)))
            {
                return false;
            }
            if (!writer.WriteBytes (src.Slice (offset, CompressionCast.ToUSize (chunk))))
            {
                return false;
            }
            offset += CompressionCast.ToUSize(chunk);
            remaining -= chunk;
        }
        written = writer.BytesWritten;
        return !writer.Overflowed;
    }
    public static bool EmitFixed(ReadOnlySpan <byte >src, Span <byte >dst, out int written) {
        written = 0;
        var writer = new BitWriter(dst);
        // single final block, fixed Huffman
        if (!writer.WriteBits (1u, 1) || !writer.WriteBits (0b01u, 2))
        {
            return false;
        }
        let len = src.Length;
        for (var i = 0usize; i <len; i += 1usize) {
            if (!WriteFixedLiteral (ref writer, src[i])) {
                return false;
            }
        }
        if (!WriteFixedLiteral (ref writer, 256)) {
            return false;
        }
        if (!writer.AlignToByte ())
        {
            return false;
        }
        if (writer.Overflowed)
        {
            return false;
        }
        written = writer.BytesWritten;
        return true;
    }
    private static bool WriteFixedLiteral(ref BitWriter writer, int symbol) {
        if (symbol >= 0 && symbol <= 143)
        {
            let code = CompressionCast.ToUInt32(0b00110000 + symbol);
            return writer.WriteBits(code, 8);
        }
        if (symbol >= 144 && symbol <= 255)
        {
            let code = CompressionCast.ToUInt32(0b110010000 + (symbol - 144));
            return writer.WriteBits(code, 9);
        }
        if (symbol >= 256 && symbol <= 279)
        {
            let code = CompressionCast.ToUInt32(symbol - 256);
            return writer.WriteBits(code, 7);
        }
        // 280-287
        let code280 = CompressionCast.ToUInt32(0b11000000 + (symbol - 280));
        return writer.WriteBits(code280, 8);
    }
}
/// <summary>Bit-level reader for inflate.</summary>
internal struct BitReader
{
    private ReadOnlySpan <byte >_data;
    private int _index;
    private uint _bitBuffer;
    private int _bitCount;
    public init(ReadOnlySpan <byte >data) {
        _data = data;
        _index = 0;
        _bitBuffer = 0;
        _bitCount = 0;
    }
    public bool TryReadBits(int count, out uint value) {
        value = 0u;
        while (_bitCount <count)
        {
            if (_index >= _data.Length)
            {
                return false;
            }
            _bitBuffer |= CompressionCast.ToUInt32(_data[_index]) << _bitCount;
            _bitCount += 8;
            _index += 1;
        }
        value = _bitBuffer & ((1u << count) - 1u);
        _bitBuffer >>= count;
        _bitCount -= count;
        return true;
    }
    public bool AlignToByte() {
        let drop = _bitCount % 8;
        if (drop == 0)
        {
            return true;
        }
        return TryReadBits(drop, out var _);
    }
    public bool ReadByte(out byte value) {
        value = 0u8;
        if (_bitCount >= 8)
        {
            value = CompressionCast.ToByte(_bitBuffer & 0xFFu);
            _bitBuffer >>= 8;
            _bitCount -= 8;
            return true;
        }
        if (_index >= _data.Length)
        {
            return false;
        }
        value = _data[_index];
        _index += 1;
        return true;
    }
}
/// <summary>Fixed-size Huffman decoder.</summary>
internal struct HuffmanTable
{
    public uint[] Codes;
    public byte[] CodeLengths;
    public int MaxBits;
    public bool IsEmpty => Codes.Length == 0;
    internal static HuffmanTable Create(int count) {
        return new HuffmanTable() {
            Codes = new uint[count], CodeLengths = new byte[count], MaxBits = 0,
        }
        ;
    }
}
internal static class DeflateDecoder
{
    private static bool _tablesReady;
    private static int[] _lengthBases;
    private static int[] _lengthExtra;
    private static int[] _distBases;
    private static int[] _distExtra;
    private static void EnsureTables() {
        if (_tablesReady)
        {
            return;
        }
        _lengthBases = new int[] {
            3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258
        }
        ;
        _lengthExtra = new int[] {
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0
        }
        ;
        _distBases = new int[] {
            1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577
        }
        ;
        _distExtra = new int[] {
            0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13
        }
        ;
        _tablesReady = true;
    }
    private static int[] GetLengthBases() {
        if (_lengthBases == null)
        {
            EnsureTables();
        }
        return _lengthBases;
    }
    private static int[] GetLengthExtra() {
        if (_lengthExtra == null)
        {
            EnsureTables();
        }
        return _lengthExtra;
    }
    private static int[] GetDistBases() {
        if (_distBases == null)
        {
            EnsureTables();
        }
        return _distBases;
    }
    private static int[] GetDistExtra() {
        if (_distExtra == null)
        {
            EnsureTables();
        }
        return _distExtra;
    }
    public static bool TryDecode(ReadOnlySpan <byte >input, Span <byte >output, out int written) {
        written = 0;
        var reader = new BitReader(input);
        var outIndex = 0usize;
        var last = false;
        while (!last)
        {
            if (!reader.TryReadBits (1, out var bfinal)) {
                return false;
            }
            last = bfinal == 1u;
            if (!reader.TryReadBits (2, out var btype)) {
                return false;
            }
            if (btype == 0)
            {
                if (!reader.AlignToByte ())
                {
                    return false;
                }
                if (!reader.ReadByte (out var lenLo) || !reader.ReadByte(out var lenHi) || !reader.ReadByte(out var nlenLo) || !reader.ReadByte(out var nlenHi)) {
                    return false;
                }
                let len = CompressionCast.ToInt32((ushort)(lenLo | (CompressionCast.ToUInt16(lenHi) << 8)));
                let nlen = CompressionCast.ToInt32((ushort)(nlenLo | (CompressionCast.ToUInt16(nlenHi) << 8)));
                if ( (len ^ 0xFFFF) != nlen)
                {
                    return false;
                }
                if (outIndex + CompressionCast.ToUSize (len) >output.Length)
                {
                    return false;
                }
                for (var i = 0; i <len; i += 1) {
                    if (!reader.ReadByte (out var val)) {
                        return false;
                    }
                    output[outIndex] = val;
                    outIndex += 1usize;
                }
                continue;
            }
            var litLen = HuffmanTable.Create(0);
            var dist = HuffmanTable.Create(0);
            if (btype == 1)
            {
                litLen = BuildTable(BuildFixedLitLengths());
                dist = BuildTable(BuildFixedDistLengths());
            }
            else if (btype == 2)
            {
                if (!BuildDynamicTables (ref reader, out litLen, out dist)) {
                    return false;
                }
            }
            else
            {
                return false;
            }
            while (true)
            {
                if (!ReadSymbol (ref reader, litLen, out var sym)) {
                    return false;
                }
                if (sym <256)
                {
                    if (outIndex >= output.Length)
                    {
                        return false;
                    }
                    output[outIndex] = CompressionCast.ToByte(sym);
                    outIndex += 1usize;
                    continue;
                }
                if (sym == 256)
                {
                    break;
                }
                let lenIndex = sym - 257;
                let lengthBases = GetLengthBases();
                let lengthExtra = GetLengthExtra();
                if (lenIndex <0 || lenIndex >= lengthBases.Length)
                {
                    return false;
                }
                let baseLen = lengthBases[lenIndex];
                let extra = lengthExtra[lenIndex];
                var extraBits = 0u;
                if (extra >0 && !reader.TryReadBits (extra, out extraBits)) {
                    return false;
                }
                let matchLen = baseLen + CompressionCast.ToInt32(extraBits);
                if (!ReadSymbol (ref reader, dist, out var distSym)) {
                    return false;
                }
                let distBases = GetDistBases();
                let distExtraTable = GetDistExtra();
                if (distSym <0 || distSym >= distBases.Length)
                {
                    return false;
                }
                let distBase = distBases[distSym];
                let distExtra = distExtraTable[distSym];
                var distExtraBits = 0u;
                if (distExtra >0 && !reader.TryReadBits (distExtra, out distExtraBits)) {
                    return false;
                }
                let distance = distBase + CompressionCast.ToInt32(distExtraBits);
                if (distance <= 0 || distance >CompressionCast.ToInt32 (outIndex))
                {
                    return false;
                }
                if (outIndex + CompressionCast.ToUSize (matchLen) >output.Length)
                {
                    return false;
                }
                for (var i = 0; i <matchLen; i += 1) {
                    output[outIndex] = output[outIndex - CompressionCast.ToUSize(distance)];
                    outIndex += 1usize;
                }
            }
        }
        written = CompressionCast.ToInt32(outIndex);
        return true;
    }
    private static HuffmanTable BuildTable(byte[] lengths) {
        let table = HuffmanTable.Create(lengths.Length);
        table.MaxBits = 0;
        for (var i = 0usize; i <lengths.Length; i += 1usize) {
            table.CodeLengths[i] = lengths[i];
            if (lengths[i] >table.MaxBits)
            {
                table.MaxBits = lengths[i];
            }
        }
        // canonical codes
        var blCount = new int[table.MaxBits + 1];
        for (var i = 0usize; i <lengths.Length; i += 1usize) {
            let len = lengths[i];
            if (len >0)
            {
                blCount[len] += 1;
            }
        }
        var code = 0u;
        var nextCode = new uint[table.MaxBits + 1];
        for (var bits = 1; bits <= table.MaxBits; bits += 1) {
            code = (code + CompressionCast.ToUInt32(blCount[bits - 1])) << 1;
            nextCode[bits] = code;
        }
        for (var n = 0usize; n <lengths.Length; n += 1usize) {
            let len = lengths[n];
            if (len != 0u8)
            {
                table.Codes[n] = nextCode[len];
                nextCode[len] += 1u;
            }
        }
        return table;
    }
    private static bool BuildDynamicTables(ref BitReader reader, out HuffmanTable litLen, out HuffmanTable dist) {
        litLen = HuffmanTable.Create(0);
        dist = HuffmanTable.Create(0);
        if (!reader.TryReadBits (5, out var hlit) || !reader.TryReadBits(5, out var hdist) || !reader.TryReadBits(4, out var hclen)) {
            return false;
        }
        let litCodes = 257 + CompressionCast.ToInt32(hlit);
        let distCodes = 1 + CompressionCast.ToInt32(hdist);
        let codeLenCodes = 4 + CompressionCast.ToInt32(hclen);
        var codeLengthOrder = new int[] {
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
        }
        ;
        var codeLengths = new byte[19];
        for (var i = 0; i <codeLenCodes; i += 1) {
            if (!reader.TryReadBits (3, out var len)) {
                return false;
            }
            codeLengths[CompressionCast.ToUSize(codeLengthOrder[i])] = CompressionCast.ToByte(len);
        }
        var codeLenTable = BuildTable(codeLengths);
        var litLenLengths = new byte[litCodes];
        var distLengths = new byte[distCodes];
        if (!ReadLengthList (ref reader, codeLenTable, litLenLengths)) {
            return false;
        }
        if (!ReadLengthList (ref reader, codeLenTable, distLengths)) {
            return false;
        }
        litLen = BuildTable(litLenLengths);
        dist = BuildTable(distLengths);
        return true;
    }
    private static bool ReadLengthList(ref BitReader reader, HuffmanTable codeLenTable, byte[] destination) {
        var index = 0;
        while (index <destination.Length)
        {
            if (!ReadSymbol (ref reader, codeLenTable, out var sym)) {
                return false;
            }
            if (sym <= 15)
            {
                destination[index] = CompressionCast.ToByte(sym);
                index += 1;
            }
            else if (sym == 16)
            {
                if (index == 0)
                {
                    return false;
                }
                if (!reader.TryReadBits (2, out var repeatBits)) {
                    return false;
                }
                let repeat = 3 + CompressionCast.ToInt32(repeatBits);
                let value = destination[index - 1];
                for (var i = 0; i <repeat && index <destination.Length; i += 1) {
                    destination[index] = value;
                    index += 1;
                }
            }
            else if (sym == 17)
            {
                if (!reader.TryReadBits (3, out var repeatBits)) {
                    return false;
                }
                let repeat = 3 + CompressionCast.ToInt32(repeatBits);
                for (var i = 0; i <repeat && index <destination.Length; i += 1) {
                    destination[index] = 0u8;
                    index += 1;
                }
            }
            else if (sym == 18)
            {
                if (!reader.TryReadBits (7, out var repeatBits)) {
                    return false;
                }
                let repeat = 11 + CompressionCast.ToInt32(repeatBits);
                for (var i = 0; i <repeat && index <destination.Length; i += 1) {
                    destination[index] = 0u8;
                    index += 1;
                }
            }
            else
            {
                return false;
            }
        }
        return true;
    }
    private static bool ReadSymbol(ref BitReader reader, HuffmanTable table, out int symbol) {
        symbol = 0;
        var code = 0u;
        for (var bits = 1; bits <= table.MaxBits; bits += 1) {
            if (!reader.TryReadBits (1, out var bit)) {
                return false;
            }
            code |= bit << (bits - 1);
            for (var i = 0usize; i <table.Codes.Length; i += 1usize) {
                if (table.CodeLengths[i] == bits && table.Codes[i] == code)
                {
                    symbol = CompressionCast.ToInt32(i);
                    return true;
                }
            }
        }
        return false;
    }
    private static byte[] BuildFixedLitLengths() {
        var lengths = new byte[288];
        for (var i = 0; i <= 143; i += 1) {
            lengths[i] = 8u8;
        }
        for (var i = 144; i <= 255; i += 1) {
            lengths[i] = 9u8;
        }
        for (var i = 256; i <= 279; i += 1) {
            lengths[i] = 7u8;
        }
        for (var i = 280; i <= 287; i += 1) {
            lengths[i] = 8u8;
        }
        return lengths;
    }
    private static byte[] BuildFixedDistLengths() {
        var lengths = new byte[32];
        for (var i = 0; i <lengths.Length; i += 1) {
            lengths[i] = 5u8;
        }
        return lengths;
    }
}
