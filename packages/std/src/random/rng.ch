namespace Std.Random;
import Std.Core;
import Std.Hashing;
import Std.Numeric;
import Std.Testing;
public struct SplitResult
{
    public RNG Child;
}
public struct RNG
{
    public ulong State0;
    public ulong State1;
    public ulong StreamId;
    public UInt128 Seed;
    public ulong Counter;
    public init() {
        self.State0 = 0ul;
        self.State1 = 0ul;
        self.StreamId = 0ul;
        self.Seed = new UInt128(0u128);
        self.Counter = 0ul;
    }
    public static RNG Seed(ulong hi, ulong lo) {
        var rng = new RNG();
        rng.Seed = ComposeSeed(hi, lo);
        rng.State0 = SplitMix64(lo);
        rng.State1 = SplitMix64(hi);
        rng.StreamId = DeriveStreamId(in rng.Seed, 0ul);
        rng.Counter = 0ul;
        RunLogRecorder.RecordStream(rng.StreamId, in rng.Seed);
        return rng;
    }
    public static ulong NextU64(ref RNG rng) {
        let result = NumericBitOperations.RotateLeftUInt64(rng.State0 + rng.State1, 17) + rng.State0;
        let s1 = rng.State1 ^ rng.State0;
        rng.State0 = NumericBitOperations.RotateLeftUInt64(rng.State0, 49) ^ s1 ^ (s1 << 21);
        rng.State1 = NumericBitOperations.RotateLeftUInt64(s1, 28);
        rng.Counter = rng.Counter + 1ul;
        RunLogRecorder.RecordEvent(rng.StreamId, in rng.Seed, "next", 64u, 0ul, 0ul, rng.Counter - 1ul);
        return result;
    }
    public static uint NextU32(ref RNG rng) {
        return NumericUnchecked.ToUInt32(NextU64(ref rng));
    }
    public static UInt128 NextU128(ref RNG rng) {
        let hi = NextU64(ref rng);
        let lo = NextU64(ref rng);
        return ComposeSeed(hi, lo);
    }
    public static void Advance(ref RNG rng, ulong n) {
        var remaining = n;
        while (remaining >0ul)
        {
            var s0 = rng.State0;
            var s1 = rng.State1;
            let _ = NextU64Internal(ref s0, ref s1);
            rng.State0 = s0;
            rng.State1 = s1;
            rng.Counter = rng.Counter + 1ul;
            remaining = remaining - 1ul;
        }
        RunLogRecorder.RecordEvent(rng.StreamId, in rng.Seed, "advance", 0u, n, 0ul, rng.Counter);
    }
    public static SplitResult Split(ref RNG rng) {
        const ulong Jump0 = 0xDF900294D8F554A5ul;
        const ulong Jump1 = 0x170865DF4B3201FCul;
        var childState0 = rng.State0;
        var childState1 = rng.State1;
        var parentState0 = rng.State0;
        var parentState1 = rng.State1;
        JumpState(ref parentState0, ref parentState1, Jump0, Jump1);
        rng.State0 = parentState0;
        rng.State1 = parentState1;
        JumpState(ref childState0, ref childState1, Jump0, Jump1);
        let childSeed = DeriveChildSeed(in rng.Seed, 0ul);
        let childStreamId = DeriveStreamId(in childSeed, 1ul);
        var child = new RNG();
        child.Seed = childSeed;
        child.State0 = childState0;
        child.State1 = childState1;
        child.StreamId = childStreamId;
        child.Counter = 0ul;
        RunLogRecorder.RecordStream(child.StreamId, in child.Seed);
        RunLogRecorder.RecordEvent(rng.StreamId, in rng.Seed, "split", 0u, 0ul, child.StreamId, rng.Counter);
        var result = new SplitResult();
        result.Child = child;
        return result;
    }
    private static UInt128 ComposeSeed(ulong hi, ulong lo) {
        let hi128 = NumericUnchecked.ToUInt128(hi);
        let lo128 = NumericUnchecked.ToUInt128(lo);
        let combined = (hi128 << 64) | lo128;
        return new UInt128(combined);
    }
    private static ulong SplitMix64(ulong x) {
        var z = x + 0x9E3779B97F4A7C15ul;
        z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ul;
        z = (z ^ (z >> 27)) * 0x94D049BB133111EBul;
        return z ^ (z >> 31);
    }
    private static void JumpState(ref ulong s0, ref ulong s1, ulong jump0, ulong jump1) {
        var accum0 = 0ul;
        var accum1 = 0ul;
        JumpWord(ref s0, ref s1, jump0, ref accum0, ref accum1);
        JumpWord(ref s0, ref s1, jump1, ref accum0, ref accum1);
        s0 = accum0;
        s1 = accum1;
    }
    private static void JumpWord(ref ulong s0, ref ulong s1, ulong jumpWord, ref ulong accum0, ref ulong accum1) {
        var bit = 0;
        while (bit <64)
        {
            if ( (jumpWord & (1ul << NumericUnchecked.ToUInt32 (bit))) != 0ul)
            {
                accum0 = accum0 ^ s0;
                accum1 = accum1 ^ s1;
            }
            let _ = NextU64Internal(ref s0, ref s1);
            bit = bit + 1;
        }
    }
    private static ulong NextU64Internal(ref ulong state0, ref ulong state1) {
        let result = NumericBitOperations.RotateLeftUInt64(state0 + state1, 17) + state0;
        let s1 = state1 ^ state0;
        state0 = NumericBitOperations.RotateLeftUInt64(state0, 49) ^ s1 ^ (s1 << 21);
        state1 = NumericBitOperations.RotateLeftUInt64(s1, 28);
        return result;
    }
    private static UInt128 DeriveChildSeed(in UInt128 seed, ulong lane) {
        let mixed = Hashing.HashValue <UInt128, DefaultHasher >(in seed, new DefaultHasher());
        let lane64 = mixed ^ lane;
        let hi = (seed >> 64).ToUInt64(null);
        let lo = seed.ToUInt64(null);
        let derivedHi = hi ^ lane64;
        let derivedLo = lo ^ (lane64 << 1);
        return ComposeSeed(derivedHi, derivedLo);
    }
    private static ulong DeriveStreamId(in UInt128 seed, ulong salt) {
        let hi = (seed >> 64).ToUInt64(null);
        let lo = seed.ToUInt64(null);
        let combined = hi ^ lo ^ salt;
        return Hashing.HashValue <ulong, DefaultHasher >(in combined, new DefaultHasher());
    }
}
testcase Given_rng_sequence_first_value_When_executed_Then_rng_sequence_first_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let value = RNG.NextU64(ref rng);
    Assert.That(value).IsEqualTo(0x7566e3a5d9bae565ul);
}
testcase Given_rng_sequence_second_value_When_executed_Then_rng_sequence_second_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let value = RNG.NextU64(ref rng);
    Assert.That(value).IsEqualTo(0xd2cf6de25e02cf7dul);
}
testcase Given_rng_sequence_third_value_When_executed_Then_rng_sequence_third_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let value = RNG.NextU64(ref rng);
    Assert.That(value).IsEqualTo(0x93386e2fdd7712ddul);
}
testcase Given_rng_advance_skips_first_value_When_executed_Then_rng_advance_skips_first_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    RNG.Advance(ref rng, 3ul);
    let value = RNG.NextU64(ref rng);
    Assert.That(value).IsEqualTo(0xd0b5e24fcebe1778ul);
}
testcase Given_rng_advance_skips_second_value_When_executed_Then_rng_advance_skips_second_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    RNG.Advance(ref rng, 3ul);
    let _ = RNG.NextU64(ref rng);
    let value = RNG.NextU64(ref rng);
    Assert.That(value).IsEqualTo(0xf69db286ae86440cul);
}
testcase Given_rng_split_parent0_value_When_executed_Then_rng_split_parent0_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let split = RNG.Split(ref rng);
    var parent = rng;
    let parent0 = RNG.NextU64(ref parent);
    Assert.That(parent0).IsEqualTo(0x0099e78375f79648ul);
}
testcase Given_rng_split_parent1_value_When_executed_Then_rng_split_parent1_value()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let split = RNG.Split(ref rng);
    var parent = rng;
    let _ = RNG.NextU64(ref parent);
    let parent1 = RNG.NextU64(ref parent);
    Assert.That(parent1).IsEqualTo(0xe849526d3138f961ul);
}
testcase Given_rng_split_child0_matches_parent0_When_executed_Then_rng_split_child0_matches_parent0()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let split = RNG.Split(ref rng);
    var child = split.Child;
    var parent = rng;
    let parent0 = RNG.NextU64(ref parent);
    let child0 = RNG.NextU64(ref child);
    Assert.That(child0).IsEqualTo(parent0);
}
testcase Given_rng_split_child1_matches_parent1_When_executed_Then_rng_split_child1_matches_parent1()
{
    var rng = RNG.Seed(0x1234ul, 0x5678ul);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let _ = RNG.NextU64(ref rng);
    let split = RNG.Split(ref rng);
    var child = split.Child;
    var parent = rng;
    let _ = RNG.NextU64(ref parent);
    let parent1 = RNG.NextU64(ref parent);
    let _ = RNG.NextU64(ref child);
    let child1 = RNG.NextU64(ref child);
    Assert.That(child1).IsEqualTo(parent1);
}
