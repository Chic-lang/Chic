namespace Std.Numeric;
public interface IFormattable
{
    string Format(string format, string culture);
    string ToString(string format);
    string ToString(string format, string culture);
}
