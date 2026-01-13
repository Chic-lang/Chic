namespace Std.Numeric;
public interface IDivisionOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator / (TSelf left, TOther right);
}
