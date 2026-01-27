namespace Std.IO;
import Std.Async;
import Std.Core;
import Std.Numeric;
import Std.Span;
/// <summary>Abstract base class for stream-based IO with span-first APIs.</summary>
public abstract class Stream
{
    private const int DefaultCopyBufferSize = 81920;
    private bool _disposed;
    /// <summary>Gets a value indicating whether the stream has been disposed.</summary>
    protected bool IsDisposed => _disposed;
    /// <summary>Throws if the stream has already been disposed.</summary>
    /// <exception cref="Std.ObjectDisposedException">Thrown when the stream is disposed.</exception>
    protected void ThrowIfDisposed() {
        if (_disposed)
        {
            throw new Std.ObjectDisposedException("Stream has been disposed");
        }
    }
    /// <summary>Gets a value indicating whether the current stream supports reading.</summary>
    public abstract bool CanRead {
        get;
    }
    /// <summary>Gets a value indicating whether the current stream supports writing.</summary>
    public abstract bool CanWrite {
        get;
    }
    /// <summary>Gets a value indicating whether the current stream supports seeking.</summary>
    public abstract bool CanSeek {
        get;
    }
    /// <summary>Gets the length of the stream in bytes.</summary>
    /// <returns>The total length of the stream.</returns>
    /// <exception cref="Std.NotSupportedException">Thrown when the stream does not support length.</exception>
    public virtual long Length {
        get {
            this.ThrowIfDisposed();
            throw new Std.NotSupportedException("Length not supported");
        }
    }
    /// <summary>Gets or sets the current position within the stream.</summary>
    /// <exception cref="Std.NotSupportedException">Thrown when seeking is not supported.</exception>
    /// <exception cref="Std.ArgumentOutOfRangeException">Thrown when the position is negative.</exception>
    public virtual long Position {
        get {
            this.ThrowIfDisposed();
            throw new Std.NotSupportedException("Position not supported");
        }
        set {
            this.ThrowIfDisposed();
            Seek(value, SeekOrigin.Begin);
        }
    }
    /// <summary>Reads a block of bytes from the stream into the provided buffer.</summary>
    /// <param name="buffer">Destination buffer to fill.</param>
    /// <returns>The number of bytes read; 0 if the end of the stream is reached.</returns>
    public virtual int Read(Span <byte >buffer) {
        this.ThrowIfDisposed();
        throw new Std.NotSupportedException("Read not supported");
    }
    /// <summary>Writes a block of bytes to the stream from the provided buffer.</summary>
    /// <param name="buffer">Source buffer to write.</param>
    public abstract void Write(ReadOnlySpan <byte >buffer);
    /// <summary>Flushes any buffered data to the underlying storage or transport.</summary>
    public abstract void Flush();
    /// <summary>Reads a block of bytes asynchronously.</summary>
    /// <param name="buffer">Destination buffer to fill.</param>
    /// <returns>A task that resolves to the number of bytes read.</returns>
    public Task <int >ReadAsync(Memory <byte >buffer) => ReadAsync(buffer, CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Reads a block of bytes asynchronously.</summary>
    /// <param name="buffer">Destination buffer to fill.</param>
    /// <param name="ct">Cancellation token to abort the operation.</param>
    /// <returns>A task that resolves to the number of bytes read.</returns>
    /// <exception cref="Std.TaskCanceledException">Thrown when cancellation is requested.</exception>
    public virtual Task <int >ReadAsync(Memory <byte >buffer, CancellationToken ct) {
        this.ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Read canceled");
        }
        let read = Read(buffer.Span);
        return TaskRuntime.FromResult(read);
    }
    /// <summary>Writes a block of bytes asynchronously.</summary>
    /// <param name="buffer">Source buffer to write.</param>
    /// <returns>A task that completes when the write finishes.</returns>
    public Task WriteAsync(ReadOnlyMemory <byte >buffer) => WriteAsync(buffer, CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Writes a block of bytes asynchronously.</summary>
    /// <param name="buffer">Source buffer to write.</param>
    /// <param name="ct">Cancellation token to abort the operation.</param>
    /// <returns>A task that completes when the write finishes.</returns>
    /// <exception cref="Std.TaskCanceledException">Thrown when cancellation is requested.</exception>
    public virtual Task WriteAsync(ReadOnlyMemory <byte >buffer, CancellationToken ct) {
        this.ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Write canceled");
        }
        Write(buffer.Span);
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Flushes buffered data asynchronously.</summary>
    /// <returns>A completed task when flushing is done.</returns>
    public Task FlushAsync() => FlushAsync(CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Flushes buffered data asynchronously.</summary>
    /// <param name="ct">Cancellation token to abort the operation.</param>
    /// <returns>A completed task when flushing is done.</returns>
    /// <exception cref="Std.TaskCanceledException">Thrown when cancellation is requested.</exception>
    public virtual Task FlushAsync(CancellationToken ct) {
        ThrowIfDisposed();
        if (ct.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Flush canceled");
        }
        Flush();
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Sets the stream position to the specified offset.</summary>
    /// <param name="offset">Byte offset relative to <paramref name="origin"/>.</param>
    /// <param name="origin">Reference point used to obtain the new position.</param>
    /// <returns>The new position within the stream.</returns>
    /// <exception cref="Std.NotSupportedException">Thrown when seeking is not supported.</exception>
    public virtual long Seek(long offset, SeekOrigin origin) {
        ThrowIfDisposed();
        throw new Std.NotSupportedException("Seek not supported");
    }
    /// <summary>Changes the length of the stream.</summary>
    /// <param name="value">New length of the stream.</param>
    /// <exception cref="Std.NotSupportedException">Thrown when the stream does not support resizing.</exception>
    public virtual void SetLength(long value) {
        ThrowIfDisposed();
        throw new Std.NotSupportedException("SetLength not supported");
    }
    /// <summary>Reads a single byte from the stream.</summary>
    /// <returns>The byte cast to int, or -1 if at the end of the stream.</returns>
    public virtual int ReadByte() {
        var buffer = Span <byte >.StackAlloc(1);
        let read = Read(buffer);
        if (read == 0)
        {
            return - 1;
        }
        return buffer[0];
    }
    /// <summary>Writes a single byte to the stream.</summary>
    /// <param name="value">The byte to write.</param>
    public virtual void WriteByte(byte value) {
        var buffer = Span <byte >.StackAlloc(1);
        buffer[0] = value;
        Write(buffer.AsReadOnly());
    }
    /// <summary>Copies the contents of the stream to another stream.</summary>
    /// <param name="destination">The destination stream.</param>
    /// <exception cref="Std.ArgumentNullException">Thrown when destination is null.</exception>
    /// <exception cref="Std.ArgumentOutOfRangeException">Thrown when buffer size is invalid.</exception>
    public void CopyTo(Stream destination) => CopyTo(destination, DefaultCopyBufferSize);
    /// <summary>Copies the contents of the stream to another stream.</summary>
    /// <param name="destination">The destination stream.</param>
    /// <param name="bufferSize">Size of the intermediate buffer.</param>
    /// <exception cref="Std.ArgumentNullException">Thrown when destination is null.</exception>
    /// <exception cref="Std.ArgumentOutOfRangeException">Thrown when buffer size is invalid.</exception>
    public virtual void CopyTo(Stream destination, int bufferSize = DefaultCopyBufferSize) {
        ThrowIfDisposed();
        if (destination == null)
        {
            throw new Std.ArgumentNullException("destination");
        }
        if (bufferSize <= 0)
        {
            throw new Std.ArgumentOutOfRangeException("bufferSize");
        }
        var buffer = new byte[bufferSize];
        while (true)
        {
            var read = 0;
            {
                let span = Span <byte >.FromArray(ref buffer);
                read = Read(span);
            }
            if (read == 0)
            {
                break;
            }
            let slice = ReadOnlySpan <byte >.FromArray(in buffer).Slice(0usize, NumericUnchecked.ToUSize(read));
            destination.Write(slice);
        }
    }
    /// <summary>Asynchronously copies the contents of the stream to another stream using the default buffer size.</summary>
    /// <param name="destination">The destination stream.</param>
    /// <returns>A task that completes when copying finishes.</returns>
    public Task CopyToAsync(Stream destination) => CopyToAsync(destination, DefaultCopyBufferSize, CoreIntrinsics.DefaultValue <CancellationToken >());
    /// <summary>Asynchronously copies the contents of the stream to another stream.</summary>
    /// <param name="destination">The destination stream.</param>
    /// <param name="ct">Cancellation token to abort the operation.</param>
    /// <returns>A task that completes when copying finishes.</returns>
    public Task CopyToAsync(Stream destination, CancellationToken ct) => CopyToAsync(destination, DefaultCopyBufferSize,
    ct);
    /// <summary>Asynchronously copies the contents of the stream to another stream.</summary>
    /// <param name="destination">The destination stream.</param>
    /// <param name="bufferSize">Size of the intermediate buffer.</param>
    /// <param name="ct">Cancellation token to abort the operation.</param>
    /// <returns>A task that completes when copying finishes.</returns>
    /// <exception cref="Std.ArgumentNullException">Thrown when destination is null.</exception>
    /// <exception cref="Std.ArgumentOutOfRangeException">Thrown when buffer size is invalid.</exception>
    /// <exception cref="Std.TaskCanceledException">Thrown when cancellation is requested.</exception>
    public virtual Task CopyToAsync(Stream destination, int bufferSize, CancellationToken ct) {
        ThrowIfDisposed();
        if (destination == null)
        {
            throw new Std.ArgumentNullException("destination");
        }
        if (bufferSize <= 0)
        {
            throw new Std.ArgumentOutOfRangeException("bufferSize");
        }
        var buffer = new byte[bufferSize];
        while (true)
        {
            if (ct.IsCancellationRequested ())
            {
                throw new Std.TaskCanceledException("CopyToAsync canceled");
            }
            var mem = new Memory <byte >(buffer, 0, buffer.Length);
            let readTask = ReadAsync(mem, ct);
            let read = TaskRuntime.GetResult <int >(readTask);
            if (read == 0)
            {
                break;
            }
            let slice = new ReadOnlyMemory <byte >(buffer, 0, read);
            let writeTask = destination.WriteAsync(slice, ct);
        }
        return TaskRuntime.CompletedTask();
    }
    /// <summary>Reads the entire stream into a new byte array.</summary>
    /// <returns>A byte array containing the stream contents.</returns>
    public virtual byte[] ReadAllBytes() {
        ThrowIfDisposed();
        var ms = new MemoryStream();
        var result = new byte[0];
        try {
            CopyTo(ms, 4096);
            result = ms.ToArray();
        }
        finally {
            ms.Dispose();
        }
        return result;
    }
    /// <summary>Closes the stream and releases associated resources.</summary>
    public void Close() => Dispose();
    /// <summary>Releases all resources used by the stream.</summary>
    public virtual void Dispose() {
        Dispose(true);
    }
    /// <summary>Releases managed/unmanaged resources.</summary>
    /// <param name="disposing">True when called explicitly, false when called from the finalizer.</param>
    protected virtual void Dispose(bool disposing) {
        _disposed = true;
    }
    /// <summary>Runtime hook for deterministic destruction.</summary>
    public void dispose(ref this) {
        Dispose();
    }
}
