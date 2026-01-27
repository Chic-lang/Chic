namespace Std.Numeric;
public interface IMultiplyOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator * (TSelf left, TOther right);
}
