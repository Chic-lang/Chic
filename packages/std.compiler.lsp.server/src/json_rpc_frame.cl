namespace Std.Compiler.Lsp.Server;

public struct JsonRpcFrame
{
    public int ContentLength;
    public string Body;

    public init(string body) {
        Body = body;
        ContentLength = body.Length;
    }

    public string ToWire(in this) {
        return "Content-Length: " + ContentLength.ToString() + "\r\n\r\n" + Body;
    }

    public static bool TryParse(string wire, out JsonRpcFrame frame) {
        frame = new JsonRpcFrame("");
        let headerEnd = wire.IndexOf("\r\n\r\n");
        if (headerEnd < 0)
        {
            return false;
        }
        let header = wire.Substring(0, headerEnd);
        let prefix = "Content-Length:";
        let prefixIndex = header.IndexOf(prefix);
        if (prefixIndex < 0)
        {
            return false;
        }
        let valueStart = prefixIndex + prefix.Length;
        if (! ParseContentLength(header, valueStart, out var len))
        {
            return false;
        }
        let bodyStart = headerEnd + 4;
        if (bodyStart + len > wire.Length)
        {
            return false;
        }
        let body = wire.Substring(bodyStart, len);
        frame = new JsonRpcFrame(body);
        return true;
    }

    private static bool ParseContentLength(string header, int start, out int value) {
        value = 0;
        if (start < 0 || start >= header.Length)
        {
            return false;
        }
        var i = start;
        while (i < header.Length && header[i] == ' ')
        {
            i += 1;
        }
        if (i >= header.Length)
        {
            return false;
        }
        var acc = 0;
        var sawDigit = false;
        while (i < header.Length)
        {
            let ch = header[i];
            if (ch < '0' || ch > '9')
            {
                break;
            }
            sawDigit = true;
            acc = (acc * 10) + ((int) ch - (int) '0');
            i += 1;
        }
        if (! sawDigit)
        {
            return false;
        }
        value = acc;
        return true;
    }
}
