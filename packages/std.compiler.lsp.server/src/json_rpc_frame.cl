namespace Std.Compiler.Lsp.Server;
import Std.Numeric;
import Std.Span;
import Std.Strings;
public struct JsonRpcFrame
{
    public int ContentLength;
    public string Body;
    public init(string body) {
        Body = body;
        ContentLength = NumericUnchecked.ToInt32(ReadOnlySpan.FromString(body).Length);
    }
    public string ToWire(in this) {
        let lengthText = JsonRpcFrameIntrinsics.chic_rt_startup_i32_to_string(ContentLength);
        return "Content-Length: " + lengthText + "\r\n\r\n" + Body;
    }
    public static bool TryParse(string wire, out JsonRpcFrame frame) {
        frame = new JsonRpcFrame("");
        let bytes = ReadOnlySpan.FromString(wire);
        let headerEnd = FindHeaderEnd(in bytes);
        if (headerEnd <0)
        {
            return false;
        }
        if (!TryParseContentLength (in bytes, NumericUnchecked.ToUSize(headerEnd), out var len)) {
            return false;
        }
        let bodyStart = NumericUnchecked.ToUSize(headerEnd) + 4usize;
        let bodyLen = NumericUnchecked.ToUSize(len);
        if (bodyStart + bodyLen >bytes.Length)
        {
            return false;
        }
        let bodyBytes = bytes.Slice(bodyStart, bodyLen);
        let body = Utf8String.FromSpan(bodyBytes);
        frame = new JsonRpcFrame(body);
        return true;
    }
    private static int FindHeaderEnd(in ReadOnlySpan <byte >bytes) {
        if (bytes.Length <4usize)
        {
            return - 1;
        }
        var i = 0usize;
        let limit = bytes.Length - 4usize;
        while (i <= limit)
        {
            if (bytes[i] == 13u8 && bytes[i + 1usize] == 10u8 && bytes[i + 2usize] == 13u8 && bytes[i + 3usize] == 10u8)
            {
                return(int) i;
            }
            i += 1usize;
        }
        return - 1;
    }
    private static bool TryParseContentLength(in ReadOnlySpan <byte >bytes, usize headerEnd, out int value) {
        value = 0;
        let prefixText = "Content-Length:";
        let prefixBytes = ReadOnlySpan.FromString(prefixText);
        if (headerEnd <prefixBytes.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <prefixBytes.Length)
        {
            if (bytes[idx] != prefixBytes[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        while (idx <headerEnd && bytes[idx] == 32u8)
        {
            idx += 1usize;
        }
        if (idx >= headerEnd)
        {
            return false;
        }
        var acc = 0;
        var sawDigit = false;
        while (idx <headerEnd)
        {
            let ch = bytes[idx];
            if (ch <48u8 || ch >57u8)
            {
                break;
            }
            sawDigit = true;
            acc = (acc * 10) + ((int) ch - 48);
            idx += 1usize;
        }
        if (!sawDigit)
        {
            return false;
        }
        value = acc;
        return true;
    }
}
