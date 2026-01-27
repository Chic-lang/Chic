namespace Std.Globalization;
import Std.Runtime;
/// Minimal culture-aware format provider abstraction used by numeric and date/time conversions.
public interface IFormatProvider
{
    string CultureId {
        get;
    }
}
/// Invariant format provider that always resolves to the invariant culture.
public sealed class InvariantFormatProvider : IFormatProvider
{
    private string _cultureId;
    public string CultureId {
        get {
            return _cultureId;
        }
    }
    public init() {
        _cultureId = StringRuntime.FromStr("invariant");
    }
}
/// Simple culture provider backed by a string culture identifier (e.g., "en-US" or "fr-FR").
public sealed class CultureFormatProvider : IFormatProvider
{
    private string _cultureId;
    public string CultureId {
        get {
            return _cultureId;
        }
    }
    public init(string cultureId) {
        if (cultureId == null)
        {
            _cultureId = StringRuntime.FromStr("invariant");
            return;
        }
        _cultureId = cultureId;
    }
}
/// Shared helpers for normalising format providers for culture-aware conversions.
public static class FormatProviderHelpers
{
    public static string ResolveCulture(IFormatProvider provider) {
        if (provider == null)
        {
            return "invariant";
        }
        let culture = provider.CultureId;
        if (culture == null || culture.Length == 0)
        {
            return "invariant";
        }
        return culture;
    }
}
