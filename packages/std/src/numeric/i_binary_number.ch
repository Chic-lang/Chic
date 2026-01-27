namespace Std.Numeric;
public interface IBinaryNumber <TSelf >: INumber <TSelf >
{
    bool IsPowerOfTwo(TSelf value);
}
