namespace Std.Numeric;
public interface IMinMaxValue <TSelf >
{
    TSelf Min(TSelf left, TSelf right);
    TSelf Max(TSelf left, TSelf right);
}
