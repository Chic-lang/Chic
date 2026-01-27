namespace Std.Security.Cryptography;
import Std;
import Std.Span;
import Std.Testing;
private static bool BytesEqual(ReadOnlySpan <byte >expected, ReadOnlySpan <byte >actual) {
    if (actual.Length != expected.Length)
    {
        return false;
    }
    let len = expected.Length;
    for (var i = 0usize; i <len; i += 1usize) {
        if (actual[i] != expected[i])
        {
            return false;
        }
    }
    return true;
}
testcase Given_pbkdf2_sha256_iter1_known_vector_When_executed_Then_pbkdf2_sha256_iter1_known_vector()
{
    let password = ReadOnlySpan.FromString("password");
    let salt = ReadOnlySpan.FromString("salt");
    let key = Rfc2898DeriveBytes.Pbkdf2(password, salt, 1, 32, HashAlgorithmName.Sha256());
    let expected = new byte[32] {
        0x12u8, 0x0Fu8, 0xB6u8, 0xCFu8, 0xFCu8, 0xF8u8, 0xB3u8, 0x2Cu8, 0x43u8, 0xE7u8, 0x22u8, 0x52u8, 0x56u8, 0xC4u8, 0xF8u8, 0x37u8, 0xA8u8, 0x65u8, 0x48u8, 0xC9u8, 0x2Cu8, 0xCCu8, 0x35u8, 0x48u8, 0x08u8, 0x05u8, 0x98u8, 0x7Cu8, 0xB7u8, 0x0Bu8, 0xE1u8, 0x7Bu8,
    }
    ;
    Assert.That(BytesEqual(ReadOnlySpan <byte >.FromArray(in expected), ReadOnlySpan <byte >.FromArray(in key))).IsTrue();
}
testcase Given_pbkdf2_sha256_iter2_known_vector_When_executed_Then_pbkdf2_sha256_iter2_known_vector()
{
    let password = ReadOnlySpan.FromString("password");
    let salt = ReadOnlySpan.FromString("salt");
    let key = Rfc2898DeriveBytes.Pbkdf2(password, salt, 2, 32, HashAlgorithmName.Sha256());
    let expected = new byte[32] {
        0xAEu8, 0x4Du8, 0x0Cu8, 0x95u8, 0xAFu8, 0x6Bu8, 0x46u8, 0xD3u8, 0x2Du8, 0x0Au8, 0xDFu8, 0xF9u8, 0x28u8, 0xF0u8, 0x6Du8, 0xD0u8, 0x2Au8, 0x30u8, 0x3Fu8, 0x8Eu8, 0xF3u8, 0xC2u8, 0x51u8, 0xDFu8, 0xD6u8, 0xE2u8, 0xD8u8, 0x5Au8, 0x95u8, 0x47u8, 0x4Cu8, 0x43u8,
    }
    ;
    Assert.That(BytesEqual(ReadOnlySpan <byte >.FromArray(in expected), ReadOnlySpan <byte >.FromArray(in key))).IsTrue();
}
testcase Given_pbkdf2_rejects_zero_iterations_When_executed_Then_pbkdf2_rejects_zero_iterations()
{
    let password = ReadOnlySpan.FromString("password");
    let salt = ReadOnlySpan.FromString("salt");
    Assert.Throws <ArgumentOutOfRangeException >(() => {
        let _ = Rfc2898DeriveBytes.Pbkdf2(password, salt, 0, 32, HashAlgorithmName.Sha256());
    }
    );
}
testcase Given_pbkdf2_rejects_zero_length_When_executed_Then_pbkdf2_rejects_zero_length()
{
    let password = ReadOnlySpan.FromString("password");
    let salt = ReadOnlySpan.FromString("salt");
    Assert.Throws <ArgumentOutOfRangeException >(() => {
        let _ = Rfc2898DeriveBytes.Pbkdf2(password, salt, 1, 0, HashAlgorithmName.Sha256());
    }
    );
}
testcase Given_pbkdf2_rejects_negative_length_When_executed_Then_pbkdf2_rejects_negative_length()
{
    let password = ReadOnlySpan.FromString("password");
    let salt = ReadOnlySpan.FromString("salt");
    Assert.Throws <ArgumentOutOfRangeException >(() => {
        let _ = Rfc2898DeriveBytes.Pbkdf2(password, salt, 1, - 1, HashAlgorithmName.Sha256());
    }
    );
}
