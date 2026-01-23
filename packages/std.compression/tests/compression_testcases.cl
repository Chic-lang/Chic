namespace Std.IO.Compression;
import Std.IO;
import Std.Numeric;
import Std.Platform;
import Std.Span;
import Std.Testing;
testcase Given_crc32_known_vector_When_executed_Then_crc32_known_vector()
{
    let data = ReadOnlySpan.FromString("123456789");
    let crc = Crc32.Compute(data);
    Assert.That(crc).IsEqualTo(0xCBF43926u);
}
testcase Given_deflate_roundtrip_fixed_compress_ok_When_executed_Then_deflate_roundtrip_fixed_compress_ok()
{
    let payload = ReadOnlySpan.FromString("hello");
    var compressed = new byte[64];
    let ok = Deflate.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    Assert.That(ok).IsTrue();
}
testcase Given_deflate_roundtrip_fixed_decompress_ok_When_executed_Then_deflate_roundtrip_fixed_decompress_ok()
{
    let payload = ReadOnlySpan.FromString("hello");
    var compressed = new byte[64];
    let _ = Deflate.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    let encoded = ReadOnlySpan <byte >.FromArray(in compressed).Slice(0usize, CompressionCast.ToUSize(written));
    var decompressed = new byte[16];
    let ok2 = Deflate.TryDecompress(encoded, Span <byte >.FromArray(ref decompressed), out var decompressedWritten);
    Assert.That(ok2).IsTrue();
}
testcase Given_deflate_roundtrip_fixed_payload_matches_When_executed_Then_deflate_roundtrip_fixed_payload_matches()
{
    let payload = ReadOnlySpan.FromString("hello");
    var compressed = new byte[64];
    let _ = Deflate.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    let encoded = ReadOnlySpan <byte >.FromArray(in compressed).Slice(0usize, CompressionCast.ToUSize(written));
    var decompressed = new byte[16];
    let _ = Deflate.TryDecompress(encoded, Span <byte >.FromArray(ref decompressed), out var decompressedWritten);
    let decoded = ReadOnlySpan <byte >.FromArray(in decompressed).Slice(0usize, CompressionCast.ToUSize(decompressedWritten));
    Assert.That(decoded).IsEqualTo(payload);
}
testcase Given_deflate_roundtrip_stored_compress_ok_When_executed_Then_deflate_roundtrip_stored_compress_ok()
{
    let payload = ReadOnlySpan.FromString("stored-data");
    var compressed = new byte[64];
    let ok = Deflate.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.NoCompression, out var written);
    Assert.That(ok).IsTrue();
}
testcase Given_deflate_roundtrip_stored_decompress_ok_When_executed_Then_deflate_roundtrip_stored_decompress_ok()
{
    let payload = ReadOnlySpan.FromString("stored-data");
    var compressed = new byte[64];
    let _ = Deflate.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.NoCompression, out var written);
    let encoded = ReadOnlySpan <byte >.FromArray(in compressed).Slice(0usize, CompressionCast.ToUSize(written));
    var decompressed = new byte[32];
    let ok2 = Deflate.TryDecompress(encoded, Span <byte >.FromArray(ref decompressed), out var decompressedWritten);
    Assert.That(ok2).IsTrue();
}
testcase Given_deflate_roundtrip_stored_payload_matches_When_executed_Then_deflate_roundtrip_stored_payload_matches()
{
    let payload = ReadOnlySpan.FromString("stored-data");
    var compressed = new byte[64];
    let _ = Deflate.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.NoCompression, out var written);
    let encoded = ReadOnlySpan <byte >.FromArray(in compressed).Slice(0usize, CompressionCast.ToUSize(written));
    var decompressed = new byte[32];
    let _ = Deflate.TryDecompress(encoded, Span <byte >.FromArray(ref decompressed), out var decompressedWritten);
    let decoded = ReadOnlySpan <byte >.FromArray(in decompressed).Slice(0usize, CompressionCast.ToUSize(decompressedWritten));
    Assert.That(decoded).IsEqualTo(payload);
}
testcase Given_deflate_invalid_data_fails_ok_false_When_executed_Then_deflate_invalid_data_fails_ok_false()
{
    var invalid = new byte[1];
    invalid[0] = 0u8;
    var output = new byte[8];
    let ok = Deflate.TryDecompress(ReadOnlySpan <byte >.FromArray(in invalid), Span <byte >.FromArray(ref output), out var written);
    Assert.That(ok).IsFalse();
}
testcase Given_deflate_invalid_data_fails_written_zero_When_executed_Then_deflate_invalid_data_fails_written_zero()
{
    var invalid = new byte[1];
    invalid[0] = 0u8;
    var output = new byte[8];
    let _ = Deflate.TryDecompress(ReadOnlySpan <byte >.FromArray(in invalid), Span <byte >.FromArray(ref output), out var written);
    Assert.That(written).IsEqualTo(0);
}
testcase Given_gzip_roundtrip_compress_ok_When_executed_Then_gzip_roundtrip_compress_ok()
{
    let payload = ReadOnlySpan.FromString("gzip");
    var compressed = new byte[64];
    let ok = GZip.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    Assert.That(ok).IsTrue();
}
testcase Given_gzip_roundtrip_decompress_ok_When_executed_Then_gzip_roundtrip_decompress_ok()
{
    let payload = ReadOnlySpan.FromString("gzip");
    var compressed = new byte[64];
    let _ = GZip.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    let encoded = ReadOnlySpan <byte >.FromArray(in compressed).Slice(0usize, CompressionCast.ToUSize(written));
    var decompressed = new byte[16];
    let ok2 = GZip.TryDecompress(encoded, Span <byte >.FromArray(ref decompressed), out var decompressedWritten);
    Assert.That(ok2).IsTrue();
}
testcase Given_gzip_roundtrip_payload_matches_When_executed_Then_gzip_roundtrip_payload_matches()
{
    let payload = ReadOnlySpan.FromString("gzip");
    var compressed = new byte[64];
    let _ = GZip.TryCompress(payload, Span <byte >.FromArray(ref compressed), CompressionLevel.Optimal, out var written);
    let encoded = ReadOnlySpan <byte >.FromArray(in compressed).Slice(0usize, CompressionCast.ToUSize(written));
    var decompressed = new byte[16];
    let _ = GZip.TryDecompress(encoded, Span <byte >.FromArray(ref decompressed), out var decompressedWritten);
    let decoded = ReadOnlySpan <byte >.FromArray(in decompressed).Slice(0usize, CompressionCast.ToUSize(decompressedWritten));
    Assert.That(decoded).IsEqualTo(payload);
}
testcase Given_gzip_invalid_header_fails_ok_false_When_executed_Then_gzip_invalid_header_fails_ok_false()
{
    var invalid = new byte[10];
    invalid[0] = 0u8;
    var output = new byte[8];
    let ok = GZip.TryDecompress(ReadOnlySpan <byte >.FromArray(in invalid), Span <byte >.FromArray(ref output), out var written);
    Assert.That(ok).IsFalse();
}
testcase Given_gzip_invalid_header_fails_written_zero_When_executed_Then_gzip_invalid_header_fails_written_zero()
{
    var invalid = new byte[10];
    invalid[0] = 0u8;
    var output = new byte[8];
    let _ = GZip.TryDecompress(ReadOnlySpan <byte >.FromArray(in invalid), Span <byte >.FromArray(ref output), out var written);
    Assert.That(written).IsEqualTo(0);
}
testcase Given_deflate_stream_roundtrip_reads_expected_length_When_executed_Then_deflate_stream_roundtrip_reads_expected_length()
{
    var inner = new MemoryStream();
    var writer = new DeflateStream(inner, CompressionMode.Compress, true);
    let payload = ReadOnlySpan.FromString("stream-data");
    writer.Write(payload);
    writer.Flush();
    writer.Dispose();
    inner.Position = 0;
    var reader = new DeflateStream(inner, CompressionMode.Decompress, true);
    var buffer = Span <byte >.StackAlloc(payload.Length);
    let read = reader.Read(buffer);
    let expected = NumericUnchecked.ToInt32(payload.Length);
    Assert.That(read).IsEqualTo(expected);
    reader.Dispose();
}
testcase Given_deflate_stream_roundtrip_reads_payload_When_executed_Then_deflate_stream_roundtrip_reads_payload()
{
    var inner = new MemoryStream();
    var writer = new DeflateStream(inner, CompressionMode.Compress, true);
    let payload = ReadOnlySpan.FromString("stream-data");
    writer.Write(payload);
    writer.Flush();
    writer.Dispose();
    inner.Position = 0;
    var reader = new DeflateStream(inner, CompressionMode.Decompress, true);
    var buffer = Span <byte >.StackAlloc(payload.Length);
    let _ = reader.Read(buffer);
    Assert.That(buffer.AsReadOnly()).IsEqualTo(payload);
    reader.Dispose();
}
testcase Given_gzip_stream_roundtrip_reads_expected_length_When_executed_Then_gzip_stream_roundtrip_reads_expected_length()
{
    var inner = new MemoryStream();
    var writer = new GZipStream(inner, CompressionMode.Compress, true);
    let payload = ReadOnlySpan.FromString("gzip-stream");
    writer.Write(payload);
    writer.Flush();
    writer.Dispose();
    inner.Position = 0;
    var reader = new GZipStream(inner, CompressionMode.Decompress, true);
    var buffer = Span <byte >.StackAlloc(payload.Length);
    let read = reader.Read(buffer);
    let expected = NumericUnchecked.ToInt32(payload.Length);
    Assert.That(read).IsEqualTo(expected);
    reader.Dispose();
}
testcase Given_gzip_stream_roundtrip_reads_payload_When_executed_Then_gzip_stream_roundtrip_reads_payload()
{
    var inner = new MemoryStream();
    var writer = new GZipStream(inner, CompressionMode.Compress, true);
    let payload = ReadOnlySpan.FromString("gzip-stream");
    writer.Write(payload);
    writer.Flush();
    writer.Dispose();
    inner.Position = 0;
    var reader = new GZipStream(inner, CompressionMode.Decompress, true);
    var buffer = Span <byte >.StackAlloc(payload.Length);
    let _ = reader.Read(buffer);
    Assert.That(buffer.AsReadOnly()).IsEqualTo(payload);
    reader.Dispose();
}
testcase Given_compression_hardware_default_is_scalar_When_executed_Then_compression_hardware_default_is_scalar()
{
    EnvironmentVariables.Remove("STD_COMPRESSION_FORCE_SCALAR");
    EnvironmentVariables.Remove("STD_COMPRESSION_FORCE_ACCEL");
    let useAccel = CompressionHardware.UseAcceleratedCrc32;
    Assert.That(useAccel).IsFalse();
}
