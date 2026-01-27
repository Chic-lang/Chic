namespace Std.Numeric;
import Std.Span;
public interface IUtf8SpanParsable <TSelf >
{
    TSelf Parse(ReadOnlySpan <byte >text);
    bool TryParse(ReadOnlySpan <byte >text, out TSelf value);
}
