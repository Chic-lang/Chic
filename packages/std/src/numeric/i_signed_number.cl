namespace Std.Numeric;
public interface ISignedNumber <TSelf >: INumber <TSelf >, IUnaryNegationOperators <TSelf, TSelf >
{
    TSelf NegativeOne();
    TSelf Abs(TSelf value);
}
