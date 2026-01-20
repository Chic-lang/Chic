namespace Std.Net.Http;
import Std.Collections;
import Std.Core;
import Std.Strings;
import Std.Span;
import Std.Numeric;
/// <summary>
/// Lightweight header collection with case-insensitive keys.
/// </summary>
public class HttpHeaders
{
    private HashMap <string, string >_headers;
    public init() {
        _headers = new HashMap <string, string >();
    }
    public void Add(string name, string value) {
        if (name == null)
        {
            throw new Std.ArgumentNullException("name");
        }
        if (value == null)
        {
            throw new Std.ArgumentNullException("value");
        }
        let key = Normalize(name);
        var previous = Std.Option <string >.None();
        let status = _headers.Insert(key, value, out previous);
        var existing = Std.Runtime.StringRuntime.Create();
        if (status == HashMapError.Success && previous.IsSome (out existing)) {
            // Merge values with a comma separator to retain ordering semantics.
            var merged = existing + ", " + value;
            _headers.Insert(key, merged, out var mergedPrevious);
        }
    }
    public void Set(string name, string value) {
        if (name == null)
        {
            throw new Std.ArgumentNullException("name");
        }
        if (value == null)
        {
            throw new Std.ArgumentNullException("value");
        }
        let key = Normalize(name);
        _headers.Insert(key, value, out var previous);
    }
    public bool Contains(string name) {
        if (name == null)
        {
            return false;
        }
        let key = Normalize(name);
        return _headers.ContainsKey(in key);
    }
    public bool TryGetValue(string name, out string value) {
        if (name == null)
        {
            value = "";
            return false;
        }
        let key = Normalize(name);
        let entry = _headers.Get(in key);
        if (entry.IsSome (out var found)) {
            value = found;
            return true;
        }
        value = "";
        return false;
    }
    public bool Remove(string name) {
        if (name == null)
        {
            return false;
        }
        let key = Normalize(name);
        return _headers.Remove(in key);
    }
    public HashMapIterator <string, string >Iterate() {
        return _headers.Iter();
    }
    private static string Normalize(string name) {
        let utf8 = name.AsUtf8Span();
        var buf = new byte[utf8.Length];
        var idx = 0usize;
        while (idx <utf8.Length)
        {
            var b = utf8[idx];
            if (b >= NumericUnchecked.ToByte (65) && b <= NumericUnchecked.ToByte (90))
            {
                b = NumericUnchecked.ToByte(b + NumericUnchecked.ToByte(32));
            }
            buf[idx] = b;
            idx += 1;
        }
        return Utf8String.FromSpan(ReadOnlySpan <byte >.FromArray(in buf));
    }
}
