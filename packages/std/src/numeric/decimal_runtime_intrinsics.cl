namespace Std.Numeric.Decimal;
/// Entry points that bridge Chic decimal helpers to the Rust runtime intrinsics.
/// These signatures mirror the ABI registered in `codegen/llvm/signatures.rs`
/// and are surfaced through the generic math wrappers in `decimal_fast_algorithms.cl`.
internal static class RuntimeIntrinsics
{
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_sum(DecimalConstPtr values, usize length,
    DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_dot(DecimalConstPtr lhs, DecimalConstPtr rhs,
    usize length, DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_add(DecimalConstPtr lhs, DecimalConstPtr rhs,
    DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_sub(DecimalConstPtr lhs, DecimalConstPtr rhs,
    DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_mul(DecimalConstPtr lhs, DecimalConstPtr rhs,
    DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_div(DecimalConstPtr lhs, DecimalConstPtr rhs,
    DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_rem(DecimalConstPtr lhs, DecimalConstPtr rhs,
    DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalRuntimeCall chic_rt_decimal_fma(DecimalConstPtr lhs, DecimalConstPtr rhs,
    DecimalConstPtr addend, DecimalRoundingEncoding rounding, uint flags);
    @extern("C") public static extern DecimalStatus chic_rt_decimal_clone(DecimalConstPtr source, DecimalMutPtr destination);
    @extern("C") public static extern DecimalStatus chic_rt_decimal_matmul(DecimalConstPtr left, usize leftRows, usize leftCols,
    DecimalConstPtr right, usize rightCols, DecimalMutPtr destination, DecimalRoundingEncoding rounding, uint flags);
}
