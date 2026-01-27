namespace Std;
import Std.Numeric;
// Bridge interfaces so Std primitives can inherit the familiar names without
// repeating Std.Numeric qualifications. Each bridge simply aliases the
// corresponding Std.Numeric contract.
public interface IComparable : Std.Numeric.IComparable <Object >
{
}
public interface IComparable <TSelf >: Std.Numeric.IComparable <TSelf >
{
}
public interface IConvertible : Std.Numeric.IConvertible
{
}
public interface IEquatable <TSelf >: Std.Numeric.IEquatable <TSelf >
{
}
public interface IParsable <TSelf >: Std.Numeric.IParsable <TSelf >
{
}
public interface ISpanParsable <TSelf >: Std.Numeric.ISpanParsable <TSelf >
{
}
public interface IUtf8SpanParsable <TSelf >: Std.Numeric.IUtf8SpanParsable <TSelf >
{
}
public interface IAdditionOperators <TSelf, TOther, TResult >: Std.Numeric.IAdditionOperators <TSelf, TOther, TResult >
{
}
public interface IAdditiveIdentity <TSelf, TResult >: Std.Numeric.IAdditiveIdentity <TSelf, TResult >
{
}
public interface IBinaryInteger <TSelf >: Std.Numeric.IBinaryInteger <TSelf >
{
}
public interface IBinaryNumber <TSelf >: Std.Numeric.IBinaryNumber <TSelf >
{
}
public interface IBitwiseOperators <TSelf, TOther, TResult >: Std.Numeric.IBitwiseOperators <TSelf, TOther, TResult >
{
}
public interface IComparisonOperators <TSelf, TOther, TResult >: Std.Numeric.IComparisonOperators <TSelf, TOther, TResult >
{
}
public interface IDecrementOperators <TSelf >: Std.Numeric.IDecrementOperators <TSelf >
{
}
public interface IDivisionOperators <TSelf, TOther, TResult >: Std.Numeric.IDivisionOperators <TSelf, TOther, TResult >
{
}
public interface IEqualityOperators <TSelf, TOther, TResult >: Std.Numeric.IEqualityOperators <TSelf, TOther, TResult >
{
}
public interface IIncrementOperators <TSelf >: Std.Numeric.IIncrementOperators <TSelf >
{
}
public interface IMinMaxValue <TSelf >: Std.Numeric.IMinMaxValue <TSelf >
{
}
public interface IModulusOperators <TSelf, TOther, TResult >: Std.Numeric.IModulusOperators <TSelf, TOther, TResult >
{
}
public interface IMultiplicativeIdentity <TSelf, TResult >: Std.Numeric.IMultiplicativeIdentity <TSelf, TResult >
{
}
public interface IMultiplyOperators <TSelf, TOther, TResult >: Std.Numeric.IMultiplyOperators <TSelf, TOther, TResult >
{
}
public interface INumber <TSelf >: Std.Numeric.INumber <TSelf >
{
}
public interface INumberBase <TSelf >: Std.Numeric.INumberBase <TSelf >
{
}
public interface IShiftOperators <TSelf, TOther, TResult >: Std.Numeric.IShiftOperators <TSelf, TOther, TResult >
{
}
public interface ISignedNumber <TSelf >: Std.Numeric.ISignedNumber <TSelf >
{
}
public interface ISubtractionOperators <TSelf, TOther, TResult >: Std.Numeric.ISubtractionOperators <TSelf, TOther, TResult >
{
}
public interface IUnaryNegationOperators <TSelf, TResult >: Std.Numeric.IUnaryNegationOperators <TSelf, TResult >
{
}
public interface IUnaryPlusOperators <TSelf, TResult >: Std.Numeric.IUnaryPlusOperators <TSelf, TResult >
{
}
public interface IUnsignedNumber <TSelf >: Std.Numeric.IUnsignedNumber <TSelf >
{
}
public interface ISpanFormattable : Std.Numeric.ISpanFormattable
{
}
public interface IUtf8SpanFormattable : Std.Numeric.IUtf8SpanFormattable
{
}
public interface IFormattable : Std.Numeric.IFormattable
{
}
