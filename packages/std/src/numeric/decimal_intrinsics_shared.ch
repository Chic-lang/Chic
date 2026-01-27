namespace Std.Numeric.Decimal;
public static class DecimalIntrinsicsShared
{
    /// Build a `DecimalIntrinsicResult` from a status, value, and variant.
    public static DecimalIntrinsicResult BuildResult(DecimalStatus status, decimal value, DecimalIntrinsicVariant variant) {
        return new DecimalIntrinsicResult(status, value, variant);
    }
}
