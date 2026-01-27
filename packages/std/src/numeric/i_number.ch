namespace Std.Numeric;
import Std.Span;
public interface INumber <TSelf >: INumberBase <TSelf >, IAdditionOperators <TSelf, TSelf, TSelf >, ISubtractionOperators <TSelf, TSelf, TSelf >, IMultiplyOperators <TSelf, TSelf, TSelf >, IDivisionOperators <TSelf, TSelf, TSelf >, IModulusOperators <TSelf, TSelf, TSelf >, IUnaryPlusOperators <TSelf, TSelf >, IIncrementOperators <TSelf >, IDecrementOperators <TSelf >, IComparisonOperators <TSelf, TSelf, bool >, IEqualityOperators <TSelf, TSelf, bool >, IAdditiveIdentity <TSelf, TSelf >, IMultiplicativeIdentity <TSelf, TSelf >, IMinMaxValue <TSelf >
{
    TSelf Parse(string text);
    bool TryParse(string text, out TSelf value);
    TSelf Parse(ReadOnlySpan <byte >text);
    bool TryParse(ReadOnlySpan <byte >text, out TSelf value);
    TSelf Min(TSelf left, TSelf right);
    TSelf Max(TSelf left, TSelf right);
}
