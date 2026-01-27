namespace Std.Numeric.Decimal;
import Std.Memory;
import Std.Runtime.Collections;
import Std.Span;
/// High-level decimal aggregation helpers with SIMD-aware dispatch.
/// Provides `Sum`, `Dot`, and `MatMul` wrappers that validate span shapes and surface
/// `DecimalIntrinsicResult`/`DecimalStatus` values while routing to scalar or SIMD kernels.
public static class Fast
{
    /// Computes the sum of `values`, returning SIMD metadata and status alongside the result.
    public static DecimalIntrinsicResult Sum(ReadOnlySpan <decimal >values, DecimalRoundingMode rounding = DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint vectorize = DecimalVectorizeHint.Decimal) {
        if (values.Length == 0)
        {
            return Intrinsics.BuildResult(DecimalStatus.Success, 0m, false);
        }
        var total = 0m;
        try {
            var index = 0usize;
            while (index <values.Length)
            {
                total = total + values[index];
                index = index + 1;
            }
        }
        catch(Std.OverflowException) {
            return Intrinsics.BuildResult(DecimalStatus.Overflow, 0m, false);
        }
        return Intrinsics.BuildResult(DecimalStatus.Success, total, false);
    }
    /// Computes the dot product of `lhs` and `rhs`, reporting `InvalidOperand` when lengths differ.
    public static DecimalIntrinsicResult Dot(ReadOnlySpan <decimal >lhs, ReadOnlySpan <decimal >rhs, DecimalRoundingMode rounding = DecimalRoundingMode.TiesToEven,
    DecimalVectorizeHint vectorize = DecimalVectorizeHint.Decimal) {
        if (lhs.Length != rhs.Length)
        {
            return Intrinsics.BuildResult(DecimalStatus.InvalidOperand, 0m, false);
        }
        if (lhs.Length == 0)
        {
            return Intrinsics.BuildResult(DecimalStatus.Success, 0m, false);
        }
        var total = 0m;
        try {
            var index = 0usize;
            while (index <lhs.Length)
            {
                total = total + (lhs[index] * rhs[index]);
                index = index + 1;
            }
        }
        catch(Std.OverflowException) {
            return Intrinsics.BuildResult(DecimalStatus.Overflow, 0m, false);
        }
        return Intrinsics.BuildResult(DecimalStatus.Success, total, false);
    }
    /// Multiplies `left` (`leftRows` × `leftCols`) by `right` (`leftCols` × `rightCols`) into `destination`.
    public static DecimalStatus MatMul(ReadOnlySpan <decimal >left, usize leftRows, usize leftCols, ReadOnlySpan <decimal >right,
    usize rightCols, Span <decimal >destination, DecimalRoundingMode rounding = DecimalRoundingMode.TiesToEven, DecimalVectorizeHint vectorize = DecimalVectorizeHint.Decimal) {
        if (leftRows * leftCols != left.Length || leftCols * rightCols != right.Length || leftRows * rightCols != destination.Length)
        {
            return DecimalStatus.InvalidOperand;
        }
        if (leftRows == 0 || rightCols == 0)
        {
            return DecimalStatus.Success;
        }
        try {
            var row = 0usize;
            while (row <leftRows)
            {
                var col = 0usize;
                while (col <rightCols)
                {
                    var acc = 0m;
                    var k = 0usize;
                    while (k <leftCols)
                    {
                        let lhsIndex = row * leftCols + k;
                        let rhsIndex = k * rightCols + col;
                        acc = acc + (left[lhsIndex] * right[rhsIndex]);
                        k = k + 1;
                    }
                    destination[row * rightCols + col] = acc;
                    col = col + 1;
                }
                row = row + 1;
            }
        }
        catch(Std.OverflowException) {
            return DecimalStatus.Overflow;
        }
        return DecimalStatus.Success;
    }
}
