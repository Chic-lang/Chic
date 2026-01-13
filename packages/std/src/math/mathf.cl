namespace Std;
import Std.MathInternal;
/// Floating-point math helpers modeled after System.MathF.
public static class MathF
{
    public const float E = 2.7182817f;
    public const float PI = 3.1415927f;
    public const float Tau = 6.2831855f;
    public static float Abs(float value) => InternalIntrinsics.AbsF32(value);
    public static float Ceiling(float value) => InternalIntrinsics.CeilingF32(value);
    public static float Floor(float value) => InternalIntrinsics.FloorF32(value);
    public static float Truncate(float value) => InternalIntrinsics.TruncateF32(value);
    public static float Clamp(float value, float min, float max) {
        if (IsNaN (value))
        {
            return value;
        }
        if (IsNaN (min) || IsNaN (max))
        {
            return float.NaN;
        }
        if (min >max)
        {
            throw new ArgumentException("min cannot be greater than max");
        }
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
    public static float Max(float left, float right) {
        if (IsNaN (left))
        {
            return left;
        }
        if (IsNaN (right))
        {
            return right;
        }
        if (left == right)
        {
            if (left == 0.0f)
            {
                return 0.0f;
            }
            return left;
        }
        return left >right ?left : right;
    }
    public static float Min(float left, float right) {
        if (IsNaN (left))
        {
            return left;
        }
        if (IsNaN (right))
        {
            return right;
        }
        if (left == right)
        {
            if (left == 0.0f)
            {
                return - 0.0f;
            }
            return left;
        }
        return left <right ?left : right;
    }
    public static int Sign(float value) {
        if (IsNaN (value))
        {
            throw new ArgumentException("value is NaN");
        }
        if (value >0.0f)
        {
            return 1;
        }
        if (value <0.0f)
        {
            return - 1;
        }
        return 0;
    }
    public static float MaxMagnitude(float left, float right) {
        if (IsNaN (left))
        {
            return left;
        }
        if (IsNaN (right))
        {
            return right;
        }
        let absLeft = Abs(left);
        let absRight = Abs(right);
        if (absLeft >absRight)
        {
            return left;
        }
        if (absLeft <absRight)
        {
            return right;
        }
        return Max(left, right);
    }
    public static float MinMagnitude(float left, float right) {
        if (IsNaN (left))
        {
            return left;
        }
        if (IsNaN (right))
        {
            return right;
        }
        let absLeft = Abs(left);
        let absRight = Abs(right);
        if (absLeft <absRight)
        {
            return left;
        }
        if (absLeft >absRight)
        {
            return right;
        }
        return Min(left, right);
    }
    public static float CopySign(float value, float sign) => InternalIntrinsics.CopySignF32(value, sign);
    public static float BitIncrement(float value) => InternalIntrinsics.BitIncrementF32(value);
    public static float BitDecrement(float value) => InternalIntrinsics.BitDecrementF32(value);
    public static float ScaleB(float value, int n) => InternalIntrinsics.ScaleBF32(value, n);
    public static int ILogB(float value) => InternalIntrinsics.ILogBF32(value);
    public static float IEEERemainder(float lhs, float rhs) => InternalIntrinsics.IeeeRemainderF32(lhs, rhs);
    public static float FusedMultiplyAdd(float a, float b, float c) => InternalIntrinsics.FusedMultiplyAddF32(a, b, c);
    public static float Cbrt(float value) => InternalIntrinsics.CbrtF32(value);
    public static float Sqrt(float value) => InternalIntrinsics.SqrtF32(value);
    public static float Pow(float value, float power) => InternalIntrinsics.PowF32(value, power);
    public static float ReciprocalEstimate(float value) => 1.0f / value;
    public static float ReciprocalSqrtEstimate(float value) => 1.0f / Sqrt(value);
    public static float Sin(float value) => InternalIntrinsics.SinF32(value);
    public static float Cos(float value) => InternalIntrinsics.CosF32(value);
    public static float Tan(float value) => InternalIntrinsics.TanF32(value);
    public static float Asin(float value) => InternalIntrinsics.AsinF32(value);
    public static float Acos(float value) => InternalIntrinsics.AcosF32(value);
    public static float Atan(float value) => InternalIntrinsics.AtanF32(value);
    public static float Atan2(float y, float x) => InternalIntrinsics.Atan2F32(y, x);
    public static float Sinh(float value) => InternalIntrinsics.SinhF32(value);
    public static float Cosh(float value) => InternalIntrinsics.CoshF32(value);
    public static float Tanh(float value) => InternalIntrinsics.TanhF32(value);
    public static float Asinh(float value) => InternalIntrinsics.AsinhF32(value);
    public static float Acosh(float value) => InternalIntrinsics.AcoshF32(value);
    public static float Atanh(float value) => InternalIntrinsics.AtanhF32(value);
    public static float Exp(float value) => InternalIntrinsics.ExpF32(value);
    public static float Log(float value) => InternalIntrinsics.LogF32(value);
    public static float Log(float value, float baseValue) {
        if (baseValue == 1.0f)
        {
            return float.NaN;
        }
        return Log(value) / Log(baseValue);
    }
    public static float Log10(float value) => InternalIntrinsics.Log10F32(value);
    public static float Log2(float value) => InternalIntrinsics.Log2F32(value);
    public static void SinCos(float value, out float sin, out float cos) {
        sin = Sin(value);
        cos = Cos(value);
    }
    public static float Round(float value) => Round(value, 0, MidpointRounding.ToEven);
    public static float Round(float value, int digits) => Round(value, digits, MidpointRounding.ToEven);
    public static float Round(float value, MidpointRounding mode) => Round(value, 0, mode);
    public static float Round(float value, int digits, MidpointRounding mode) {
        ValidateRoundingMode(mode);
        return InternalIntrinsics.RoundF32(value, digits, (int) mode);
    }
    private static bool IsNaN(float value) => value != value;
    private static void ValidateRoundingMode(MidpointRounding mode) {
        let raw = (int) mode;
        if (raw <0 || raw >4)
        {
            throw new ArgumentOutOfRangeException("mode");
        }
    }
}
