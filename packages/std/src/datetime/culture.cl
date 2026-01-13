namespace Std.Datetime;
import Std.Runtime;
// Minimal culture abstraction so formatting/parsing can consult separators and names
// without binding to host locale APIs.
public interface IDateTimeCulture
{
    string DateSeparator();
    string TimeSeparator();
    string UtcDesignator();
}
public static class InvariantDateTimeCulture
{
    public static IDateTimeCulture Instance => new InvariantCultureImpl();
}
internal sealed class InvariantCultureImpl : IDateTimeCulture
{
    public string DateSeparator() => StringRuntime.FromStr("-");
    public string TimeSeparator() => StringRuntime.FromStr(":");
    public string UtcDesignator() => StringRuntime.FromStr("Z");
}
