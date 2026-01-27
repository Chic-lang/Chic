namespace Std.Net.Http;
import Std.Span;
import Std.Numeric;
import Std.Platform;
import Foundation.Collections;
import Std.Net.Sockets;
/// <summary>
/// HttpContent that keeps the underlying socket open until the body is read, enabling
/// ResponseHeadersRead semantics.
/// </summary>
internal sealed class SocketHttpContent : HttpContent
{
    private Socket _socket;
    private VecPtr _buffer;
    private bool _bufferOwned;
    private bool _completed;
    private string _poolKey;
    private HttpClientHandler _handler;
    private bool _returned;
    private byte[] ?_cached;
    private usize _expectedLength;
    private Std.Async.CancellationToken _token;
    private Std.Async.CancellationToken _globalToken;
    private ulong _startNs;
    private Std.Datetime.Duration _timeout;
    private long _maxBuffer;
    private bool _allowReuse;
    private bool _remoteClosed;
    private bool _hasKnownLength;
    public init(Socket socket, VecPtr buffer, usize expectedLength, bool hasKnownLength, Std.Async.CancellationToken token,
    Std.Async.CancellationToken globalToken, ulong startNs, Std.Datetime.Duration timeout, long maxBuffer, string poolKey,
    HttpClientHandler handler, bool allowReuse) : base() {
        _socket = socket;
        _buffer = buffer;
        _bufferOwned = true;
        _completed = false;
        _cached = null;
        _expectedLength = expectedLength;
        _token = token;
        _globalToken = globalToken;
        _startNs = startNs;
        _timeout = timeout;
        _maxBuffer = maxBuffer;
        _poolKey = poolKey;
        _handler = handler;
        _returned = false;
        _allowReuse = allowReuse;
        _remoteClosed = false;
        _hasKnownLength = hasKnownLength;
        if (expectedLength >0)
        {
            Headers.Set("Content-Length", NumericUnchecked.ToInt32(expectedLength).ToString());
        }
    }
    public override void Dispose() {
        base.Dispose();
        if (_socket != null && _socket.IsValid && !_returned)
        {
            _socket.Close();
        }
        if (_bufferOwned)
        {
            FVecIntrinsics.chic_rt_vec_drop(ref _buffer);
            _bufferOwned = false;
        }
    }
    internal override byte[] GetBytes() {
        if (_completed && _cached != null)
        {
            return _cached;
        }
        var recvTemp = Span <byte >.StackAlloc(512);
        var currentLen = FVec.Len(in _buffer);
        EnforceBufferLimit(currentLen);
        while ( (!_hasKnownLength && !_remoteClosed) || (_hasKnownLength && currentLen <_expectedLength))
        {
            let read = _socket.Receive(recvTemp);
            let readCount = NumericUnchecked.ToUSize(read);
            CheckCancellation();
            if (readCount == 0usize)
            {
                _remoteClosed = true;
                break;
            }
            AppendSpan(ref _buffer, recvTemp.Slice(0, readCount));
            currentLen = FVec.Len(in _buffer);
            EnforceBufferLimit(currentLen);
        }
        if (_hasKnownLength && _expectedLength >0usize && FVec.Len (in _buffer) <_expectedLength) {
            if (_bufferOwned)
            {
                FVecIntrinsics.chic_rt_vec_drop(ref _buffer);
                _bufferOwned = false;
            }
            if (_socket.IsValid)
            {
                _socket.Close();
                _returned = true;
            }
            throw new HttpRequestException("Incomplete HTTP response body");
        }
        var array = new byte[NumericUnchecked.ToInt32(FVec.Len(in _buffer))];
        var span = Span <byte >.FromArray(ref array);
        let src = FVec.AsReadOnlySpan <byte >(in _buffer);
        span.CopyFrom(src);
        if (_bufferOwned)
        {
            FVecIntrinsics.chic_rt_vec_drop(ref _buffer);
            _bufferOwned = false;
        }
        ReturnToPool();
        _cached = array;
        _completed = true;
        return array;
    }
    public override byte[] ReadAsByteArray() {
        return GetBytes();
    }
    private void AppendSpan(ref VecPtr vec, ReadOnlySpan <byte >span) {
        var idx = 0usize;
        while (idx <span.Length)
        {
            FVec.Push <byte >(ref vec, span[idx]);
            idx += 1;
        }
    }
    private void EnforceBufferLimit(usize length) {
        if (_maxBuffer <0)
        {
            return;
        }
        let len64 = NumericUnchecked.ToInt64(length);
        if (len64 >_maxBuffer)
        {
            throw new HttpRequestException("Response content exceeded buffer limit");
        }
    }
    private void CheckCancellation() {
        if (_token.IsCancellationRequested () || _globalToken.IsCancellationRequested ())
        {
            throw new Std.TaskCanceledException("Request canceled");
        }
        let timeoutTicks = _timeout.Ticks;
        if (timeoutTicks <= 0)
        {
            return;
        }
        let timeoutNs = NumericUnchecked.ToUInt64(timeoutTicks) * 100UL;
        let elapsed = Time.MonotonicNanoseconds() - _startNs;
        if (elapsed >timeoutNs)
        {
            throw new Std.TaskCanceledException("Request timed out");
        }
    }
    private void ReturnToPool() {
        if (_socket == null || !_socket.IsValid || _returned)
        {
            return;
        }
        if (!_allowReuse || _remoteClosed || _globalToken.IsCancellationRequested () || _token.IsCancellationRequested ())
        {
            _socket.Close();
            _returned = true;
            return;
        }
        _handler.ReturnSocket(_poolKey, _socket);
        _returned = true;
    }
}
