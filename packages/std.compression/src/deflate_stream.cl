namespace Std.IO.Compression;
import Std.IO;
import Std.Span;
import Std.Async;
import Std;
/// <summary>Stream wrapper that transparently compresses or decompresses deflate data.</summary>
public sealed class DeflateStream : Std.IO.Stream
{
    private Stream _inner;
    private CompressionMode _mode;
    private bool _leaveOpen;
    private CompressionLevel _level;
    private MemoryStream _buffer;
    private bool _disposed;
    private bool _finalized;
    public init(Stream stream, CompressionMode mode, bool leaveOpen = false) : self(stream, mode, CompressionLevel.Optimal,
    leaveOpen) {
    }
    public init(Stream stream, CompressionLevel level, bool leaveOpen = false) : self(stream, CompressionMode.Compress, level,
    leaveOpen) {
    }
    public init(Stream stream, CompressionMode mode, CompressionLevel level, bool leaveOpen) {
        _inner = stream;
        _mode = mode;
        _level = level;
        _leaveOpen = leaveOpen;
        _disposed = false;
        _finalized = false;
        if (mode == CompressionMode.Decompress)
        {
            let compressed = ReadAllBytes(stream);
            let compressedSpan = ReadOnlySpan <byte >.FromArray(in compressed);
            let estimated = compressed.Length * 4 + 64;
            var output = new byte[estimated];
            var written = 0;
            while (true)
            {
                if (Deflate.TryDecompress (compressedSpan, Span <byte >.FromArray (ref output), out written)) {
                    break;
                }
                // grow and retry
                var larger = new byte[output.Length * 2 + 64];
                output = larger;
            }
            _buffer = new MemoryStream(output, false);
            _buffer.SetLength(CompressionCast.ToInt64(written));
        }
        else
        {
            _buffer = new MemoryStream();
        }
    }
    public override bool CanRead => _mode == CompressionMode.Decompress && !_disposed;
    public override bool CanWrite => _mode == CompressionMode.Compress && !_disposed;
    public override bool CanSeek => !_disposed && _buffer.CanSeek;
    public override long Length {
        get {
            ThrowIfDisposed();
            return _buffer.Length;
        }
    }
    public override long Position {
        get {
            ThrowIfDisposed();
            return _buffer.Position;
        }
        set {
            ThrowIfDisposed();
            _buffer.Position = value;
        }
    }
    public override int Read(Span <byte >buffer) {
        ThrowIfDisposed();
        if (_mode != CompressionMode.Decompress)
        {
            throw new Std.NotSupportedException("Stream not opened for reading");
        }
        return _buffer.Read(buffer);
    }
    public override void Write(ReadOnlySpan <byte >buffer) {
        ThrowIfDisposed();
        if (_mode != CompressionMode.Compress)
        {
            throw new Std.NotSupportedException("Stream not opened for writing");
        }
        _buffer.Write(buffer);
    }
    public override Task <int >ReadAsync(Memory <byte >buffer, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Read canceled");
        }
        let result = Read(buffer.Span);
        return TaskRuntime.FromResult(result);
    }
    public override Task WriteAsync(ReadOnlyMemory <byte >buffer, CancellationToken ct) {
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Write canceled");
        }
        Write(buffer.Span);
        return TaskRuntime.CompletedTask();
    }
    public override void Flush() {
        if (_mode == CompressionMode.Compress && !_finalized)
        {
            FinalizeAndWrite();
        }
        _inner.Flush();
    }
    public override long Seek(long offset, SeekOrigin origin) {
        return _buffer.Seek(offset, origin);
    }
    public override void SetLength(long value) {
        _buffer.SetLength(value);
    }
    protected override void Dispose(bool disposing) {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        if (disposing)
        {
            if (_mode == CompressionMode.Compress && !_finalized)
            {
                FinalizeAndWrite();
            }
            if (!_leaveOpen)
            {
                var inner = _inner;
                inner.Dispose();
            }
            var buffer = _buffer;
            buffer.Dispose();
        }
    }
    private void FinalizeAndWrite() {
        _finalized = true;
        var raw = new byte[CompressionCast.ToInt32(_buffer.Length)];
        if (raw.Length >0)
        {
            _buffer.Position = 0;
            var read = 0;
            {
                let rawSpan = Span <byte >.FromArray(ref raw);
                read = _buffer.Read(rawSpan);
            }
            _buffer.Position = 0;
            if (read != raw.Length)
            {
                throw new Std.IOException("Unable to finalize compression buffer");
            }
        }
        var compressed = new byte[raw.Length + 64];
        var written = 0;
        let rawSpan = ReadOnlySpan <byte >.FromArray(in raw);
        let compressedSpan = Span <byte >.FromArray(ref compressed);
        if (!Deflate.TryCompress (rawSpan, compressedSpan, _level, out written)) {
            throw new Std.IOException("Compression failed");
        }
        let output = compressedSpan.Slice(0usize, CompressionCast.ToUSize(written)).AsReadOnly();
        _inner.Write(output);
    }
    private static byte[] ReadAllBytes(Stream stream) {
        var buffer = new MemoryStream();
        var tmp = new byte[4096];
        while (true)
        {
            var read = 0;
            {
                let tmpSpan = Span <byte >.FromArray(ref tmp);
                read = stream.Read(tmpSpan);
            }
            if (read == 0)
            {
                break;
            }
            buffer.Write(ReadOnlySpan <byte >.FromArray(in tmp).Slice(0usize, CompressionCast.ToUSize(read)));
        }
        return buffer.ToArray();
    }
    private void ThrowIfDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("DeflateStream");
        }
    }
}
