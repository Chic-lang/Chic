namespace Chic.Web;
import Std.Core;
import Std.Numeric;
import Std.Span;
import Std.Strings;
/// <summary>Parsed representation of a simple route template (literal segments and {parameters}).</summary>
public sealed class RouteTemplate
{
    private string[] _segments;
    private bool[] _isParameter;
    private int _count;
    public init(string template) {
        var value = template;
        if (value == null || value.Length == 0)
        {
            value = "/";
        }
        Parse(value);
    }
    public bool TryMatch(string path, out RouteValues values) {
        values = new RouteValues();
        var candidate = path;
        if (candidate == null)
        {
            candidate = "";
        }
        var parts = CoreIntrinsics.DefaultValue <string[] >();
        var partCount = 0;
        ExtractSegments(candidate, out parts, out partCount);
        if (partCount != _count)
        {
            return false;
        }
        var idx = 0;
        while (idx <_count)
        {
            if (_isParameter[idx])
            {
                values.Set(_segments[idx], parts[idx]);
            }
            else if (_segments[idx] != parts[idx])
            {
                return false;
            }
            idx += 1;
        }
        return true;
    }
    private void Parse(string template) {
        var parts = CoreIntrinsics.DefaultValue <string[] >();
        var count = 0;
        ExtractSegments(template, out parts, out count);
        _segments = new string[count];
        _isParameter = new bool[count];
        _count = count;
        var idx = 0;
        while (idx <count)
        {
            let part = parts[idx];
            if (part.Length >= 2 && part[0] == '{' && part[part.Length - 1] == '}')
            {
                let name = part.Substring(1, part.Length - 2);
                _segments[idx] = name;
                _isParameter[idx] = true;
            }
            else
            {
                _segments[idx] = part;
                _isParameter[idx] = false;
            }
            idx += 1;
        }
    }
    private static void ExtractSegments(string source, out string[] segments, out int count) {
        let utf8 = source.AsUtf8Span();
        let segmentCount = CountSegments(utf8);
        segments = new string[segmentCount];
        count = 0;
        var idx = 0usize;
        var start = 0usize;
        if (utf8.Length >0usize && utf8[0] == NumericUnchecked.ToByte ('/'))
        {
            idx = 1usize;
            start = 1usize;
        }
        while (idx <= utf8.Length)
        {
            if (idx == utf8.Length || utf8[idx] == NumericUnchecked.ToByte ('/'))
            {
                let length = idx - start;
                if (length >0usize)
                {
                    let slice = utf8.Slice(start, length);
                    segments[count] = Utf8String.FromSpan(slice);
                    count += 1;
                }
                idx += 1usize;
                start = idx;
                continue;
            }
            idx += 1usize;
        }
    }
    private static int CountSegments(ReadOnlySpan <byte >utf8) {
        var idx = 0usize;
        var start = 0usize;
        var count = 0;
        if (utf8.Length >0usize && utf8[0] == NumericUnchecked.ToByte ('/'))
        {
            idx = 1usize;
            start = 1usize;
        }
        while (idx <= utf8.Length)
        {
            if (idx == utf8.Length || utf8[idx] == NumericUnchecked.ToByte ('/'))
            {
                let length = idx - start;
                if (length >0usize)
                {
                    count += 1;
                }
                idx += 1usize;
                start = idx;
                continue;
            }
            idx += 1usize;
        }
        return count;
    }
}
