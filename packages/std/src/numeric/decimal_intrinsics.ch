namespace Std.Numeric.Decimal;
import Std.Numeric;
import Std.Runtime.Collections;
public static class DecimalIntrinsics
{
    public delegate decimal Binary(decimal lhs, decimal rhs);
    public delegate decimal Ternary(decimal lhs, decimal rhs, decimal addend);
    private static DecimalIntrinsicResult Success(decimal value) {
        return new DecimalIntrinsicResult(DecimalStatus.Success, value, DecimalIntrinsicVariant.Scalar);
    }
    private static DecimalIntrinsicResult FromException(Std.Exception error) {
        if (error is Std.DivideByZeroException) {
            return new DecimalIntrinsicResult(DecimalStatus.DivisionByZero, 0m, DecimalIntrinsicVariant.Scalar);
        }
        if (error is Std.OverflowException) {
            return new DecimalIntrinsicResult(DecimalStatus.Overflow, 0m, DecimalIntrinsicVariant.Scalar);
        }
        throw error;
    }
    private static DecimalIntrinsicResult ExecuteBinary(decimal lhs, decimal rhs, Binary op) {
        try {
            return Success(op(lhs, rhs));
        }
        catch(Std.Exception error) {
            return FromException(error);
        }
        return Success(0m);
    }
    private static DecimalIntrinsicResult ExecuteTernary(decimal lhs, decimal rhs, decimal addend, Ternary op) {
        try {
            return Success(op(lhs, rhs, addend));
        }
        catch(Std.Exception error) {
            return FromException(error);
        }
        return Success(0m);
    }
    public static DecimalIntrinsicResult Add(decimal lhs, decimal rhs) => AddWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.None);
    public static DecimalIntrinsicResult AddVectorized(decimal lhs, decimal rhs) => AddWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.Decimal);
    public static DecimalIntrinsicResult AddWithOptions(decimal lhs, decimal rhs, DecimalRoundingMode rounding, DecimalVectorizeHint vectorize) {
        return ExecuteBinary(lhs, rhs, (a, b) => a + b);
    }
    public static DecimalIntrinsicResult Sub(decimal lhs, decimal rhs) => SubWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.None);
    public static DecimalIntrinsicResult SubVectorized(decimal lhs, decimal rhs) => SubWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.Decimal);
    public static DecimalIntrinsicResult SubWithOptions(decimal lhs, decimal rhs, DecimalRoundingMode rounding, DecimalVectorizeHint vectorize) {
        return ExecuteBinary(lhs, rhs, (a, b) => a - b);
    }
    public static DecimalIntrinsicResult Mul(decimal lhs, decimal rhs) => MulWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.None);
    public static DecimalIntrinsicResult MulVectorized(decimal lhs, decimal rhs) => MulWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.Decimal);
    public static DecimalIntrinsicResult MulWithOptions(decimal lhs, decimal rhs, DecimalRoundingMode rounding, DecimalVectorizeHint vectorize) {
        return ExecuteBinary(lhs, rhs, (a, b) => a * b);
    }
    public static DecimalIntrinsicResult Div(decimal lhs, decimal rhs) => DivWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.None);
    public static DecimalIntrinsicResult DivVectorized(decimal lhs, decimal rhs) => DivWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.Decimal);
    public static DecimalIntrinsicResult DivWithOptions(decimal lhs, decimal rhs, DecimalRoundingMode rounding, DecimalVectorizeHint vectorize) {
        return ExecuteBinary(lhs, rhs, (a, b) => a / b);
    }
    public static DecimalIntrinsicResult Rem(decimal lhs, decimal rhs) => RemWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.None);
    public static DecimalIntrinsicResult RemVectorized(decimal lhs, decimal rhs) => RemWithOptions(lhs, rhs, DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint.Decimal);
    public static DecimalIntrinsicResult RemWithOptions(decimal lhs, decimal rhs, DecimalRoundingMode rounding, DecimalVectorizeHint vectorize) {
        return ExecuteBinary(lhs, rhs, (a, b) => a % b);
    }
    public static DecimalIntrinsicResult Fma(decimal lhs, decimal rhs, decimal addend) => FmaWithOptions(lhs, rhs, addend,
    DecimalRoundingMode.TiesToEven, DecimalVectorizeHint.None);
    public static DecimalIntrinsicResult FmaVectorized(decimal lhs, decimal rhs, decimal addend) => FmaWithOptions(lhs, rhs,
    addend, DecimalRoundingMode.TiesToEven, DecimalVectorizeHint.Decimal);
    public static DecimalIntrinsicResult FmaWithOptions(decimal lhs, decimal rhs, decimal addend, DecimalRoundingMode rounding,
    DecimalVectorizeHint vectorize) {
        return ExecuteTernary(lhs, rhs, addend, (a, b, c) => a * b + c);
    }
}
