namespace Std.Security.Cryptography;
import Std.Span;
import Std.Testing;
private sealed class DummyHash : HashAlgorithm
{
    public int ResetCount;
    public int FinalSize;
    public init(int finalSize = 2) {
        ResetCount = 0;
        FinalSize = finalSize;
    }
    public override int HashSizeBits => 16;
    public override void Append(ReadOnlySpan <byte >data) {
        let _ = data.Length;
    }
    public override int FinalizeHash(Span <byte >destination) {
        destination[0usize] = 1u8;
        if (FinalSize >1)
        {
            destination[1usize] = 2u8;
        }
        return FinalSize;
    }
    public override void Reset() {
        ResetCount += 1;
    }
}
testcase Given_hash_algorithm_compute_hash_trims_output_When_executed_Then_hash_algorithm_compute_hash_trims_output()
{
    var algo = new DummyHash(1);
    let data = ReadOnlySpan.FromString("ab");
    let output = algo.ComputeHash(data);
    Assert.That(output.Length).IsEqualTo(1);
}
testcase Given_hash_algorithm_compute_hash_first_byte_When_executed_Then_hash_algorithm_compute_hash_first_byte()
{
    var algo = new DummyHash(1);
    let data = ReadOnlySpan.FromString("ab");
    let output = algo.ComputeHash(data);
    Assert.That(output[0]).IsEqualTo(1u8);
}
testcase Given_hash_algorithm_compute_hash_resets_twice_When_executed_Then_hash_algorithm_compute_hash_resets_twice()
{
    var algo = new DummyHash(1);
    let data = ReadOnlySpan.FromString("ab");
    let _ = algo.ComputeHash(data);
    Assert.That(algo.ResetCount).IsEqualTo(2);
}
testcase Given_hash_algorithm_factory_creates_sha384_When_executed_Then_hash_algorithm_factory_creates_sha384()
{
    var sha384 = HashAlgorithmFactory.CreateSha384();
    Assert.That(sha384.HashSizeBits).IsEqualTo(384);
}
testcase Given_hash_algorithm_factory_creates_sha512_When_executed_Then_hash_algorithm_factory_creates_sha512()
{
    var sha512 = HashAlgorithmFactory.CreateSha512();
    Assert.That(sha512.HashSizeBits).IsEqualTo(512);
}
