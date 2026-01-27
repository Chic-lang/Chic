namespace Std.Probability;
import Std.Core;
import Std.Random;
import Std.Numeric;
public struct Uniform
{
    public double Low;
    public double High;
}
public struct LogProb
{
    public double Value;
}
public static class Distributions
{
    public static Uniform NewUniform(double low, double high) {
        var u = new Uniform();
        u.Low = low;
        u.High = high;
        return u;
    }
    public static LogProb NewLogProb() {
        var lp = new LogProb();
        lp.Value = 0.0;
        return lp;
    }
    public static double Sample(ref RNG rng, Uniform uniform) {
        let bits = RNG.NextU64(ref rng);
        let unit = (double) bits / (double) ulong.MaxValue;
        return uniform.Low + (uniform.High - uniform.Low) * unit;
    }
    public static void Add(ref LogProb accumulator, double logProb) {
        accumulator.Value = accumulator.Value + logProb;
    }
}
