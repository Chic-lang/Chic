namespace Std.MathInternal;
/// Chic-native math intrinsics backed directly by platform libm/LLVM lowering.
/// All language-visible math semantics must route through these helpers so no
/// bootstrap shim math symbols remain.
internal static class InternalIntrinsics
{
    @extern("C") private static extern double fabs(double value);
    @extern("C") private static extern float fabsf(float value);
    @extern("C") private static extern double floor(double value);
    @extern("C") private static extern float floorf(float value);
    @extern("C") private static extern double ceil(double value);
    @extern("C") private static extern float ceilf(float value);
    @extern("C") private static extern double trunc(double value);
    @extern("C") private static extern float truncf(float value);
    @extern("C") private static extern double copysign(double value, double sign);
    @extern("C") private static extern float copysignf(float value, float sign);
    @extern("C") private static extern double scalbn(double value, int n);
    @extern("C") private static extern float scalbnf(float value, int n);
    @extern("C") private static extern double fma(double a, double b, double c);
    @extern("C") private static extern float fmaf(float a, float b, float c);
    @extern("C") private static extern double cbrt(double value);
    @extern("C") private static extern float cbrtf(float value);
    @extern("C") private static extern double sqrt(double value);
    @extern("C") private static extern float sqrtf(float value);
    @extern("C") private static extern double pow(double value, double power);
    @extern("C") private static extern float powf(float value, float power);
    @extern("C") private static extern double sin(double value);
    @extern("C") private static extern float sinf(float value);
    @extern("C") private static extern double cos(double value);
    @extern("C") private static extern float cosf(float value);
    @extern("C") private static extern double tan(double value);
    @extern("C") private static extern float tanf(float value);
    @extern("C") private static extern double asin(double value);
    @extern("C") private static extern float asinf(float value);
    @extern("C") private static extern double acos(double value);
    @extern("C") private static extern float acosf(float value);
    @extern("C") private static extern double atan(double value);
    @extern("C") private static extern float atanf(float value);
    @extern("C") private static extern double atan2(double y, double x);
    @extern("C") private static extern float atan2f(float y, float x);
    @extern("C") private static extern double sinh(double value);
    @extern("C") private static extern float sinhf(float value);
    @extern("C") private static extern double cosh(double value);
    @extern("C") private static extern float coshf(float value);
    @extern("C") private static extern double tanh(double value);
    @extern("C") private static extern float tanhf(float value);
    @extern("C") private static extern double asinh(double value);
    @extern("C") private static extern float asinhf(float value);
    @extern("C") private static extern double acosh(double value);
    @extern("C") private static extern float acoshf(float value);
    @extern("C") private static extern double atanh(double value);
    @extern("C") private static extern float atanhf(float value);
    @extern("C") private static extern double exp(double value);
    @extern("C") private static extern float expf(float value);
    @extern("C") private static extern double log(double value);
    @extern("C") private static extern float logf(float value);
    @extern("C") private static extern double log10(double value);
    @extern("C") private static extern float log10f(float value);
    @extern("C") private static extern double log2(double value);
    @extern("C") private static extern float log2f(float value);
    @extern("C") private static extern double round(double value);
    @extern("C") private static extern float roundf(float value);
    @extern("C") private static extern double nextafter(double value, double direction);
    @extern("C") private static extern float nextafterf(float value, float direction);
    private static double PositiveInfinity64() {
        let zero = 0.0d;
        return 1.0d / zero;
    }
    private static double NegativeInfinity64() {
        let zero = 0.0d;
        return - 1.0d / zero;
    }
    private static float PositiveInfinity32() {
        let zero = 0.0f;
        return 1.0f / zero;
    }
    private static float NegativeInfinity32() {
        let zero = 0.0f;
        return - 1.0f / zero;
    }
    private static bool IsNaNF64(double value) => value != value;
    private static bool IsNaNF32(float value) => value != value;
    private static bool IsPosInfF64(double value) => value == PositiveInfinity64();
    private static bool IsNegInfF64(double value) => value == NegativeInfinity64();
    private static bool IsPosInfF32(float value) => value == PositiveInfinity32();
    private static bool IsNegInfF32(float value) => value == NegativeInfinity32();
    private static bool IsFiniteF64(double value) => !IsNaNF64(value) && !IsPosInfF64(value) && !IsNegInfF64(value);
    private static bool IsFiniteF32(float value) => !IsNaNF32(value) && !IsPosInfF32(value) && !IsNegInfF32(value);
    private static double RoundToEvenF64(double value) {
        if (!IsFiniteF64 (value))
        {
            return value;
        }
        let truncated = trunc(value);
        let frac = value - truncated;
        if (fabs (frac) != 0.5d)
        {
            return round(value);
        }
        if (fabs (truncated) >9007199254740992.0d)
        {
            return value;
        }
        let truncatedInt = (long) truncated;
        if ( (truncatedInt % 2) == 0)
        {
            return truncated;
        }
        return truncated + copysign(1.0d, value);
    }
    private static float RoundToEvenF32(float value) {
        if (!IsFiniteF32 (value))
        {
            return value;
        }
        let truncated = truncf(value);
        let frac = value - truncated;
        if (fabsf (frac) != 0.5f)
        {
            return roundf(value);
        }
        if (fabsf (truncated) >= 16777216.0f)
        {
            return value;
        }
        let truncatedInt = (int) truncated;
        if ( (truncatedInt % 2) == 0)
        {
            return truncated;
        }
        return truncated + copysignf(1.0f, value);
    }
    private static double BitIncrement64(double value) {
        if (IsNaNF64 (value) || IsPosInfF64 (value))
        {
            return value;
        }
        if (IsNegInfF64 (value))
        {
            return nextafter(NegativeInfinity64(), 0.0d);
        }
        return nextafter(value, value >= 0.0d ?PositiveInfinity64() : NegativeInfinity64());
    }
    private static double BitDecrement64(double value) {
        if (IsNaNF64 (value) || IsNegInfF64 (value))
        {
            return value;
        }
        if (IsPosInfF64 (value))
        {
            return nextafter(PositiveInfinity64(), 0.0d);
        }
        return nextafter(value, value >= 0.0d ?NegativeInfinity64() : PositiveInfinity64());
    }
    private static float BitIncrement32(float value) {
        if (IsNaNF32 (value) || IsPosInfF32 (value))
        {
            return value;
        }
        if (IsNegInfF32 (value))
        {
            return nextafterf(NegativeInfinity32(), 0.0f);
        }
        return nextafterf(value, value >= 0.0f ?PositiveInfinity32() : NegativeInfinity32());
    }
    private static float BitDecrement32(float value) {
        if (IsNaNF32 (value) || IsNegInfF32 (value))
        {
            return value;
        }
        if (IsPosInfF32 (value))
        {
            return nextafterf(PositiveInfinity32(), 0.0f);
        }
        return nextafterf(value, value >= 0.0f ?NegativeInfinity32() : PositiveInfinity32());
    }
    private static int ILogB64(double value) {
        if (value == 0.0d || IsNaNF64 (value))
        {
            return int.MinValue;
        }
        if (IsPosInfF64 (value) || IsNegInfF64 (value))
        {
            return int.MaxValue;
        }
        return(int) floor(log2(fabs(value)));
    }
    private static int ILogB32(float value) {
        if (value == 0.0f || IsNaNF32 (value))
        {
            return int.MinValue;
        }
        if (IsPosInfF32 (value) || IsNegInfF32 (value))
        {
            return int.MaxValue;
        }
        return(int) floorf(log2f(fabsf(value)));
    }
    private static double IeeeRemainder64(double lhs, double rhs) {
        if (IsNaNF64 (lhs))
        {
            return lhs;
        }
        if (IsNaNF64 (rhs))
        {
            return rhs;
        }
        if (rhs == 0.0d)
        {
            return 0.0d / 0.0d;
        }
        let quotient = RoundToEvenF64(lhs / rhs);
        return lhs - quotient * rhs;
    }
    private static float IeeeRemainder32(float lhs, float rhs) {
        if (IsNaNF32 (lhs))
        {
            return lhs;
        }
        if (IsNaNF32 (rhs))
        {
            return rhs;
        }
        if (rhs == 0.0f)
        {
            return 0.0f / 0.0f;
        }
        let quotient = RoundToEvenF32(lhs / rhs);
        return lhs - quotient * rhs;
    }
    private static double Round64(double value, int digits, int mode) {
        if (!IsFiniteF64 (value))
        {
            return value;
        }
        let clamped = digits <- 308 ?- 308 : (digits >308 ?308 : digits);
        let scale = pow(10.0d, (double) clamped);
        if (scale == 0.0d || IsPosInfF64 (scale) || IsNegInfF64 (scale))
        {
            return value;
        }
        let scaled = value * scale;
        var rounded = scaled;
        if (mode == 0)
        {
            rounded = RoundToEvenF64(scaled);
        }
        else if (mode == 1)
        {
            rounded = round(scaled);
        }
        else if (mode == 2)
        {
            rounded = trunc(scaled);
        }
        else if (mode == 3)
        {
            rounded = floor(scaled);
        }
        else if (mode == 4)
        {
            rounded = ceil(scaled);
        }
        else
        {
            rounded = RoundToEvenF64(scaled);
        }
        return rounded / scale;
    }
    private static float Round32(float value, int digits, int mode) {
        if (!IsFiniteF32 (value))
        {
            return value;
        }
        let clamped = digits <- 45 ?- 45 : (digits >45 ?45 : digits);
        let scale = powf(10.0f, (float) clamped);
        if (scale == 0.0f || IsPosInfF32 (scale) || IsNegInfF32 (scale))
        {
            return value;
        }
        let scaled = value * scale;
        var rounded = scaled;
        if (mode == 0)
        {
            rounded = RoundToEvenF32(scaled);
        }
        else if (mode == 1)
        {
            rounded = roundf(scaled);
        }
        else if (mode == 2)
        {
            rounded = truncf(scaled);
        }
        else if (mode == 3)
        {
            rounded = floorf(scaled);
        }
        else if (mode == 4)
        {
            rounded = ceilf(scaled);
        }
        else
        {
            rounded = RoundToEvenF32(scaled);
        }
        return rounded / scale;
    }
    public static double AbsF64(double value) {
        return fabs(value);
    }
    public static double CeilingF64(double value) {
        return ceil(value);
    }
    public static double FloorF64(double value) {
        return floor(value);
    }
    public static double TruncateF64(double value) {
        return trunc(value);
    }
    public static double CopySignF64(double value, double sign) {
        return copysign(value, sign);
    }
    public static double BitIncrementF64(double value) {
        return BitIncrement64(value);
    }
    public static double BitDecrementF64(double value) {
        return BitDecrement64(value);
    }
    public static double ScaleBF64(double value, int n) {
        return scalbn(value, n);
    }
    public static int ILogBF64(double value) {
        return ILogB64(value);
    }
    public static double IeeeRemainderF64(double lhs, double rhs) {
        return IeeeRemainder64(lhs, rhs);
    }
    public static double FusedMultiplyAddF64(double a, double b, double c) {
        return fma(a, b, c);
    }
    public static double CbrtF64(double value) {
        return cbrt(value);
    }
    public static double SqrtF64(double value) {
        return sqrt(value);
    }
    public static double PowF64(double value, double power) {
        return pow(value, power);
    }
    public static double SinF64(double value) {
        return sin(value);
    }
    public static double CosF64(double value) {
        return cos(value);
    }
    public static double TanF64(double value) {
        return tan(value);
    }
    public static double AsinF64(double value) {
        return asin(value);
    }
    public static double AcosF64(double value) {
        return acos(value);
    }
    public static double AtanF64(double value) {
        return atan(value);
    }
    public static double Atan2F64(double y, double x) {
        return atan2(y, x);
    }
    public static double SinhF64(double value) {
        return sinh(value);
    }
    public static double CoshF64(double value) {
        return cosh(value);
    }
    public static double TanhF64(double value) {
        return tanh(value);
    }
    public static double AsinhF64(double value) {
        return asinh(value);
    }
    public static double AcoshF64(double value) {
        return acosh(value);
    }
    public static double AtanhF64(double value) {
        return atanh(value);
    }
    public static double ExpF64(double value) {
        return exp(value);
    }
    public static double LogF64(double value) {
        return log(value);
    }
    public static double Log10F64(double value) {
        return log10(value);
    }
    public static double Log2F64(double value) {
        return log2(value);
    }
    public static double RoundF64(double value, int digits, int mode) {
        return Round64(value, digits, mode);
    }
    public static float AbsF32(float value) {
        return fabsf(value);
    }
    public static float CeilingF32(float value) {
        return ceilf(value);
    }
    public static float FloorF32(float value) {
        return floorf(value);
    }
    public static float TruncateF32(float value) {
        return truncf(value);
    }
    public static float CopySignF32(float value, float sign) {
        return copysignf(value, sign);
    }
    public static float BitIncrementF32(float value) {
        return BitIncrement32(value);
    }
    public static float BitDecrementF32(float value) {
        return BitDecrement32(value);
    }
    public static float ScaleBF32(float value, int n) {
        return scalbnf(value, n);
    }
    public static int ILogBF32(float value) {
        return ILogB32(value);
    }
    public static float IeeeRemainderF32(float lhs, float rhs) {
        return IeeeRemainder32(lhs, rhs);
    }
    public static float FusedMultiplyAddF32(float a, float b, float c) {
        return fmaf(a, b, c);
    }
    public static float CbrtF32(float value) {
        return cbrtf(value);
    }
    public static float SqrtF32(float value) {
        return sqrtf(value);
    }
    public static float PowF32(float value, float power) {
        return powf(value, power);
    }
    public static float SinF32(float value) {
        return sinf(value);
    }
    public static float CosF32(float value) {
        return cosf(value);
    }
    public static float TanF32(float value) {
        return tanf(value);
    }
    public static float AsinF32(float value) {
        return asinf(value);
    }
    public static float AcosF32(float value) {
        return acosf(value);
    }
    public static float AtanF32(float value) {
        return atanf(value);
    }
    public static float Atan2F32(float y, float x) {
        return atan2f(y, x);
    }
    public static float SinhF32(float value) {
        return sinhf(value);
    }
    public static float CoshF32(float value) {
        return coshf(value);
    }
    public static float TanhF32(float value) {
        return tanhf(value);
    }
    public static float AsinhF32(float value) {
        return asinhf(value);
    }
    public static float AcoshF32(float value) {
        return acoshf(value);
    }
    public static float AtanhF32(float value) {
        return atanhf(value);
    }
    public static float ExpF32(float value) {
        return expf(value);
    }
    public static float LogF32(float value) {
        return logf(value);
    }
    public static float Log10F32(float value) {
        return log10f(value);
    }
    public static float Log2F32(float value) {
        return log2f(value);
    }
    public static float RoundF32(float value, int digits, int mode) {
        return Round32(value, digits, mode);
    }
}
