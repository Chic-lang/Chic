namespace Std.Security.Cryptography;
import Std;
import Std.Span;
import Std.Testing;
private static void AssertBytesEqual(ReadOnlySpan <byte >expected, ReadOnlySpan <byte >actual) {
    Assert.That(actual.Length).IsEqualTo(expected.Length);
    let len = expected.Length;
    for (var i = 0usize; i <len; i += 1usize) {
        Assert.That(actual[i]).IsEqualTo(expected[i]);
    }
}
private static byte[] Case1Ikm() {
    var ikm = new byte[22];
    var i = 0;
    while (i <22)
    {
        ikm[i] = 0x0Bu8;
        i += 1;
    }
    return ikm;
}
private static byte[] Case1Salt() {
    return new byte[13] {
        0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x06u8, 0x07u8, 0x08u8, 0x09u8, 0x0Au8, 0x0Bu8, 0x0Cu8,
    }
    ;
}
private static byte[] Case1Info() {
    return new byte[10] {
        0xF0u8, 0xF1u8, 0xF2u8, 0xF3u8, 0xF4u8, 0xF5u8, 0xF6u8, 0xF7u8, 0xF8u8, 0xF9u8,
    }
    ;
}
private static byte[] Case1Prk() {
    return new byte[32] {
        0x07u8, 0x77u8, 0x09u8, 0x36u8, 0x2Cu8, 0x2Eu8, 0x32u8, 0xDFu8, 0x0Du8, 0xDCu8, 0x3Fu8, 0x0Du8, 0xC4u8, 0x7Bu8, 0xBAu8, 0x63u8, 0x90u8, 0xB6u8, 0xC7u8, 0x3Bu8, 0xB5u8, 0x0Fu8, 0x9Cu8, 0x31u8, 0x22u8, 0xECu8, 0x84u8, 0x4Au8, 0xD7u8, 0xC2u8, 0xB3u8, 0xE5u8,
    }
    ;
}
private static byte[] Case1Okm() {
    return new byte[42] {
        0x3Cu8, 0xB2u8, 0x5Fu8, 0x25u8, 0xFAu8, 0xACu8, 0xD5u8, 0x7Au8, 0x90u8, 0x43u8, 0x4Fu8, 0x64u8, 0xD0u8, 0x36u8, 0x2Fu8, 0x2Au8, 0x2Du8, 0x2Du8, 0x0Au8, 0x90u8, 0xCFu8, 0x1Au8, 0x5Au8, 0x4Cu8, 0x5Du8, 0xB0u8, 0x2Du8, 0x56u8, 0xECu8, 0xC4u8, 0xC5u8, 0xBFu8, 0x34u8, 0x00u8, 0x72u8, 0x08u8, 0xD5u8, 0xB8u8, 0x87u8, 0x18u8, 0x58u8, 0x65u8,
    }
    ;
}
private static byte[] Case3Prk() {
    return new byte[32] {
        0x19u8, 0xEFu8, 0x24u8, 0xA3u8, 0x2Cu8, 0x71u8, 0x7Bu8, 0x16u8, 0x7Fu8, 0x33u8, 0xA9u8, 0x1Du8, 0x6Fu8, 0x64u8, 0x8Bu8, 0xDFu8, 0x96u8, 0x59u8, 0x67u8, 0x76u8, 0xAFu8, 0xDBu8, 0x63u8, 0x77u8, 0xACu8, 0x43u8, 0x4Cu8, 0x1Cu8, 0x29u8, 0x3Cu8, 0xCBu8, 0x04u8,
    }
    ;
}
testcase Given_hkdf_extract_sha256_known_vector_When_executed_Then_hkdf_extract_sha256_known_vector()
{
    let ikm = Case1Ikm();
    let salt = Case1Salt();
    var prk = new byte[32];
    let written = HKDF.Extract(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in ikm), ReadOnlySpan <byte >.FromArray(in salt),
    Span <byte >.FromArray(ref prk));
    Assert.That(written).IsEqualTo(32);
    let expected = Case1Prk();
    AssertBytesEqual(ReadOnlySpan <byte >.FromArray(in expected), ReadOnlySpan <byte >.FromArray(in prk));
}
testcase Given_hkdf_expand_sha256_known_vector_When_executed_Then_hkdf_expand_sha256_known_vector()
{
    let prk = Case1Prk();
    let info = Case1Info();
    var okm = new byte[42];
    let written = HKDF.Expand(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in prk), ReadOnlySpan <byte >.FromArray(in info),
    Span <byte >.FromArray(ref okm));
    Assert.That(written).IsEqualTo(42);
    let expected = Case1Okm();
    AssertBytesEqual(ReadOnlySpan <byte >.FromArray(in expected), ReadOnlySpan <byte >.FromArray(in okm));
}
testcase Given_hkdf_extract_empty_salt_matches_vector_When_executed_Then_hkdf_extract_empty_salt_matches_vector()
{
    let ikm = Case1Ikm();
    var prk = new byte[32];
    let written = HKDF.Extract(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in ikm), ReadOnlySpan <byte >.Empty,
    Span <byte >.FromArray(ref prk));
    Assert.That(written).IsEqualTo(32);
    let expected = Case3Prk();
    AssertBytesEqual(ReadOnlySpan <byte >.FromArray(in expected), ReadOnlySpan <byte >.FromArray(in prk));
}
testcase Given_hkdf_extract_rejects_short_destination_When_executed_Then_hkdf_extract_rejects_short_destination()
{
    let ikm = Case1Ikm();
    let salt = Case1Salt();
    var prk = new byte[31];
    Assert.Throws <ArgumentException >(() => {
        let _ = HKDF.Extract(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in ikm), ReadOnlySpan <byte >.FromArray(in salt),
        Span <byte >.FromArray(ref prk));
    }
    );
}
testcase Given_hkdf_expand_rejects_short_prk_When_executed_Then_hkdf_expand_rejects_short_prk()
{
    var prk = new byte[31];
    var output = new byte[1];
    Assert.Throws <ArgumentException >(() => {
        let _ = HKDF.Expand(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in prk), ReadOnlySpan <byte >.Empty,
        Span <byte >.FromArray(ref output));
    }
    );
}
testcase Given_hkdf_expand_rejects_too_long_output_When_executed_Then_hkdf_expand_rejects_too_long_output()
{
    let prk = Case1Prk();
    var output = new byte[8161];
    Assert.Throws <ArgumentException >(() => {
        let _ = HKDF.Expand(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in prk), ReadOnlySpan <byte >.Empty,
        Span <byte >.FromArray(ref output));
    }
    );
}
testcase Given_hkdf_derive_key_zero_length_returns_empty_When_executed_Then_hkdf_derive_key_zero_length_returns_empty()
{
    let ikm = Case1Ikm();
    let derived = HKDF.DeriveKey(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in ikm), ReadOnlySpan <byte >.Empty,
    ReadOnlySpan <byte >.Empty, 0);
    Assert.That(derived.Length).IsEqualTo(0);
}
testcase Given_hkdf_derive_key_negative_length_throws_When_executed_Then_hkdf_derive_key_negative_length_throws()
{
    let ikm = Case1Ikm();
    Assert.Throws <ArgumentOutOfRangeException >(() => {
        let _ = HKDF.DeriveKey(HashAlgorithmName.Sha256(), ReadOnlySpan <byte >.FromArray(in ikm), ReadOnlySpan <byte >.Empty,
        ReadOnlySpan <byte >.Empty, - 1);
    }
    );
}
