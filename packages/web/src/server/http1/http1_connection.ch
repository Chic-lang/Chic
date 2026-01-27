namespace Chic.Web;
import Std.Async;
import Std.Core;
import Std.IO;
import Std.Net.Http;
import Std.Numeric;
import Std.Span;
import Std.Strings;
/// <summary>Processes HTTP/1.1 requests on a single TCP connection.</summary>
public sealed class Http1Connection
{
    private Stream _stream;
    private RequestDelegate _app;
    private byte[] _buffer;
    private int _start;
    private int _end;
    public init(Stream stream, RequestDelegate app) {
        _stream = stream;
        _app = app;
        _buffer = new byte[8192];
        _start = 0;
        _end = 0;
    }
    public void Process(CancellationToken ct) {
        while (!ct.IsCancellationRequested ())
        {
            if (!TryReadRequest (out var request, out var shouldClose)) {
                break;
            }
            var response = new HttpResponse();
            var context = new HttpContext(request, response, ct);
            try {
                let task = _app.Invoke(context);
                Std.Async.Runtime.BlockOn(task);
            }
            catch(Std.Exception ex) {
                response.StatusCode = 500;
                response.Headers.Set("Content-Type", "text/plain");
                response.WriteStringAsync("Unhandled error: " + ex.Message);
            }
            if (ct.IsCancellationRequested ())
            {
                break;
            }
            try {
                SendResponse(response, !shouldClose);
            }
            catch(Std.Exception) {
                break;
            }
            if (shouldClose || ct.IsCancellationRequested ())
            {
                break;
            }
        }
    }
    private bool TryReadRequest(out HttpRequest request, out bool shouldClose) {
        request = CoreIntrinsics.DefaultValue <HttpRequest >();
        shouldClose = false;
        if (!TryReadLine (out var requestLine)) {
            return false;
        }
        if (!ParseRequestLine (requestLine, out var method, out var target, out var version)) {
            SendSimpleResponse(400, "Bad Request");
            return false;
        }
        if (!ReadHeaders (out var headers, out var connectionClose, out var keepAlive, out var contentLength, out var chunked)) {
            SendSimpleResponse(400, "Bad Request");
            return false;
        }
        shouldClose = connectionClose;
        if (version == "HTTP/1.0" && !keepAlive)
        {
            shouldClose = true;
        }
        var body = ReadBody(contentLength, chunked);
        body.Position = 0;
        var path = target;
        var queryString = "";
        let queryIdx = target.IndexOf("?");
        if (queryIdx >= 0)
        {
            path = target.Substring(0, queryIdx);
            queryString = target.Substring(queryIdx);
        }
        if (path.Length == 0)
        {
            path = "/";
        }
        request = new HttpRequest(method, path, queryString, headers, body);
        return true;
    }
    private MemoryStream ReadBody(long contentLength, bool chunked) {
        if (chunked)
        {
            return ReadChunkedBody();
        }
        if (contentLength <= 0)
        {
            return new MemoryStream();
        }
        return ReadFixedLengthBody(contentLength);
    }
    private MemoryStream ReadFixedLengthBody(long length) {
        var body = new MemoryStream();
        var remaining = length;
        var buffer = new byte[4096];
        while (remaining >0)
        {
            let toRead = remaining >buffer.Length ?buffer.Length : NumericUnchecked.ToInt32(remaining);
            var span = Span <byte >.FromArray(ref buffer).Slice(0usize, NumericUnchecked.ToUSize(toRead));
            let read = ReadInto(span);
            if (read == 0)
            {
                throw new Std.IOException("unexpected end of stream while reading body");
            }
            let slice = ReadOnlySpan <byte >.FromArray(ref buffer).Slice(0usize, NumericUnchecked.ToUSize(read));
            body.Write(slice);
            remaining -= read;
        }
        body.Position = 0;
        return body;
    }
    private MemoryStream ReadChunkedBody() {
        var body = new MemoryStream();
        while (true)
        {
            if (!TryReadLine (out var line)) {
                throw new Std.IOException("incomplete chunk header");
            }
            let size = ParseHex(line);
            if (size == 0)
            {
                // Trailing headers (ignored).
                while (true)
                {
                    if (!TryReadLine (out var trailer)) {
                        break;
                    }
                    if (trailer.Length == 0usize)
                    {
                        break;
                    }
                }
                break;
            }
            var remaining = size;
            var buffer = new byte[4096];
            while (remaining >0usize)
            {
                let toRead = remaining >NumericUnchecked.ToUSize(buffer.Length) ?buffer.Length : NumericUnchecked.ToInt32(remaining);
                var span = Span <byte >.FromArray(ref buffer).Slice(0usize, NumericUnchecked.ToUSize(toRead));
                let read = ReadInto(span);
                if (read == 0)
                {
                    throw new Std.IOException("unexpected end of stream while reading chunk");
                }
                let slice = ReadOnlySpan <byte >.FromArray(ref buffer).Slice(0usize, NumericUnchecked.ToUSize(read));
                body.Write(slice);
                remaining -= NumericUnchecked.ToUSize(read);
            }
            // Consume CRLF after chunk data.
            ConsumeCrlf();
        }
        body.Position = 0;
        return body;
    }
    private void SendResponse(HttpResponse response, bool keepAlive) {
        response.MarkStarted();
        let body = response.BodyStream.GetSpan();
        if (!response.HasContentLength)
        {
            response.ContentLength = NumericUnchecked.ToInt64(body.Length);
        }
        response.Headers.Set("Content-Length", response.ContentLength.ToString());
        response.Headers.Set("Connection", keepAlive ?"keep-alive" : "close");
        WriteStatusLine(response.StatusCode);
        WriteHeaders(response.Headers);
        WriteAscii("\r\n");
        if (body.Length >0usize)
        {
            _stream.Write(body);
        }
        _stream.Flush();
    }
    private void SendSimpleResponse(int statusCode, string message) {
        var response = new HttpResponse();
        response.StatusCode = statusCode;
        response.Headers.Set("Content-Type", "text/plain");
        response.WriteStringAsync(message);
        SendResponse(response, false);
    }
    private bool ParseRequestLine(ReadOnlySpan <byte >line, out string method, out string target, out string version) {
        method = "";
        target = "";
        version = "";
        let firstSpace = FindByte(line, NumericUnchecked.ToByte(' '));
        if (firstSpace <0)
        {
            return false;
        }
        let secondSpace = FindByteFrom(line, NumericUnchecked.ToByte(' '), NumericUnchecked.ToUSize(firstSpace + 1));
        if (secondSpace <0)
        {
            return false;
        }
        method = Utf8String.FromSpan(line.Slice(0usize, NumericUnchecked.ToUSize(firstSpace)));
        let targetLen = secondSpace - firstSpace - 1;
        target = Utf8String.FromSpan(line.Slice(NumericUnchecked.ToUSize(firstSpace + 1), NumericUnchecked.ToUSize(targetLen)));
        version = Utf8String.FromSpan(line.Slice(NumericUnchecked.ToUSize(secondSpace + 1), line.Length - NumericUnchecked.ToUSize(secondSpace + 1)));
        return true;
    }
    private bool ReadHeaders(out HttpHeaders headers, out bool connectionClose, out bool keepAlive, out long contentLength,
    out bool chunked) {
        headers = new HttpHeaders();
        connectionClose = false;
        keepAlive = false;
        contentLength = 0;
        chunked = false;
        while (true)
        {
            if (!TryReadLine (out var line)) {
                return false;
            }
            if (line.Length == 0usize)
            {
                break;
            }
            let colon = FindByte(line, NumericUnchecked.ToByte(':'));
            if (colon <0)
            {
                continue;
            }
            let nameSpan = Trim(line.Slice(0usize, NumericUnchecked.ToUSize(colon)));
            let valueSpan = Trim(line.Slice(NumericUnchecked.ToUSize(colon + 1), line.Length - NumericUnchecked.ToUSize(colon + 1)));
            let name = Utf8String.FromSpan(nameSpan);
            let value = Utf8String.FromSpan(valueSpan);
            headers.Set(name, value);
            if (EqualsIgnoreCase (nameSpan, "connection"))
            {
                if (ContainsToken (valueSpan, "close"))
                {
                    connectionClose = true;
                }
                if (ContainsToken (valueSpan, "keep-alive"))
                {
                    connectionClose = false;
                    keepAlive = true;
                }
            }
            if (EqualsIgnoreCase (nameSpan, "content-length"))
            {
                contentLength = ParseDecimal(valueSpan);
            }
            if (EqualsIgnoreCase (nameSpan, "transfer-encoding") && ContainsToken (valueSpan, "chunked"))
            {
                chunked = true;
            }
        }
        return true;
    }
    private bool TryReadLine(out ReadOnlySpan <byte >line) {
        while (true)
        {
            let crlfIndex = FindCrlf();
            if (crlfIndex >= 0)
            {
                let length = NumericUnchecked.ToUSize(crlfIndex - _start);
                line = ReadOnlySpan <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_start), length);
                _start = crlfIndex + 2;
                if (_start >= _end)
                {
                    _start = 0;
                    _end = 0;
                }
                return true;
            }
            if (!ReadMore ())
            {
                line = ReadOnlySpan <byte >.Empty;
                return false;
            }
        }
    }
    private int FindCrlf() {
        var idx = _start;
        while (idx + 1 <_end)
        {
            if (_buffer[idx] == 13 && _buffer[idx + 1] == 10)
            {
                return idx;
            }
            idx += 1;
        }
        return - 1;
    }
    private bool ReadMore() {
        if (_start >0 && _start == _end)
        {
            _start = 0;
            _end = 0;
        }
        if (_end >= _buffer.Length)
        {
            if (_start >0)
            {
                ShiftBuffer();
            }
            else
            {
                throw new Std.IOException("line too long");
            }
        }
        let available = _buffer.Length - _end;
        var span = Span <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_end), NumericUnchecked.ToUSize(available));
        let read = _stream.Read(span);
        if (read == 0)
        {
            return false;
        }
        _end += read;
        return true;
    }
    private void ShiftBuffer() {
        if (_start == 0)
        {
            return;
        }
        let length = _end - _start;
        if (length >0)
        {
            var target = Span <byte >.FromArray(ref _buffer).Slice(0usize, NumericUnchecked.ToUSize(length));
            let source = ReadOnlySpan <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_start), NumericUnchecked.ToUSize(length));
            target.CopyFrom(source);
        }
        _start = 0;
        _end = length;
    }
    private int CopyFromBuffer(Span <byte >destination) {
        if (_start >= _end)
        {
            return 0;
        }
        let available = _end - _start;
        let toCopy = available;
        if (toCopy >destination.Length)
        {
            toCopy = destination.Length;
        }
        let source = ReadOnlySpan <byte >.FromArray(ref _buffer).Slice(NumericUnchecked.ToUSize(_start), NumericUnchecked.ToUSize(toCopy));
        destination.Slice(0usize, NumericUnchecked.ToUSize(toCopy)).CopyFrom(source);
        _start += toCopy;
        if (_start >= _end)
        {
            _start = 0;
            _end = 0;
        }
        return toCopy;
    }
    private int ReadInto(Span <byte >destination) {
        var total = CopyFromBuffer(destination);
        while (total <destination.Length)
        {
            let slice = destination.Slice(NumericUnchecked.ToUSize(total), NumericUnchecked.ToUSize(destination.Length - total));
            let read = _stream.Read(slice);
            if (read == 0)
            {
                break;
            }
            total += read;
        }
        return total;
    }
    private void ConsumeCrlf() {
        var tmp = new byte[2];
        var span = Span <byte >.FromArray(ref tmp);
        let read = ReadInto(span);
        if (read <2)
        {
            throw new Std.IOException("unexpected end of stream after chunk");
        }
    }
    private long ParseDecimal(ReadOnlySpan <byte >span) {
        var value = 0L;
        var idx = 0usize;
        while (idx <span.Length)
        {
            let b = span[idx];
            if (b <NumericUnchecked.ToByte ('0') || b >NumericUnchecked.ToByte ('9'))
            {
                break;
            }
            value = value * 10L + NumericUnchecked.ToInt64(b - NumericUnchecked.ToByte('0'));
            idx += 1usize;
        }
        return value;
    }
    private usize ParseHex(ReadOnlySpan <byte >span) {
        var value = 0usize;
        var idx = 0usize;
        while (idx <span.Length)
        {
            let b = span[idx];
            var digit = - 1;
            if (b >= NumericUnchecked.ToByte ('0') && b <= NumericUnchecked.ToByte ('9'))
            {
                digit = NumericUnchecked.ToInt32(b - NumericUnchecked.ToByte('0'));
            }
            else if (b >= NumericUnchecked.ToByte ('a') && b <= NumericUnchecked.ToByte ('f'))
            {
                digit = NumericUnchecked.ToInt32(b - NumericUnchecked.ToByte('a')) + 10;
            }
            else if (b >= NumericUnchecked.ToByte ('A') && b <= NumericUnchecked.ToByte ('F'))
            {
                digit = NumericUnchecked.ToInt32(b - NumericUnchecked.ToByte('A')) + 10;
            }
            else
            {
                break;
            }
            value = (value << 4) + NumericUnchecked.ToUSize(digit);
            idx += 1usize;
        }
        return value;
    }
    private ReadOnlySpan <byte >Trim(ReadOnlySpan <byte >span) {
        var start = 0usize;
        var end = span.Length;
        while (start <span.Length && IsWhitespace (span[start]))
        {
            start += 1usize;
        }
        while (end >start && IsWhitespace (span[end - 1usize]))
        {
            end -= 1usize;
        }
        return span.Slice(start, end - start);
    }
    private static bool IsWhitespace(byte value) {
        return value == NumericUnchecked.ToByte(' ') || value == NumericUnchecked.ToByte('\t') || value == NumericUnchecked.ToByte('\r') || value == NumericUnchecked.ToByte('\n');
    }
    private static int FindByte(ReadOnlySpan <byte >span, byte value) {
        var idx = 0usize;
        while (idx <span.Length)
        {
            if (span[idx] == value)
            {
                return NumericUnchecked.ToInt32(idx);
            }
            idx += 1usize;
        }
        return - 1;
    }
    private static int FindByteFrom(ReadOnlySpan <byte >span, byte value, usize start) {
        var idx = start;
        while (idx <span.Length)
        {
            if (span[idx] == value)
            {
                return NumericUnchecked.ToInt32(idx);
            }
            idx += 1usize;
        }
        return - 1;
    }
    private static bool EqualsIgnoreCase(ReadOnlySpan <byte >span, string text) {
        let utf8 = text.AsUtf8Span();
        if (utf8.Length != span.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <span.Length)
        {
            var a = ToLower(span[idx]);
            var b = ToLower(utf8[idx]);
            if (a != b)
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
    private static bool ContainsToken(ReadOnlySpan <byte >span, string token) {
        let normalized = token.AsUtf8Span();
        var idx = 0usize;
        while (idx + normalized.Length <= span.Length)
        {
            let slice = span.Slice(idx, normalized.Length);
            if (EqualsIgnoreCase (slice, token))
            {
                return true;
            }
            idx += 1usize;
        }
        return false;
    }
    private static byte ToLower(byte value) {
        if (value >= NumericUnchecked.ToByte ('A') && value <= NumericUnchecked.ToByte ('Z'))
        {
            return NumericUnchecked.ToByte(value + NumericUnchecked.ToByte(32));
        }
        return value;
    }
    private void WriteStatusLine(int statusCode) {
        let reason = ReasonPhrase(statusCode);
        let line = "HTTP/1.1 " + statusCode.ToString();
        if (reason.Length >0)
        {
            line = line + " " + reason;
        }
        line = line + "\r\n";
        WriteAscii(line);
    }
    private void WriteHeaders(HttpHeaders headers) {
        let iter = headers.Iterate();
        while (iter.Next (out var name, out var value)) {
            WriteAscii(name);
            WriteAscii(": ");
            WriteAscii(value);
            WriteAscii("\r\n");
        }
    }
    private void WriteAscii(string text) {
        let utf8 = text.AsUtf8Span();
        if (utf8.Length == 0usize)
        {
            return;
        }
        _stream.Write(utf8);
    }
    private static string ReasonPhrase(int statusCode) {
        if (statusCode == 200)
        {
            return "OK";
        }
        if (statusCode == 201)
        {
            return "Created";
        }
        if (statusCode == 204)
        {
            return "No Content";
        }
        if (statusCode == 400)
        {
            return "Bad Request";
        }
        if (statusCode == 404)
        {
            return "Not Found";
        }
        if (statusCode == 405)
        {
            return "Method Not Allowed";
        }
        if (statusCode == 500)
        {
            return "Internal Server Error";
        }
        return "";
    }
}
