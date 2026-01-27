namespace Tests.Linalg.Quantized;

import Std.Linalg.Quantized;
import Std.Span;

public static class Program
{
    public static int Main()
    {
        QuantizeRounding();
        SaturationClamps();
        Elementwise();
        DotAndGemm();
        Requantize();
        PerChannel();
        return 0;
    }

    private static QuantizationPolicy DefaultPolicy()
    {
        return QuantizationPolicy.PerTensor(
            0.5f,
            0,
            8u,
            true,
            true,
            QuantizedRoundingMode.NearestEven
        );
    }

    private static void QuantizeRounding()
    {
        let policy = DefaultPolicy();
        let q1 = Kernels.QuantizeScalar(1.0f, policy);
        if (q1 != 2)
        {
            throw new Std.InvalidOperationException("quantize 1.0 should be 2");
        }
        let tieUp = Kernels.QuantizeScalar(1.75f, policy);
        if (tieUp != 4)
        {
            throw new Std.InvalidOperationException("nearest-even tie should round up");
        }
        var towardZero = policy;
        towardZero.Rounding = QuantizedRoundingMode.TowardZero;
        let tz = Kernels.QuantizeScalar(1.75f, towardZero);
        if (tz != 3)
        {
            throw new Std.InvalidOperationException("toward-zero rounding failed");
        }
    }

    private static void SaturationClamps()
    {
        var policy = QuantizationPolicy.PerTensor(1.0f, 0, 8u, true, true);
        let maxed = Kernels.QuantizeScalar(200.0f, policy);
        if (maxed != 127)
        {
            throw new Std.InvalidOperationException("saturation did not clamp signed 8-bit");
        }
        var unsig = QuantizationPolicy.PerTensor(1.0f, 0, 8u, false, true);
        let under = Kernels.QuantizeScalar(-4.0f, unsig);
        if (under != 0)
        {
            throw new Std.InvalidOperationException("unsigned saturation should clamp to zero");
        }
    }

    private static void Elementwise()
    {
        let policy = DefaultPolicy();
        var leftArr = new int[] { 2, 4 };
        var rightArr = new int[] { 6, 2 };
        var addDest = new int[] { 0, 0 };
        var mulDest = new int[] { 0, 0 };
        Kernels.Add(
            ReadOnlySpan<int>.FromArray(ref leftArr),
            ReadOnlySpan<int>.FromArray(ref rightArr),
            policy,
            policy,
            Span<int>.FromArray(ref addDest)
        );
        Kernels.Multiply(
            ReadOnlySpan<int>.FromArray(ref leftArr),
            ReadOnlySpan<int>.FromArray(ref rightArr),
            policy,
            policy,
            Span<int>.FromArray(ref mulDest)
        );
        if (addDest[0] != 8 || addDest[1] != 6)
        {
            throw new Std.InvalidOperationException("quantized add mismatch");
        }
        if (mulDest[0] != 2 || mulDest[1] != 4)
        {
            throw new Std.InvalidOperationException("quantized multiply mismatch");
        }
    }

    private static void DotAndGemm()
    {
        let policy = DefaultPolicy();
        var leftArr = new int[] { 2, 4 };
        var rightArr = new int[] { 6, 2 };
        let dot = Kernels.Dot(
            ReadOnlySpan<int>.FromArray(ref leftArr),
            ReadOnlySpan<int>.FromArray(ref rightArr),
            policy,
            policy,
            policy
        );
        if (dot != 10)
        {
            throw new Std.InvalidOperationException("quantized dot mismatch");
        }

        // left: 2x2, right: 2x2, output: 2x2
        var leftMat = new int[] { 2, 4, 0, 2 };
        var rightMat = new int[] { 4, 0, 2, 2 };
        var dest = new int[] { 0, 0, 0, 0 };
        Kernels.Gemm(
            ReadOnlySpan<int>.FromArray(ref leftMat),
            ReadOnlySpan<int>.FromArray(ref rightMat),
            2usize,
            2usize,
            2usize,
            policy,
            policy,
            policy,
            Span<int>.FromArray(ref dest)
        );
        if (dest[0] != 8 || dest[1] != 4 || dest[2] != 2 || dest[3] != 2)
        {
            throw new Std.InvalidOperationException("quantized gemm mismatch");
        }
    }

    private static void Requantize()
    {
        let sourcePolicy = DefaultPolicy();
        var destPolicy = QuantizationPolicy.PerTensor(1.0f, 0, 8u, true, true);
        var source = new int[] { 2, 4 };
        var dest = new int[] { 0, 0 };
        Kernels.Requantize(
            ReadOnlySpan<int>.FromArray(ref source),
            sourcePolicy,
            destPolicy,
            Span<int>.FromArray(ref dest)
        );
        if (dest[0] != 1 || dest[1] != 2)
        {
            throw new Std.InvalidOperationException("requantize mismatch");
        }
    }

    private static void PerChannel()
    {
        var policy = new QuantizationPolicy
        {
            Scales = new float[] { 0.5f, 1.0f },
            ZeroPoints = new int[] { 0, 1 },
            Bits = 8u,
            Signed = true,
            Saturate = true,
            Rounding = QuantizedRoundingMode.NearestEven
        };
        let q0 = Kernels.QuantizeScalar(1.0f, policy, 0usize);
        let q1 = Kernels.QuantizeScalar(1.0f, policy, 1usize);
        if (q0 != 2 || q1 != 2)
        {
            throw new Std.InvalidOperationException("per-channel quantization failed");
        }
        let d0 = Kernels.DequantizeScalar(q0, policy, 0usize);
        let d1 = Kernels.DequantizeScalar(q1, policy, 1usize);
        if (d0 != 1.0f || d1 != 1.0f)
        {
            throw new Std.InvalidOperationException("per-channel dequantize failed");
        }
    }
}
