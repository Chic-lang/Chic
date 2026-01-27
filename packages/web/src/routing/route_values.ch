namespace Chic.Web;
import Std.Collections;
/// <summary>Stores route parameter captures for the current request.</summary>
public sealed class RouteValues
{
    private HashMap <string, string >_values;
    public init() {
        _values = new HashMap <string, string >();
    }
    public void Set(string name, string value) {
        if (name == null)
        {
            return;
        }
        var stored = value;
        if (stored == null)
        {
            stored = "";
        }
        _values.Insert(name, stored, out var existing);
    }
    public bool TryGetValue(string name, out string value) {
        if (name == null)
        {
            value = "";
            return false;
        }
        let found = _values.Get(name);
        if (found.IsSome (out var captured)) {
            value = captured;
            return true;
        }
        value = "";
        return false;
    }
}
