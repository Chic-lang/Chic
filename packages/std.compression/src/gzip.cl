namespace Std.IO.Compression;
import Std.Span;
/// <summary>Span-based gzip codec.</summary>
public static class GZip
{
    private const byte Id1 = 0x1Fu8;
    private const byte Id2 = 0x8Bu8;
    private const byte MethodDeflate = 8u8;
    /// <summary>Attempts to compress the source into gzip format.</summary>
    public static bool TryCompress(ReadOnlySpan <byte >src, Span <byte >dst, CompressionLevel level, out int bytesWritten) {
        bytesWritten = 0;
        if (dst.Length <18)
        {
            return false;
        }
        dst[0usize] = CompressionCast.ToByte(0x1F);
        dst[1usize] = CompressionCast.ToByte(0x8B);
        dst[2usize] = CompressionCast.ToByte(8);
        dst[3usize] = CompressionCast.ToByte(0);
        dst[4usize] = CompressionCast.ToByte(0);
        // MTIME (zero)
        dst[5usize] = CompressionCast.ToByte(0);
        dst[6usize] = CompressionCast.ToByte(0);
        dst[7usize] = CompressionCast.ToByte(0);
        dst[8usize] = CompressionCast.ToByte(0);
        // XFL
        dst[9usize] = CompressionCast.ToByte(255);
        let payloadDest = dst.Slice(10usize);
        if (! Deflate.TryCompress (src, payloadDest.Slice (0usize, payloadDest.Length - 8), level, out var deflateWritten)) {
            return false;
        }
        var trailerIndex = 10 + deflateWritten;
        if (trailerIndex + 8 >dst.Length)
        {
            return false;
        }
        var crc = Crc32.Compute(src);
        dst[CompressionCast.ToUSize(trailerIndex + 0)] = CompressionCast.ToByte((long)(crc & 0xFFu));
        dst[CompressionCast.ToUSize(trailerIndex + 1)] = CompressionCast.ToByte((long)((crc >> 8) & 0xFFu));
        dst[CompressionCast.ToUSize(trailerIndex + 2)] = CompressionCast.ToByte((long)((crc >> 16) & 0xFFu));
        dst[CompressionCast.ToUSize(trailerIndex + 3)] = CompressionCast.ToByte((long)((crc >> 24) & 0xFFu));
        let isize = CompressionCast.ToUInt32(src.Length & 0xFFFFFFFF);
        dst[CompressionCast.ToUSize(trailerIndex + 4)] = CompressionCast.ToByte((long)(isize & 0xFFu));
        dst[CompressionCast.ToUSize(trailerIndex + 5)] = CompressionCast.ToByte((long)((isize >> 8) & 0xFFu));
        dst[CompressionCast.ToUSize(trailerIndex + 6)] = CompressionCast.ToByte((long)((isize >> 16) & 0xFFu));
        dst[CompressionCast.ToUSize(trailerIndex + 7)] = CompressionCast.ToByte((long)((isize >> 24) & 0xFFu));
        bytesWritten = trailerIndex + 8;
        return true;
    }
    /// <summary>Attempts to decompress a gzip payload.</summary>
    public static bool TryDecompress(ReadOnlySpan <byte >src, Span <byte >dst, out int bytesWritten) {
        bytesWritten = 0;
        if (src.Length <18)
        {
            return false;
        }
        if (src[0usize] != Id1 || src[1usize] != Id2 || src[2usize] != MethodDeflate)
        {
            return false;
        }
        let flags = src[3usize];
        if (flags != 0u8)
        {
            return false;
        }
        let trailerStart = src.Length - 8;
        if (trailerStart <= 10)
        {
            return false;
        }
        let crc = CompressionCast.ToUInt32(src[trailerStart]) | (CompressionCast.ToUInt32(src[trailerStart + 1usize]) << 8) | (CompressionCast.ToUInt32(src[trailerStart + 2usize]) << 16) | (CompressionCast.ToUInt32(src[trailerStart + 3usize]) << 24);
        let isize = CompressionCast.ToUInt32(src[trailerStart + 4usize]) | (CompressionCast.ToUInt32(src[trailerStart + 5usize]) << 8) | (CompressionCast.ToUInt32(src[trailerStart + 6usize]) << 16) | (CompressionCast.ToUInt32(src[trailerStart + 7usize]) << 24);
        let payload = src.Slice(10usize, trailerStart - 10);
        if (! Deflate.TryDecompress (payload, dst, out var decompressed)) {
            return false;
        }
        let size32 = CompressionCast.ToUInt32(decompressed);
        if (size32 != isize)
        {
            return false;
        }
        var copy = new byte[decompressed];
        if (decompressed >0)
        {
            dst.Slice(0usize, CompressionCast.ToUSize(decompressed)).CopyTo(Span <byte >.FromArray(ref copy));
        }
        let computed = Crc32.Compute(ReadOnlySpan <byte >.FromArray(in copy));
        if (computed != crc)
        {
            return false;
        }
        bytesWritten = decompressed;
        return true;
    }
}
