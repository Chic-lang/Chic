namespace Std.Datetime;
/// Minimal resolver that currently treats known Chic cultures as invariant for date/time formatting and parsing.
public static class DateTimeCultures
{
    public static IDateTimeCulture Resolve(string cultureId) {
        if (cultureId == null || cultureId.Length == 0)
        {
            return InvariantDateTimeCulture.Instance;
        }
        if (cultureId == "invariant")
        {
            return InvariantDateTimeCulture.Instance;
        }
        // Cultures map to invariant formatting/parsing today; add localisation later as needed.
        return InvariantDateTimeCulture.Instance;
    }
}
