namespace Std.Numeric.Decimal;
import Std.Runtime.InteropServices;
@Intrinsic @StructLayout(LayoutKind.Sequential) public readonly struct DecimalIntrinsicResult
{
    public readonly DecimalStatus Status;
    public readonly decimal Value;
    public readonly DecimalIntrinsicVariant Variant;
    public init(DecimalStatus status, decimal value, DecimalIntrinsicVariant variant) {
        Status = status;
        Value = value;
        Variant = variant;
    }
}
