namespace Std.Numeric;
import Std.Span;
public interface ISpanFormattable
{
    bool TryFormat(Span <byte >destination, out usize written);
    bool TryFormat(Span <byte >destination, out usize written, string format);
    bool TryFormat(Span <byte >destination, out usize written, string format, string culture);
}
