namespace Std.Numeric.Decimal;
public static class DecimalIntrinsicsFma
{
    public static DecimalIntrinsicResult Fma(decimal a, decimal b, decimal c) {
        return DecimalIntrinsics.Fma(a, b, c);
    }
}
