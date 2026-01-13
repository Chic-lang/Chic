namespace Std.Numeric;
public interface IComparisonOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator <(TSelf left, TOther right);
    static abstract TResult operator <= (TSelf left, TOther right);
    static abstract TResult operator >(TSelf left, TOther right);
    static abstract TResult operator >= (TSelf left, TOther right);
}
