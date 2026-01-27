namespace Chic.Web;
import Std.Collections;
import Std.Numeric;
import Std.Span;
import Std.Strings;
/// <summary>Simple query-string key/value store.</summary>
public sealed class QueryCollection
{
    private HashMap <string, string >_values;
    public init() {
        _values = new HashMap <string, string >();
    }
    public static QueryCollection Parse(string query) {
        var collection = new QueryCollection();
        collection.ParseInto(query);
        return collection;
    }
    public bool TryGetValue(string name, out string value) {
        if (name == null)
        {
            value = "";
            return false;
        }
        let entry = _values.Get(name);
        if (entry.IsSome (out var found)) {
            value = found;
            return true;
        }
        value = "";
        return false;
    }
    public string GetValueOrDefault(string name, string defaultValue) {
        if (TryGetValue (name, out var value)) {
            return value;
        }
        return defaultValue;
    }
    internal HashMapIterator <string, string >Iterate() {
        return _values.Iter();
    }
    private void ParseInto(string query) {
        if (query == null)
        {
            return;
        }
        var source = query;
        if (source.Length == 0)
        {
            return;
        }
        if (source[0] == '?')
        {
            source = source.Substring(1);
        }
        let utf8 = source.AsUtf8Span();
        var start = 0usize;
        var idx = 0usize;
        while (idx <= utf8.Length)
        {
            if (idx == utf8.Length || utf8[idx] == NumericUnchecked.ToByte ('&'))
            {
                let pairLen = idx - start;
                if (pairLen >0usize)
                {
                    let pair = utf8.Slice(start, pairLen);
                    let eqIndex = FindByte(pair, NumericUnchecked.ToByte('='));
                    var key = "";
                    var value = "";
                    if (eqIndex >= 0)
                    {
                        let keySpan = pair.Slice(0usize, NumericUnchecked.ToUSize(eqIndex));
                        let valueStart = NumericUnchecked.ToUSize(eqIndex + 1);
                        let valueLen = pairLen - valueStart;
                        let valueSpan = pair.Slice(valueStart, valueLen);
                        key = Utf8String.FromSpan(keySpan);
                        value = Utf8String.FromSpan(valueSpan);
                    }
                    else
                    {
                        key = Utf8String.FromSpan(pair);
                        value = "";
                    }
                    _values.Insert(key, value, out var existing);
                }
                idx += 1usize;
                start = idx;
                continue;
            }
            idx += 1usize;
        }
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
}
