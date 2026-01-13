# Std.IO.Compression

Deterministic gzip/deflate codecs and stream wrappers built in Chic. The APIs plug directly into the Stream stack and HttpClient.

## Public surface

- `enum CompressionMode { Compress, Decompress }`
- `enum CompressionLevel { Optimal, Fastest, NoCompression }`
- `static class Crc32`
  - `uint Compute(ReadOnlySpan<byte> data)`
  - `void Append(ref uint state, ReadOnlySpan<byte> data)`
- `static class Deflate`
  - `bool TryCompress(ReadOnlySpan<byte> src, Span<byte> dst, CompressionLevel level, out int bytesWritten)`
  - `bool TryDecompress(ReadOnlySpan<byte> src, Span<byte> dst, out int bytesWritten)`
- `static class GZip`
  - `bool TryCompress(ReadOnlySpan<byte> src, Span<byte> dst, CompressionLevel level, out int bytesWritten)`
  - `bool TryDecompress(ReadOnlySpan<byte> src, Span<byte> dst, out int bytesWritten)`
- `sealed class DeflateStream : Stream`
- `sealed class GZipStream : Stream`

## Usage

### Span-first codec

```cl
var input = new byte[]{1u8,2u8,3u8};
var buffer = new byte[128];
int written;
if (!Deflate.TryCompress(ReadOnlySpan<byte>.FromArray(ref input), Span<byte>.FromArray(ref buffer), CompressionLevel.Optimal, out written))
{
    throw new Std.IOException("compress failed");
}

var output = new byte[16];
int plain;
if (!Deflate.TryDecompress(ReadOnlySpan<byte>.FromArray(ref buffer).Slice(0usize, written), Span<byte>.FromArray(ref output), out plain))
{
    throw new Std.IOException("decompress failed");
}
```

### Stream wrappers

```cl
var target = new Std.IO.MemoryStream();
{
    var gzip = new Std.IO.Compression.GZipStream(target, CompressionMode.Compress, true);
    gzip.Write(ReadOnlySpan<byte>.FromArray(ref input));
    gzip.Dispose(); // flushes compressed payload into `target`
}
var compressed = target.ToArray();
```

### HttpClient

`HttpClient` transparently decompresses `gzip` or `deflate` response bodies when `Content-Encoding` is present. No extra flags are required; the handler removes the encoding header and rewrites the content length after decoding.

## Determinism and acceleration

The codec paths are deterministic for a given input and compression level. CRC32 uses a portable lookup table today; ISA-specific dispatch hooks are in place for future acceleration and can be extended without changing correctness. The compressor currently emits fixed-Huffman deflate blocks for deterministic output; `NoCompression` emits stored blocks.
