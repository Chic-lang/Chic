namespace Std.Data.Mapping;
import Std.Numeric;
import Std.Span;
import Std.Strings;
import Std.Runtime;
import Std.Testing;
/// <summary>Controls how auto-mapping resolves column names to target members.</summary>
public struct MappingOptions
{
    /// <summary>Whether comparisons should consider case (default: false).</summary>
    public bool CaseSensitive;
    /// <summary>Whether underscores should be stripped when matching names (default: false).</summary>
    public bool UnderscoreToCamel;
    /// <summary>Creates default options (case-insensitive, no underscore folding).</summary>
    public static MappingOptions Default() {
        var options = new MappingOptions();
        options.CaseSensitive = false;
        options.UnderscoreToCamel = false;
        return options;
    }
    /// <summary>Returns a normalised representation of the supplied name for comparisons.</summary>
    public string Normalize(string name) {
        if (name == null)
        {
            return StringRuntime.Create();
        }
        return name;
    }
}
testcase Given_mapping_options_default_case_sensitive_false_When_executed_Then_mapping_options_default_case_sensitive_false()
{
    let options = MappingOptions.Default();
    Assert.That(options.CaseSensitive).IsFalse();
}
testcase Given_mapping_options_default_underscore_to_camel_false_When_executed_Then_mapping_options_default_underscore_to_camel_false()
{
    let options = MappingOptions.Default();
    Assert.That(options.UnderscoreToCamel).IsFalse();
}
testcase Given_mapping_options_normalize_null_returns_empty_When_executed_Then_mapping_options_normalize_null_returns_empty()
{
    let options = MappingOptions.Default();
    let value = options.Normalize(null);
    Assert.That(value.Length).IsEqualTo(0);
}
