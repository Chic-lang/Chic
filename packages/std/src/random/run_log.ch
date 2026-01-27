namespace Std.Random;
import Std.Core;
import Std.Numeric;
public struct RngEvent
{
    public ulong Index;
    public string Kind;
    public uint Bits;
    public ulong AdvanceBy;
    public ulong SplitChild;
}
public struct RngStreamLog
{
    public ulong Id;
    public UInt128 Seed;
    public RngEvent[] Events;
    public usize EventCount;
}
public struct RunLog
{
    public string Version;
    public RngStreamLog[] Streams;
    public usize StreamCount;
}
/// <summary>Deterministic RNG run log recorder usable by both native and WASM targets.</summary>
public static class RunLogRecorder
{
    public const string RunLogVersion = "0.1";
    public static void Enable() {
        // run log currently acts as a no-op recorder
    }
    public static void Disable() {
        // no-op
    }
    public static void MaybeEnableFromEnv() {
        // environment-driven toggling is disabled in stubbed implementation
    }
    public static RunLog Snapshot() {
        var snapshot = new RunLog();
        snapshot.Version = RunLogVersion;
        snapshot.Streams = new RngStreamLog[0];
        snapshot.StreamCount = 0usize;
        return snapshot;
    }
    public static void RecordStream(ulong id, in UInt128 seed) {
        // recording is disabled in stub
    }
    public static void RecordEvent(ulong streamId, in UInt128 seed, string kind, uint bits, ulong advanceBy, ulong splitChild,
    ulong index) {
        // recording is disabled in stub
    }
    private static isize FindStream(ulong id) {
        return - 1isize;
    }
}
