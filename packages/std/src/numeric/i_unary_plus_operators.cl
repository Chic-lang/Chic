namespace Std.Numeric;
public interface IUnaryPlusOperators <TSelf, TResult >
{
    static abstract TResult operator + (TSelf value);
}
