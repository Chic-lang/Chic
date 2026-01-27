namespace Std.IO;
import Std.Async;
import Std.Net.Sockets;
import Std.Span;
import Std.Numeric;
/// <summary>Stream wrapper over a connected Socket.</summary>
public sealed class NetworkStream : Stream
{
    private Socket _socket;
    private bool _ownsSocket;
    /// <summary>Creates a stream over an existing connected socket.</summary>
    /// <param name="socket">Connected socket instance.</param>
    /// <param name="ownsSocket">Whether disposing the stream should close the socket.</param>
    /// <exception cref="Std.ArgumentNullException">Thrown when <paramref name="socket" /> is null.</exception>
    public init(Socket socket, bool ownsSocket = false) {
        if (socket == null)
        {
            throw new Std.ArgumentNullException("socket");
        }
        _socket = socket;
        _ownsSocket = ownsSocket;
    }
    /// <inheritdoc />
    public override bool CanRead => true;
    /// <inheritdoc />
    public override bool CanWrite => true;
    /// <inheritdoc />
    public override bool CanSeek => false;
    /// <inheritdoc />
    public override long Length => throw new Std.NotSupportedException("NetworkStream does not support Length");
    /// <inheritdoc />
    public override long Position {
        get {
            throw new Std.NotSupportedException("NetworkStream does not support Position");
        }
        set {
            throw new Std.NotSupportedException("NetworkStream does not support Position");
        }
    }
    /// <inheritdoc />
    public override int Read(Span <byte >buffer) {
        ThrowIfDisposed();
        if (buffer.Length == 0)
        {
            return 0;
        }
        let read = _socket.Receive(buffer);
        return read;
    }
    /// <inheritdoc />
    public override void Write(ReadOnlySpan <byte >buffer) {
        ThrowIfDisposed();
        if (buffer.Length == 0)
        {
            return;
        }
        var remaining = buffer.Length;
        var offset = 0usize;
        while (remaining >0)
        {
            let toSend = buffer.Slice(offset, NumericUnchecked.ToUSize(remaining));
            let written = _socket.Send(toSend);
            if (written == 0)
            {
                throw new Std.IOException("Socket send returned zero bytes");
            }
            remaining -= written;
            offset += NumericUnchecked.ToUSize(written);
        }
    }
    /// <inheritdoc />
    public override void Flush() {
        ThrowIfDisposed();
    }
    /// <inheritdoc />
    public override long Seek(long offset, SeekOrigin origin) {
        throw new Std.NotSupportedException("NetworkStream does not support Seek");
    }
    /// <inheritdoc />
    public override void SetLength(long value) {
        throw new Std.NotSupportedException("NetworkStream does not support SetLength");
    }
    /// <inheritdoc />
    protected override void Dispose(bool disposing) {
        if (IsDisposed)
        {
            return;
        }
        if (disposing && _ownsSocket)
        {
            _socket.Close();
        }
        base.Dispose(disposing);
    }
}
