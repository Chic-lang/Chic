namespace Samples.Math;

import Std;

public static class Program
{
    public static int Main()
    {
        TestConstants();
        TestAbsAndClamp();
        TestMinMaxZeros();
        TestBigMul();
        TestRounding();
        TestFloatOps();
        TestTrig();
        return 0;
    }

    private static void TestConstants()
    {
        AssertNear(Math.PI, 3.141592653589793, 1e-12, "pi");
        AssertNear(Math.Tau, 6.283185307179586, 1e-12, "tau");
        AssertNear(MathF.PI, 3.1415927f, 1e-6f, "pi_f");
        AssertNear(MathF.Tau, 6.2831855f, 1e-5f, "tau_f");
    }

    private static void TestAbsAndClamp()
    {
        if (Math.Abs(-42) != 42)
        {
            throw new Std.InvalidOperationException("abs int");
        }
        if (Math.Clamp(5, 10, 20) != 10)
        {
            throw new Std.InvalidOperationException("clamp int");
        }
        if (Math.Clamp(25, 10, 20) != 20)
        {
            throw new Std.InvalidOperationException("clamp int high");
        }
        if (MathF.Abs(-0.0f) != 0.0f)
        {
            throw new Std.InvalidOperationException("abs float zero");
        }
    }

    private static void TestMinMaxZeros()
    {
        let min = Math.Min(-0.0d, 0.0d);
        let max = Math.Max(-0.0d, 0.0d);
        if (1.0d / min != double.NegativeInfinity)
        {
            throw new Std.InvalidOperationException("min zero sign");
        }
        if (1.0d / max != double.PositiveInfinity)
        {
            throw new Std.InvalidOperationException("max zero sign");
        }
    }

    private static void TestBigMul()
    {
        let result = Math.BigMul(100000, 40000);
        if (result != 4000000000L)
        {
            throw new Std.InvalidOperationException("bigmul int");
        }

        var low = 0L;
        let high = Math.BigMul(9223372036854775807L, 2L, out low);
        if (high == 0L && low == 0L)
        {
            throw new Std.InvalidOperationException("bigmul long");
        }
    }

    private static void TestRounding()
    {
        AssertNear(Math.Round(1.5d), 2.0d, 0.0d, "round even");
        AssertNear(Math.Round(2.5d), 2.0d, 0.0d, "round ties to even");
        AssertNear(Math.Round(1.5d, MidpointRounding.AwayFromZero), 2.0d, 0.0d, "round away");
        AssertNear(Math.Round(1.9d, MidpointRounding.ToZero), 1.0d, 0.0d, "round to zero");
        AssertNear(Math.Round(-1.1d, MidpointRounding.ToNegativeInfinity), -2.0d, 0.0d, "round down");
        AssertNear(Math.Round(-1.9d, MidpointRounding.ToPositiveInfinity), -1.0d, 0.0d, "round up");
        AssertNear(MathF.Round(2.5f), 2.0f, 0.0f, "roundf even");
    }

    private static void TestFloatOps()
    {
        AssertNear(Math.CopySign(1.0d, -2.0d), -1.0d, 0.0d, "copysign");
        if (Math.BitIncrement(1.0d) <= 1.0d)
        {
            throw new Std.InvalidOperationException("bit increment");
        }
        if (Math.BitDecrement(1.0d) >= 1.0d)
        {
            throw new Std.InvalidOperationException("bit decrement");
        }
        AssertNear(Math.ScaleB(1.0d, 3), 8.0d, 0.0d, "scaleb");
        if (Math.ILogB(8.0d) != 3)
        {
            throw new Std.InvalidOperationException("ilogb");
        }
        AssertNear(Math.IEEERemainder(5.0d, 2.0d), 1.0d, 0.0d, "ieee remainder");
        AssertNear(Math.ReciprocalEstimate(4.0d), 0.25d, 0.0d, "reciprocal");
        AssertNear(Math.ReciprocalSqrtEstimate(4.0d), 0.5d, 0.0d, "reciprocal sqrt");
        AssertNear(MathF.ReciprocalEstimate(4.0f), 0.25f, 0.0f, "reciprocal f");
        AssertNear(MathF.ReciprocalSqrtEstimate(4.0f), 0.5f, 0.0f, "reciprocal sqrt f");
    }

    private static void TestTrig()
    {
        AssertNear(Math.Sin(Math.PI / 2.0d), 1.0d, 1e-12, "sin");
        AssertNear(Math.Cos(0.0d), 1.0d, 1e-12, "cos");
        AssertNear(Math.Tan(0.0d), 0.0d, 1e-12, "tan");
        AssertNear(Math.Exp(0.0d), 1.0d, 1e-12, "exp");
        AssertNear(Math.Log(1.0d), 0.0d, 1e-12, "log");
        AssertNear(Math.Sqrt(4.0d), 2.0d, 0.0d, "sqrt");
        AssertNear(Math.Pow(2.0d, 3.0d), 8.0d, 0.0d, "pow");
    }

    private static void AssertNear(double value, double expected, double epsilon, string label)
    {
        if (Math.Abs(value - expected) > epsilon)
        {
            throw new Std.InvalidOperationException(label);
        }
    }

    private static void AssertNear(float value, float expected, float epsilon, string label)
    {
        if (MathF.Abs(value - expected) > epsilon)
        {
            throw new Std.InvalidOperationException(label);
        }
    }
}
