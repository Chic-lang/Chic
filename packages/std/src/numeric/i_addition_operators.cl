namespace Std.Numeric;
public interface IAdditionOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator + (TSelf left, TOther right);
}
