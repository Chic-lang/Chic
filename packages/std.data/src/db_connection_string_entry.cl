namespace Std.Data;
internal struct DbConnectionStringEntry
{
    internal string Key;
    internal string Value;
    internal init(string key, string value) {
        Key = key;
        Value = value;
    }
}
