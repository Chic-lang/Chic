namespace Std.Security.Cryptography;
import Std.Span;
import Std.Testing;

private static bool BytesEqual(ReadOnlySpan<byte> expected, ReadOnlySpan<byte> actual) {
    if (actual.Length != expected.Length)
    {
        return false;
    }
    let len = expected.Length;
    for (var i = 0usize; i <len; i += 1usize)
    {
        if (actual[i] != expected[i])
        {
            return false;
        }
    }
    return true;
}

testcase Given_sha256_hash_matches_known_vector_When_executed_Then_sha256_hash_matches_known_vector()
{
    let data = ReadOnlySpan.FromString("abc");
    var sha = new SHA256();
    let hash = sha.ComputeHash(data);
    var expected = new byte[32];
    expected[0] = 0xBAu8;
    expected[1] = 0x78u8;
    expected[2] = 0x16u8;
    expected[3] = 0xBFu8;
    expected[4] = 0x8Fu8;
    expected[5] = 0x01u8;
    expected[6] = 0xCFu8;
    expected[7] = 0xEAu8;
    expected[8] = 0x41u8;
    expected[9] = 0x41u8;
    expected[10] = 0x40u8;
    expected[11] = 0xDEu8;
    expected[12] = 0x5Du8;
    expected[13] = 0xAEu8;
    expected[14] = 0x22u8;
    expected[15] = 0x23u8;
    expected[16] = 0xB0u8;
    expected[17] = 0x03u8;
    expected[18] = 0x61u8;
    expected[19] = 0xA3u8;
    expected[20] = 0x96u8;
    expected[21] = 0x17u8;
    expected[22] = 0x7Au8;
    expected[23] = 0x9Cu8;
    expected[24] = 0xB4u8;
    expected[25] = 0x10u8;
    expected[26] = 0xFFu8;
    expected[27] = 0x61u8;
    expected[28] = 0xF2u8;
    expected[29] = 0x00u8;
    expected[30] = 0x15u8;
    expected[31] = 0xADu8;
    let span = Span<byte>.FromArray(ref hash);
    let expectedSpan = ReadOnlySpan<byte>.FromArray(ref expected);
    let ok = hash.Length == 32usize && BytesEqual(expectedSpan, span.AsReadOnly());
    Assert.That(ok).IsTrue();
}

testcase Given_hash_algorithm_factory_creates_sha256_When_executed_Then_hash_algorithm_factory_creates_sha256()
{
    let algo = HashAlgorithmFactory.CreateSha256();
    Assert.That(algo.HashSizeBits).IsEqualTo(256);
}
