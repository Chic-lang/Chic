namespace Std.Numeric;
public interface ISubtractionOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator - (TSelf left, TOther right);
}
