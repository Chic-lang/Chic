namespace Std.Numeric;
public interface IUnaryNegationOperators <TSelf, TResult >
{
    static abstract TResult operator - (TSelf value);
}
