namespace Std.Data;
import Foundation.Collections;
import Std;
import Std.Collections;
import Std.Numeric;
import Std.Testing;
/// <summary>Parses and constructs simple key=value connection strings.</summary>
public class DbConnectionStringBuilder
{
    private VecPtr _entries;
    /// <summary>Creates an empty builder.</summary>
    public init() {
        _entries = Vec.New<DbConnectionStringEntry>();
    }
    /// <summary>Creates a builder and parses the provided connection string.</summary>
    public init(string connectionString) {
        _entries = Vec.New<DbConnectionStringEntry>();
        if (connectionString != null && connectionString.Length >0)
        {
            ApplyConnectionString(connectionString);
        }
    }
    /// <summary>Gets the number of entries tracked by the builder.</summary>
    public int Count {
        get {
            return NumericUnchecked.ToInt32(Vec.Len(in _entries));
        }
    }
    /// <summary>Gets or sets the value for the specified key.</summary>
    public string this[string key] {
        get {
            if (TryGetValue (key, out var value)) {
                return value;
            }
            throw new Std.ArgumentException("Connection string does not contain: " + key);
        }
        set {
            Set(key, value);
        }
    }
    /// <summary>Clears all entries.</summary>
    public void Clear() {
        var owner = this;
        VecIntrinsics.chic_rt_vec_clear(ref owner._entries);
    }
    /// <summary>Sets or replaces a key/value pair.</summary>
    public void Set(string key, string value) {
        var owner = this;
        if (key == null)
        {
            throw new Std.ArgumentNullException("key");
        }
        if (value == null)
        {
            value = "";
        }
        let span = Vec.AsSpan <DbConnectionStringEntry >(ref owner._entries);
        var length = span.Length;
        var idx = 0usize;
        while (idx <length)
        {
            if (span[idx].Key == key)
            {
                span[idx] = new DbConnectionStringEntry(key, value);
                return;
            }
            idx += 1usize;
        }
        let status = Vec.Push <DbConnectionStringEntry >(ref owner._entries, new DbConnectionStringEntry(key, value));
        if (status != VecError.Success)
        {
            throw new DbException("Failed to add connection string entry");
        }
    }
    /// <summary>Tries to retrieve a value by key.</summary>
    public bool TryGetValue(string key, out string value) {
        if (key == null)
        {
            throw new Std.ArgumentNullException("key");
        }
        let span = Vec.AsReadOnlySpan <DbConnectionStringEntry >(in _entries);
        var idx = 0usize;
        while (idx <span.Length)
        {
            let entry = span[idx];
            if (entry.Key == key)
            {
                value = entry.Value;
                return true;
            }
            idx += 1usize;
        }
        value = "";
        return false;
    }
    /// <summary>Returns true if the builder contains the given key.</summary>
    public bool ContainsKey(string key) {
        return TryGetValue(key, out var existing);
    }
    /// <summary>Parses and applies a full connection string.</summary>
    public void ApplyConnectionString(string connectionString) {
        if (connectionString == null)
        {
            throw new Std.ArgumentNullException("connectionString");
        }
        let length = connectionString.Length;
        var start = 0;
        while (start <length)
        {
            var end = start;
            while (end <length && connectionString[end] != ';')
            {
                end += 1;
            }
            let segmentLength = end - start;
            if (segmentLength >0)
            {
                AddSegment(connectionString, start, segmentLength);
            }
            start = end + 1;
        }
    }
    /// <summary>Builds a connection string from the current entries.</summary>
    public string ToString() {
        var result = "";
        let span = Vec.AsReadOnlySpan <DbConnectionStringEntry >(in _entries);
        var idx = 0usize;
        while (idx <span.Length)
        {
            let entry = span[idx];
            if (result.Length >0)
            {
                result += ";";
            }
            result += entry.Key + "=" + entry.Value;
            idx += 1usize;
        }
        return result;
    }
    /// <summary>Releases builder resources.</summary>
    public void dispose(ref this) {
        VecIntrinsics.chic_rt_vec_drop(ref _entries);
    }
    private void AddSegment(string source, int start, int length) {
        var separatorIndex = - 1;
        var idx = 0;
        while (idx <length)
        {
            if (source[start + idx] == '=')
            {
                separatorIndex = idx;
                break;
            }
            idx += 1;
        }
        if (separatorIndex <= 0)
        {
            return;
        }
        let key = source.Substring(start, separatorIndex);
        let valueStart = start + separatorIndex + 1;
        let valueLength = length - separatorIndex - 1;
        var value = "";
        if (valueLength >0)
        {
            value = source.Substring(valueStart, valueLength);
        }
        Set(key, value);
    }
}

testcase Given_db_connection_string_builder_default_count_zero_When_executed_Then_db_connection_string_builder_default_count_zero()
{
    var builder = new DbConnectionStringBuilder();
    Assert.That(builder.Count).IsEqualTo(0);
    builder.dispose();
}

testcase Given_db_connection_string_builder_set_get_value_When_executed_Then_db_connection_string_builder_set_get_value()
{
    var builder = new DbConnectionStringBuilder();
    builder.Set("server", "local");
    let ok = builder.TryGetValue("server", out var value);
    let matches = ok && value == "local";
    Assert.That(matches).IsTrue();
    builder.dispose();
}

testcase Given_db_connection_string_builder_contains_key_true_When_executed_Then_db_connection_string_builder_contains_key_true()
{
    var builder = new DbConnectionStringBuilder();
    builder.Set("host", "db");
    Assert.That(builder.ContainsKey("host")).IsTrue();
    builder.dispose();
}

testcase Given_db_connection_string_builder_try_get_missing_returns_false_When_executed_Then_db_connection_string_builder_try_get_missing_returns_false()
{
    var builder = new DbConnectionStringBuilder();
    let ok = builder.TryGetValue("missing", out var value);
    let _ = value;
    Assert.That(ok).IsFalse();
    builder.dispose();
}

testcase Given_db_connection_string_builder_apply_connection_string_reads_value_When_executed_Then_db_connection_string_builder_apply_connection_string_reads_value()
{
    var builder = new DbConnectionStringBuilder();
    builder.ApplyConnectionString("user=chic;role=admin");
    let ok = builder.TryGetValue("role", out var value);
    let matches = ok && value == "admin";
    Assert.That(matches).IsTrue();
    builder.dispose();
}

testcase Given_db_connection_string_builder_to_string_roundtrip_When_executed_Then_db_connection_string_builder_to_string_roundtrip()
{
    var builder = new DbConnectionStringBuilder();
    builder.Set("a", "1");
    builder.Set("b", "2");
    Assert.That(builder.ToString()).IsEqualTo("a=1;b=2");
    builder.dispose();
}

testcase Given_db_connection_string_builder_set_null_key_throws_When_executed_Then_db_connection_string_builder_set_null_key_throws()
{
    var builder = new DbConnectionStringBuilder();
    Assert.Throws<ArgumentNullException>(() => {
        builder.Set(null, "value");
    });
    builder.dispose();
}
