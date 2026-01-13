namespace Std.Numeric;
public interface IParsable <TSelf >
{
    TSelf Parse(string text);
    bool TryParse(string text, out TSelf value);
}
