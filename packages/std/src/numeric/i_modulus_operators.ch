namespace Std.Numeric;
public interface IModulusOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator % (TSelf left, TOther right);
}
