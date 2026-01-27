namespace Std.Numeric;
public interface IBitwiseOperators <TSelf, TOther, TResult >
{
    static abstract TResult operator & (TSelf left, TOther right);
    static abstract TResult operator | (TSelf left, TOther right);
    static abstract TResult operator ^ (TSelf left, TOther right);
    static abstract TResult operator ~ (TSelf value);
}
