namespace Std.Security.Cryptography;
import Std.IO;
import Std.Span;
import Std.Async;
import Std.Numeric;
/// <summary>Stream wrapper that encrypts or decrypts data on the fly.</summary>
public sealed class CryptoStream : Std.IO.Stream
{
    private readonly Stream _stream;
    private readonly ICryptoTransform _transform;
    private readonly CryptoStreamMode _mode;
    private readonly bool _leaveOpen;
    private readonly int _blockSize;
    private bool _disposed;
    private bool _finalBlockTransformed;
    private byte[] _inputBuffer;
    private int _inputBufferCount;
    private byte[] _readBuffer;
    private byte[] _outputBuffer;
    private int _outputOffset;
    private int _outputCount;
    public init(Stream stream, ICryptoTransform transform, CryptoStreamMode mode, bool leaveOpen = false) {
        if (stream == null)
        {
            throw new Std.ArgumentNullException("stream");
        }
        if (transform == null)
        {
            throw new Std.ArgumentNullException("transform");
        }
        _stream = stream;
        _transform = transform;
        _mode = mode;
        _leaveOpen = leaveOpen;
        _blockSize = transform.InputBlockSize;
        _disposed = false;
        _finalBlockTransformed = false;
        let inputSize = _blockSize * 2;
        _inputBuffer = new byte[inputSize];
        _inputBufferCount = 0;
        let readSize = _blockSize * 8;
        if (readSize <1024)
        {
            readSize = 1024;
        }
        _readBuffer = new byte[readSize];
        let outputSize = _blockSize * 4;
        if (outputSize <1024)
        {
            outputSize = 1024;
        }
        _outputBuffer = new byte[outputSize];
        _outputOffset = 0;
        _outputCount = 0;
    }
    public override bool CanRead => !_disposed && _mode == CryptoStreamMode.Read && _stream.CanRead;
    public override bool CanWrite => !_disposed && _mode == CryptoStreamMode.Write && _stream.CanWrite;
    public override bool CanSeek => false;
    public override long Length {
        get {
            throw new Std.NotSupportedException("CryptoStream does not support Length");
        }
    }
    public override long Position {
        get {
            throw new Std.NotSupportedException("CryptoStream does not support Position");
        }
        set {
            throw new Std.NotSupportedException("CryptoStream does not support Position");
        }
    }
    public override int Read(Span <byte >buffer) {
        ThrowIfDisposed();
        if (_mode != CryptoStreamMode.Read)
        {
            throw new Std.NotSupportedException("Stream is not readable");
        }
        if (buffer.Length == 0)
        {
            return 0;
        }
        if (_outputCount >0)
        {
            return CopyFromOutput(buffer);
        }
        FillOutputBuffer();
        if (_outputCount == 0)
        {
            return 0;
        }
        return CopyFromOutput(buffer);
    }
    public override void Write(ReadOnlySpan <byte >buffer) {
        ThrowIfDisposed();
        if (_mode != CryptoStreamMode.Write)
        {
            throw new Std.NotSupportedException("Stream is not writable");
        }
        if (_finalBlockTransformed)
        {
            throw new Std.InvalidOperationException("Final block already written");
        }
        if (buffer.Length == 0usize)
        {
            return;
        }
        let blockSizeU = NumericUnchecked.ToUSize(_blockSize);
        var offset = 0usize;
        if (_inputBufferCount >0)
        {
            let needed = blockSizeU - NumericUnchecked.ToUSize(_inputBufferCount);
            if (buffer.Length >= needed)
            {
                Span <byte >.FromArray(ref _inputBuffer).Slice(NumericUnchecked.ToUSize(_inputBufferCount), needed).CopyFrom(buffer.Slice(0usize,
                needed));
                WriteTransformed(ReadOnlySpan <byte >.FromArray(ref _inputBuffer).Slice(0usize, blockSizeU));
                _inputBufferCount = 0;
                offset = needed;
            }
            else
            {
                Span <byte >.FromArray(ref _inputBuffer).Slice(NumericUnchecked.ToUSize(_inputBufferCount), buffer.Length).CopyFrom(buffer);
                _inputBufferCount += NumericUnchecked.ToInt32(buffer.Length);
                return;
            }
        }
        while (buffer.Length - offset >= blockSizeU)
        {
            let chunk = buffer.Slice(offset, blockSizeU);
            WriteTransformed(chunk);
            offset += blockSizeU;
        }
        let remaining = buffer.Length - offset;
        if (remaining >0usize)
        {
            Span <byte >.FromArray(ref _inputBuffer).Slice(0usize, remaining).CopyFrom(buffer.Slice(offset, remaining));
            _inputBufferCount = NumericUnchecked.ToInt32(remaining);
        }
    }
    public override void Flush() {
        ThrowIfDisposed();
        _stream.Flush();
    }
    public void FlushFinalBlock() {
        ThrowIfDisposed();
        if (_mode != CryptoStreamMode.Write)
        {
            throw new Std.NotSupportedException("FlushFinalBlock is only valid in write mode");
        }
        if (_finalBlockTransformed)
        {
            return;
        }
        let finalInput = ReadOnlySpan <byte >.FromArray(ref _inputBuffer).Slice(0usize, NumericUnchecked.ToUSize(_inputBufferCount));
        let dest = EnsureOutputCapacity(_inputBufferCount + _blockSize);
        let written = _transform.TransformFinalBlock(finalInput, dest);
        if (written >0)
        {
            _stream.Write(dest.Slice(0usize, NumericUnchecked.ToUSize(written)));
        }
        _inputBufferCount = 0;
        _finalBlockTransformed = true;
        _stream.Flush();
    }
    public override Task <int >ReadAsync(Memory <byte >buffer, CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Read canceled");
        }
        let span = buffer.Span;
        let read = Read(span);
        return TaskRuntime.FromResult(read);
    }
    public override Task WriteAsync(ReadOnlyMemory <byte >buffer, CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Write canceled");
        }
        Write(buffer.Span);
        return TaskRuntime.CompletedTask();
    }
    public override long Seek(long offset, SeekOrigin origin) {
        throw new Std.NotSupportedException("CryptoStream does not support seeking");
    }
    public override void SetLength(long value) {
        throw new Std.NotSupportedException("CryptoStream does not support SetLength");
    }
    protected override void Dispose() {
        if (_disposed)
        {
            return;
        }
        if (_mode == CryptoStreamMode.Write && !_finalBlockTransformed)
        {
            FlushFinalBlock();
        }
        _disposed = true;
        if (!_leaveOpen)
        {
            _stream.Dispose();
        }
        base.Dispose();
    }
    private int CopyFromOutput(Span <byte >destination) {
        let available = _outputCount - _outputOffset;
        var toCopy = destination.Length;
        let availableU = NumericUnchecked.ToUSize(available);
        if (toCopy >availableU)
        {
            toCopy = availableU;
        }
        destination.Slice(0usize, NumericUnchecked.ToUSize(toCopy)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref _outputBuffer).Slice(NumericUnchecked.ToUSize(_outputOffset),
        NumericUnchecked.ToUSize(toCopy)));
        _outputOffset += NumericUnchecked.ToInt32(toCopy);
        if (_outputOffset >= _outputCount)
        {
            _outputOffset = 0;
            _outputCount = 0;
        }
        return NumericUnchecked.ToInt32(toCopy);
    }
    private void WriteTransformed(ReadOnlySpan <byte >block) {
        let dest = EnsureOutputCapacity(_blockSize + NumericUnchecked.ToInt32(block.Length));
        let written = _transform.TransformBlock(block, dest);
        if (written >0)
        {
            _stream.Write(dest.Slice(0usize, NumericUnchecked.ToUSize(written)));
        }
    }
    private void FillOutputBuffer() {
        if (_finalBlockTransformed)
        {
            _outputCount = 0;
            _outputOffset = 0;
            return;
        }
        var readSpan = Span <byte >.FromArray(ref _readBuffer);
        if (_inputBufferCount >0)
        {
            Span <byte >.FromArray(ref _inputBuffer).Slice(0usize, NumericUnchecked.ToUSize(_inputBufferCount)).CopyTo(readSpan);
        }
        let read = _stream.Read(readSpan.Slice(NumericUnchecked.ToUSize(_inputBufferCount), NumericUnchecked.ToUSize(_readBuffer.Length - _inputBufferCount)));
        let total = NumericUnchecked.ToUSize(_inputBufferCount + read);
        let blockSizeU = NumericUnchecked.ToUSize(_blockSize);
        if (read == 0)
        {
            let finalInput = readSpan.Slice(0usize, total);
            let dest = EnsureOutputCapacity(NumericUnchecked.ToInt32(total + blockSizeU));
            let written = _transform.TransformFinalBlock(finalInput, dest);
            _outputOffset = 0;
            _outputCount = written;
            _inputBufferCount = 0;
            _finalBlockTransformed = true;
            return;
        }
        let blocks = total / blockSizeU;
        if (blocks == 0usize)
        {
            Span <byte >.FromArray(ref _inputBuffer).Slice(0usize, total).CopyFrom(readSpan.Slice(0usize, total));
            _inputBufferCount = NumericUnchecked.ToInt32(total);
            _outputCount = 0;
            _outputOffset = 0;
            return;
        }
        let processBlocks = blocks - 1usize;
        if (processBlocks == 0usize)
        {
            Span <byte >.FromArray(ref _inputBuffer).Slice(0usize, total).CopyFrom(readSpan.Slice(0usize, total));
            _inputBufferCount = NumericUnchecked.ToInt32(total);
            _outputCount = 0;
            _outputOffset = 0;
            return;
        }
        let processBytes = processBlocks * blockSizeU;
        let dest = EnsureOutputCapacity(NumericUnchecked.ToInt32(processBytes + blockSizeU));
        let written = _transform.TransformBlock(readSpan.Slice(0usize, processBytes), dest);
        _outputOffset = 0;
        _outputCount = written;
        let remaining = total - processBytes;
        if (remaining >0usize)
        {
            Span <byte >.FromArray(ref _inputBuffer).Slice(0usize, remaining).CopyFrom(readSpan.Slice(processBytes, remaining));
            _inputBufferCount = NumericUnchecked.ToInt32(remaining);
        }
        else
        {
            _inputBufferCount = 0;
        }
    }
    private Span <byte >EnsureOutputCapacity(int needed) {
        if (_outputBuffer.Length <needed)
        {
            var newSize = _outputBuffer.Length * 2;
            if (newSize <needed)
            {
                newSize = needed;
            }
            _outputBuffer = new byte[newSize];
        }
        return Span <byte >.FromArray(ref _outputBuffer);
    }
}
