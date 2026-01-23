namespace Std.Linalg.Quantized;
import Std.Core;
import Std.Numeric;
import Std.Math;
import Std.Runtime;
import Std.Span;
public enum QuantizedRoundingMode
{
    NearestEven = 0, TowardZero = 1,
}
public struct QuantizationPolicy
{
    public float[] Scales;
    public int[] ZeroPoints;
    public uint Bits;
    public bool Signed;
    public bool Saturate;
    public QuantizedRoundingMode Rounding;
    public bool PerChannel {
        get {
            let scaleCount = Scales == null ?0 : Scales.Length;
            let zeroCount = ZeroPoints == null ?0 : ZeroPoints.Length;
            return scaleCount >1 || zeroCount >1;
        }
    }
    public static QuantizationPolicy PerTensor(float scale, int zeroPoint, uint bits = 8u, bool signed = true, bool saturate = true,
    QuantizedRoundingMode rounding = QuantizedRoundingMode.NearestEven) {
        var policy = CoreIntrinsics.DefaultValue <QuantizationPolicy >();
        policy.Scales = new float[] {
            scale
        }
        ;
        policy.ZeroPoints = new int[] {
            zeroPoint
        }
        ;
        policy.Bits = bits;
        policy.Signed = signed;
        policy.Saturate = saturate;
        policy.Rounding = rounding;
        policy.Validate();
        return policy;
    }
    internal void Validate() {
        if (Bits == 0u)
        {
            throw new Std.ArgumentOutOfRangeException("bits", "quantization bits must be at least 1");
        }
        if (Signed && Bits >31u)
        {
            throw new Std.ArgumentOutOfRangeException("bits", "signed quantization supports up to 31 bits");
        }
        if (!Signed && Bits >32u)
        {
            throw new Std.ArgumentOutOfRangeException("bits", "unsigned quantization supports up to 32 bits");
        }
        if (Scales != null)
        {
            var i = 0;
            while (i <Scales.Length)
            {
                if (Scales[i] == 0.0f)
                {
                    throw new Std.ArgumentException("quantization scale must be non-zero");
                }
                i += 1;
            }
        }
    }
    public float ScaleFor(usize channel) {
        let count = Scales == null ?0usize : NumericUnchecked.ToUSize(Scales.Length);
        if (count == 0usize)
        {
            return 1.0f;
        }
        let idx = NumericUnchecked.ToInt32(channel % count);
        return Scales[idx];
    }
    public int ZeroPointFor(usize channel) {
        let count = ZeroPoints == null ?0usize : NumericUnchecked.ToUSize(ZeroPoints.Length);
        if (count == 0usize)
        {
            return 0;
        }
        let idx = NumericUnchecked.ToInt32(channel % count);
        return ZeroPoints[idx];
    }
}
public static class Kernels
{
    public static int QuantizeScalar(float value, QuantizationPolicy policy, usize channel = 0usize) {
        policy.Validate();
        return QuantizeScalarUnchecked(value, policy, channel);
    }
    public static float DequantizeScalar(int value, QuantizationPolicy policy, usize channel = 0usize) {
        policy.Validate();
        return DequantizeScalarUnchecked(value, policy, channel);
    }
    private static int QuantizeScalarUnchecked(float value, QuantizationPolicy policy, usize channel) {
        let scale = policy.ScaleFor(channel);
        let zero = policy.ZeroPointFor(channel);
        let scaled = (double) value / (double) scale + (double) zero;
        let rounded = RoundValue(scaled, policy.Rounding);
        let quant = ClampToRange((long) rounded, policy);
        return NarrowToInt32(quant);
    }
    private static float DequantizeScalarUnchecked(int value, QuantizationPolicy policy, usize channel) {
        let scale = policy.ScaleFor(channel);
        let zero = policy.ZeroPointFor(channel);
        return(value - zero) * scale;
    }
    public static void Requantize(ReadOnlySpan <int >source, QuantizationPolicy sourcePolicy, QuantizationPolicy destinationPolicy,
    Span <int >destination) {
        sourcePolicy.Validate();
        destinationPolicy.Validate();
        EnsureSameLength(source.Length, destination.Length, "destination");
        var i = 0usize;
        while (i <destination.Length)
        {
            let real = DequantizeScalarUnchecked(source[i], sourcePolicy, i);
            destination[i] = QuantizeScalarUnchecked(real, destinationPolicy, i);
            i += 1usize;
        }
    }
    public static void Add(ReadOnlySpan <int >left, ReadOnlySpan <int >right, QuantizationPolicy inputPolicy, QuantizationPolicy outputPolicy,
    Span <int >destination) {
        inputPolicy.Validate();
        outputPolicy.Validate();
        EnsureSameLength(left.Length, right.Length, "right");
        EnsureSameLength(left.Length, destination.Length, "destination");
        var i = 0usize;
        while (i <destination.Length)
        {
            let lhs = DequantizeScalarUnchecked(left[i], inputPolicy, i);
            let rhs = DequantizeScalarUnchecked(right[i], inputPolicy, i);
            destination[i] = QuantizeScalarUnchecked(lhs + rhs, outputPolicy, i);
            i += 1usize;
        }
    }
    public static void Multiply(ReadOnlySpan <int >left, ReadOnlySpan <int >right, QuantizationPolicy inputPolicy, QuantizationPolicy outputPolicy,
    Span <int >destination) {
        inputPolicy.Validate();
        outputPolicy.Validate();
        EnsureSameLength(left.Length, right.Length, "right");
        EnsureSameLength(left.Length, destination.Length, "destination");
        var i = 0usize;
        while (i <destination.Length)
        {
            let lhs = DequantizeScalarUnchecked(left[i], inputPolicy, i);
            let rhs = DequantizeScalarUnchecked(right[i], inputPolicy, i);
            destination[i] = QuantizeScalarUnchecked(lhs * rhs, outputPolicy, i);
            i += 1usize;
        }
    }
    public static int Dot(ReadOnlySpan <int >left, ReadOnlySpan <int >right, QuantizationPolicy leftPolicy, QuantizationPolicy rightPolicy,
    QuantizationPolicy outputPolicy) {
        leftPolicy.Validate();
        rightPolicy.Validate();
        outputPolicy.Validate();
        let count = left.Length <right.Length ?left.Length : right.Length;
        var acc = 0.0;
        var i = 0usize;
        while (i <count)
        {
            let lhs = DequantizeScalarUnchecked(left[i], leftPolicy, i);
            let rhs = DequantizeScalarUnchecked(right[i], rightPolicy, i);
            acc += (double) lhs * (double) rhs;
            i += 1usize;
        }
        return QuantizeScalarUnchecked((float) acc, outputPolicy, 0usize);
    }
    public static void Gemm(ReadOnlySpan <int >left, ReadOnlySpan <int >right, usize m, usize n, usize k, QuantizationPolicy leftPolicy,
    QuantizationPolicy rightPolicy, QuantizationPolicy outputPolicy, Span <int >destination) {
        leftPolicy.Validate();
        rightPolicy.Validate();
        outputPolicy.Validate();
        if (left.Length != m * k)
        {
            throw new Std.ArgumentException("left matrix length does not match dimensions");
        }
        if (right.Length != k * n)
        {
            throw new Std.ArgumentException("right matrix length does not match dimensions");
        }
        if (destination.Length != m * n)
        {
            throw new Std.ArgumentException("destination length does not match output dimensions");
        }
        var row = 0usize;
        while (row <m)
        {
            var col = 0usize;
            while (col <n)
            {
                var acc = 0.0;
                var kk = 0usize;
                while (kk <k)
                {
                    let lhsIndex = row * k + kk;
                    let rhsIndex = kk * n + col;
                    let lhs = DequantizeScalarUnchecked(left[lhsIndex], leftPolicy, lhsIndex);
                    let rhs = DequantizeScalarUnchecked(right[rhsIndex], rightPolicy, rhsIndex);
                    acc += (double) lhs * (double) rhs;
                    kk += 1usize;
                }
                let destIndex = row * n + col;
                destination[destIndex] = QuantizeScalarUnchecked((float) acc, outputPolicy, col);
                col += 1usize;
            }
            row += 1usize;
        }
    }
    private static long ClampToRange(long value, QuantizationPolicy policy) {
        if (!policy.Saturate)
        {
            return value;
        }
        if (policy.Signed)
        {
            let bits = NumericUnchecked.ToInt32(policy.Bits - 1u);
            let max = (1L << bits) - 1L;
            let min = - (1L << bits);
            if (value <min)
            {
                return min;
            }
            if (value >max)
            {
                return max;
            }
            return value;
        }
        else
        {
            let bits = NumericUnchecked.ToInt32(policy.Bits);
            let max = (1L << bits) - 1L;
            if (value <0L)
            {
                return 0L;
            }
            if (value >max)
            {
                return max;
            }
            return value;
        }
    }
    private static double RoundValue(double value, QuantizedRoundingMode mode) {
        switch (mode)
        {
            case QuantizedRoundingMode.NearestEven:
                let floor = Math.Floor(value);
                let frac = value - floor;
                if (Math.Abs (frac - 0.5) >double.Epsilon)
                {
                    return Math.Round(value);
                }
                if ( ( (long) floor) % 2L == 0L)
                {
                    return floor;
                }
                return floor + Math.Sign(value);
            case QuantizedRoundingMode.TowardZero:
                return Math.Truncate(value);
            default :
                return value;
            }
        }
        private static void EnsureSameLength(usize left, usize right, string name) {
            if (left != right)
            {
                throw new Std.ArgumentException(Std.Runtime.StringRuntime.FromStr("span length mismatch for ") + name);
            }
        }
        private static int NarrowToInt32(long value) {
            if (value >Std.Numeric.Int32.MaxValue)
            {
                return Std.Numeric.Int32.MaxValue;
            }
            if (value <Std.Numeric.Int32.MinValue)
            {
                return Std.Numeric.Int32.MinValue;
            }
            return NumericUnchecked.ToInt32(value);
        }
        }
