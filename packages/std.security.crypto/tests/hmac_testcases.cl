namespace Std.Security.Cryptography;
import Std;
import Std.Span;
import Std.Testing;

private static void AssertBytesEqual(ReadOnlySpan<byte> expected, ReadOnlySpan<byte> actual) {
    Assert.That(actual.Length).IsEqualTo(expected.Length);
    let len = expected.Length;
    for (var i = 0usize; i <len; i += 1usize) {
        Assert.That(actual[i]).IsEqualTo(expected[i]);
    }
}

private static byte[] Key20Bytes() {
    var key = new byte[20];
    var i = 0;
    while (i <20)
    {
        key[i] = 0x0Bu8;
        i += 1;
    }
    return key;
}

testcase Given_hmac_sha256_known_vector_When_executed_Then_hmac_sha256_known_vector()
{
    let key = Key20Bytes();
    let data = ReadOnlySpan.FromString("Hi There");
    var hmac = new HmacSha256();
    hmac.SetKey(ReadOnlySpan<byte>.FromArray(in key));
    hmac.Append(data);
    var output = new byte[32];
    let written = hmac.FinalizeHash(Span<byte>.FromArray(ref output));
    Assert.That(written).IsEqualTo(32);
    let expected = new byte[32] {
        0xB0u8, 0x34u8, 0x4Cu8, 0x61u8, 0xD8u8, 0xDBu8, 0x38u8, 0x53u8,
        0x5Cu8, 0xA8u8, 0xAFu8, 0xCEu8, 0xAFu8, 0x0Bu8, 0xF1u8, 0x2Bu8,
        0x88u8, 0x1Du8, 0xC2u8, 0x00u8, 0xC9u8, 0x83u8, 0x3Du8, 0xA7u8,
        0x26u8, 0xE9u8, 0x37u8, 0x6Cu8, 0x2Eu8, 0x32u8, 0xCFu8, 0xF7u8,
    }
    ;
    AssertBytesEqual(ReadOnlySpan<byte>.FromArray(in expected), ReadOnlySpan<byte>.FromArray(in output));
}

testcase Given_hmac_sha384_known_vector_When_executed_Then_hmac_sha384_known_vector()
{
    let key = Key20Bytes();
    let data = ReadOnlySpan.FromString("Hi There");
    var hmac = new HmacSha384();
    hmac.SetKey(ReadOnlySpan<byte>.FromArray(in key));
    hmac.Append(data);
    var output = new byte[48];
    let written = hmac.FinalizeHash(Span<byte>.FromArray(ref output));
    Assert.That(written).IsEqualTo(48);
    let expected = new byte[48] {
        0xAFu8, 0xD0u8, 0x39u8, 0x44u8, 0xD8u8, 0x48u8, 0x95u8, 0x62u8,
        0x6Bu8, 0x08u8, 0x25u8, 0xF4u8, 0xABu8, 0x46u8, 0x90u8, 0x7Fu8,
        0x15u8, 0xF9u8, 0xDAu8, 0xDBu8, 0xE4u8, 0x10u8, 0x1Eu8, 0xC6u8,
        0x82u8, 0xAAu8, 0x03u8, 0x4Cu8, 0x7Cu8, 0xEBu8, 0xC5u8, 0x9Cu8,
        0xFAu8, 0xEAu8, 0x9Eu8, 0xA9u8, 0x07u8, 0x6Eu8, 0xDEu8, 0x7Fu8,
        0x4Au8, 0xF1u8, 0x52u8, 0xE8u8, 0xB2u8, 0xFAu8, 0x9Cu8, 0xB6u8,
    }
    ;
    AssertBytesEqual(ReadOnlySpan<byte>.FromArray(in expected), ReadOnlySpan<byte>.FromArray(in output));
}

testcase Given_hmac_sha512_known_vector_When_executed_Then_hmac_sha512_known_vector()
{
    let key = Key20Bytes();
    let data = ReadOnlySpan.FromString("Hi There");
    var hmac = new HmacSha512();
    hmac.SetKey(ReadOnlySpan<byte>.FromArray(in key));
    hmac.Append(data);
    var output = new byte[64];
    let written = hmac.FinalizeHash(Span<byte>.FromArray(ref output));
    Assert.That(written).IsEqualTo(64);
    let expected = new byte[64] {
        0x87u8, 0xAAu8, 0x7Cu8, 0xDEu8, 0xA5u8, 0xEFu8, 0x61u8, 0x9Du8,
        0x4Fu8, 0xF0u8, 0xB4u8, 0x24u8, 0x1Au8, 0x1Du8, 0x6Cu8, 0xB0u8,
        0x23u8, 0x79u8, 0xF4u8, 0xE2u8, 0xCEu8, 0x4Eu8, 0xC2u8, 0x78u8,
        0x7Au8, 0xD0u8, 0xB3u8, 0x05u8, 0x45u8, 0xE1u8, 0x7Cu8, 0xDEu8,
        0xDAu8, 0xA8u8, 0x33u8, 0xB7u8, 0xD6u8, 0xB8u8, 0xA7u8, 0x02u8,
        0x03u8, 0x8Bu8, 0x27u8, 0x4Eu8, 0xAEu8, 0xA3u8, 0xF4u8, 0xE4u8,
        0xBEu8, 0x9Du8, 0x91u8, 0x4Eu8, 0xEBu8, 0x61u8, 0xF1u8, 0x70u8,
        0x2Eu8, 0x69u8, 0x6Cu8, 0x20u8, 0x3Au8, 0x12u8, 0x68u8, 0x54u8,
    }
    ;
    AssertBytesEqual(ReadOnlySpan<byte>.FromArray(in expected), ReadOnlySpan<byte>.FromArray(in output));
}

testcase Given_hmac_factory_rejects_unknown_hash_When_executed_Then_hmac_factory_rejects_unknown_hash()
{
    var digest = 0;
    Assert.Throws<NotSupportedException>(() => {
        let _ = HmacFactory.Create(new HashAlgorithmName("MD5"), out digest);
    });
}
