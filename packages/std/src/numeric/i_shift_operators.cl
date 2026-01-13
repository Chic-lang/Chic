namespace Std.Numeric;
public interface IShiftOperators <TSelf, TOffset, TResult >
{
    static abstract TResult operator << (TSelf value, TOffset offset);
    static abstract TResult operator >> (TSelf value, TOffset offset);
}
