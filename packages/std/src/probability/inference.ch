namespace Std.Probability;
import Std.Core;
/// <summary>Lightweight trace handle for inference engines to consume.</summary>
public struct TraceHandle
{
    public double LogProb;
}
public static class Inference
{
    public static TraceHandle NewTrace() {
        var t = new TraceHandle();
        t.LogProb = 0.0;
        return t;
    }
}
