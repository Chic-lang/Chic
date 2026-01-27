namespace Std;
import Std.MathInternal;
import Std.Numeric;
/// Floating-point and integral math helpers modeled after System.Math.
public static class Math
{
    public const double E = 2.71828182845904523536d;
    public const double PI = 3.14159265358979323846d;
    public const double Tau = 6.28318530717958647692d;
    public static sbyte Abs(sbyte value) => SByte.Abs(value);
    public static short Abs(short value) => Int16.Abs(value);
    public static int Abs(int value) => Int32.Abs(value);
    public static long Abs(long value) => Int64.Abs(value);
    public static nint Abs(nint value) => IntPtr.Abs(value);
    public static decimal Abs(decimal value) => Decimal.Abs(value);
    public static double Abs(double value) => InternalIntrinsics.AbsF64(value);
    public static double Ceiling(double value) => InternalIntrinsics.CeilingF64(value);
    public static double Floor(double value) => InternalIntrinsics.FloorF64(value);
    public static double Truncate(double value) => InternalIntrinsics.TruncateF64(value);
    public static decimal Ceiling(decimal value) => ConvertibleHelpers.ToDecimalFromFloat64(Ceiling(ConvertibleHelpers.ToFloat64FromDecimal(value)));
    public static decimal Floor(decimal value) => ConvertibleHelpers.ToDecimalFromFloat64(Floor(ConvertibleHelpers.ToFloat64FromDecimal(value)));
    public static decimal Truncate(decimal value) => ConvertibleHelpers.ToDecimalFromFloat64(Truncate(ConvertibleHelpers.ToFloat64FromDecimal(value)));
    public static sbyte Clamp(sbyte value, sbyte min, sbyte max) {
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
    public static byte Clamp(byte value, byte min, byte max) {
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
    public static short Clamp(short value, short min, short max) {
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
    public static ushort Clamp(ushort value, ushort min, ushort max) {
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
    public static int Clamp(int value, int min, int max) {
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
    public static uint Clamp(uint value, uint min, uint max) {
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
    public static long Clamp(long value, long min, long max) {
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
    public static ulong Clamp(ulong value, ulong min, ulong max) {
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
    public static nint Clamp(nint value, nint min, nint max) {
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
    public static nuint Clamp(nuint value, nuint min, nuint max) {
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
    public static decimal Clamp(decimal value, decimal min, decimal max) {
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
    public static double Clamp(double value, double min, double max) {
        if (IsNaN (value))
        {
            return value;
        }
        if (IsNaN (min) || IsNaN (max))
        {
            return double.NaN;
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
    public static sbyte Max(sbyte left, sbyte right) => left >= right ?left : right;
    public static byte Max(byte left, byte right) => left >= right ?left : right;
    public static short Max(short left, short right) => left >= right ?left : right;
    public static ushort Max(ushort left, ushort right) => left >= right ?left : right;
    public static int Max(int left, int right) => left >= right ?left : right;
    public static uint Max(uint left, uint right) => left >= right ?left : right;
    public static long Max(long left, long right) => left >= right ?left : right;
    public static ulong Max(ulong left, ulong right) => left >= right ?left : right;
    public static nint Max(nint left, nint right) => left >= right ?left : right;
    public static nuint Max(nuint left, nuint right) => left >= right ?left : right;
    public static decimal Max(decimal left, decimal right) => Decimal.Max(left, right);
    public static double Max(double left, double right) {
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
            if (left == 0.0d)
            {
                return 0.0d;
            }
            return left;
        }
        return left >right ?left : right;
    }
    public static sbyte Min(sbyte left, sbyte right) => left <= right ?left : right;
    public static byte Min(byte left, byte right) => left <= right ?left : right;
    public static short Min(short left, short right) => left <= right ?left : right;
    public static ushort Min(ushort left, ushort right) => left <= right ?left : right;
    public static int Min(int left, int right) => left <= right ?left : right;
    public static uint Min(uint left, uint right) => left <= right ?left : right;
    public static long Min(long left, long right) => left <= right ?left : right;
    public static ulong Min(ulong left, ulong right) => left <= right ?left : right;
    public static nint Min(nint left, nint right) => left <= right ?left : right;
    public static nuint Min(nuint left, nuint right) => left <= right ?left : right;
    public static decimal Min(decimal left, decimal right) => Decimal.Min(left, right);
    public static double Min(double left, double right) {
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
            if (left == 0.0d)
            {
                return - 0.0d;
            }
            return left;
        }
        return left <right ?left : right;
    }
    public static int Sign(sbyte value) => value <0 ?- 1 : (value >0 ?1 : 0);
    public static int Sign(short value) => value <0 ?- 1 : (value >0 ?1 : 0);
    public static int Sign(int value) => value <0 ?- 1 : (value >0 ?1 : 0);
    public static int Sign(long value) => value <0 ?- 1 : (value >0 ?1 : 0);
    public static int Sign(decimal value) => value <0m ?- 1 : (value >0m ?1 : 0);
    public static int Sign(float value) => MathF.Sign(value);
    public static int Sign(double value) {
        if (IsNaN (value))
        {
            throw new ArgumentException("value is NaN");
        }
        if (value >0.0d)
        {
            return 1;
        }
        if (value <0.0d)
        {
            return - 1;
        }
        return 0;
    }
    public static long BigMul(int x, int y) => (long) x * (long) y;
    public static ulong BigMul(uint x, uint y) => (ulong) x * (ulong) y;
    public static long BigMul(long x, long y, out long low) {
        let product = (int128) x * (int128) y;
        low = (long) product;
        return(long)(product >> 64);
    }
    public static ulong BigMul(ulong x, ulong y, out ulong low) {
        let product = (u128) x * (u128) y;
        low = (ulong) product;
        return(ulong)(product >> 64);
    }
    public static double MaxMagnitude(double left, double right) {
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
    public static double MinMagnitude(double left, double right) {
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
    public static double CopySign(double value, double sign) => InternalIntrinsics.CopySignF64(value, sign);
    public static double BitIncrement(double value) => InternalIntrinsics.BitIncrementF64(value);
    public static double BitDecrement(double value) => InternalIntrinsics.BitDecrementF64(value);
    public static double ScaleB(double value, int n) => InternalIntrinsics.ScaleBF64(value, n);
    public static int ILogB(double value) => InternalIntrinsics.ILogBF64(value);
    public static double IEEERemainder(double lhs, double rhs) => InternalIntrinsics.IeeeRemainderF64(lhs, rhs);
    public static double FusedMultiplyAdd(double a, double b, double c) => InternalIntrinsics.FusedMultiplyAddF64(a, b, c);
    public static double Cbrt(double value) => InternalIntrinsics.CbrtF64(value);
    public static double Sqrt(double value) => InternalIntrinsics.SqrtF64(value);
    public static double Pow(double value, double power) => InternalIntrinsics.PowF64(value, power);
    public static double ReciprocalEstimate(double value) => 1.0d / value;
    public static double ReciprocalSqrtEstimate(double value) => 1.0d / Sqrt(value);
    public static double Sin(double value) => InternalIntrinsics.SinF64(value);
    public static double Cos(double value) => InternalIntrinsics.CosF64(value);
    public static double Tan(double value) => InternalIntrinsics.TanF64(value);
    public static double Asin(double value) => InternalIntrinsics.AsinF64(value);
    public static double Acos(double value) => InternalIntrinsics.AcosF64(value);
    public static double Atan(double value) => InternalIntrinsics.AtanF64(value);
    public static double Atan2(double y, double x) => InternalIntrinsics.Atan2F64(y, x);
    public static double Sinh(double value) => InternalIntrinsics.SinhF64(value);
    public static double Cosh(double value) => InternalIntrinsics.CoshF64(value);
    public static double Tanh(double value) => InternalIntrinsics.TanhF64(value);
    public static double Asinh(double value) => InternalIntrinsics.AsinhF64(value);
    public static double Acosh(double value) => InternalIntrinsics.AcoshF64(value);
    public static double Atanh(double value) => InternalIntrinsics.AtanhF64(value);
    public static double Exp(double value) => InternalIntrinsics.ExpF64(value);
    public static double Log(double value) => InternalIntrinsics.LogF64(value);
    public static double Log(double value, double baseValue) {
        if (baseValue == 1.0d)
        {
            return double.NaN;
        }
        return Log(value) / Log(baseValue);
    }
    public static double Log10(double value) => InternalIntrinsics.Log10F64(value);
    public static double Log2(double value) => InternalIntrinsics.Log2F64(value);
    public static void SinCos(double value, out double sin, out double cos) {
        sin = Sin(value);
        cos = Cos(value);
    }
    public static double Round(double value) => Round(value, 0, MidpointRounding.ToEven);
    public static double Round(double value, int digits) => Round(value, digits, MidpointRounding.ToEven);
    public static double Round(double value, MidpointRounding mode) => Round(value, 0, mode);
    public static double Round(double value, int digits, MidpointRounding mode) {
        ValidateRoundingMode(mode);
        return InternalIntrinsics.RoundF64(value, digits, (int) mode);
    }
    public static decimal Round(decimal value) => Round(value, 0, MidpointRounding.ToEven);
    public static decimal Round(decimal value, int digits) => Round(value, digits, MidpointRounding.ToEven);
    public static decimal Round(decimal value, MidpointRounding mode) => Round(value, 0, mode);
    public static decimal Round(decimal value, int digits, MidpointRounding mode) {
        ValidateRoundingMode(mode);
        let rounded = InternalIntrinsics.RoundF64(ConvertibleHelpers.ToFloat64FromDecimal(value), digits, (int) mode);
        return ConvertibleHelpers.ToDecimalFromFloat64(rounded);
    }
    private static bool IsNaN(double value) => value != value;
    private static void ValidateRoundingMode(MidpointRounding mode) {
        let raw = (int) mode;
        if (raw <0 || raw >4)
        {
            throw new ArgumentOutOfRangeException("mode");
        }
    }
}
