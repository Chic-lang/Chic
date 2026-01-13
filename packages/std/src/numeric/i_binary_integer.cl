namespace Std.Numeric;
public interface IBinaryInteger <TSelf >: IBinaryNumber <TSelf >, IBitwiseOperators <TSelf, TSelf, TSelf >, IShiftOperators <TSelf, int, TSelf >
{
    int LeadingZeroCount(TSelf value);
    int TrailingZeroCount(TSelf value);
    int PopCount(TSelf value);
    TSelf RotateLeft(TSelf value, int offset);
    TSelf RotateRight(TSelf value, int offset);
    TSelf ReverseEndianness(TSelf value);
}
