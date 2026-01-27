namespace Std.Numeric;
public interface IEqualityOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator == (TSelf left, TOther right);
    static abstract TResult operator != (TSelf left, TOther right);
}
