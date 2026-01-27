namespace Std.Security.Cryptography;
import Std.Span;
import Std.Testing;

testcase Given_aes_algorithm_rejects_invalid_key_length_When_executed_Then_aes_algorithm_rejects_invalid_key_length()
{
    var aes = new AesAlgorithm();
    var shortKey = new byte[15];
    Assert.Throws<Std.ArgumentException>(() => {
        aes.Key = ReadOnlySpan<byte>.FromArray(in shortKey);
    });
}

testcase Given_aes_algorithm_rejects_invalid_iv_length_When_executed_Then_aes_algorithm_rejects_invalid_iv_length()
{
    var aes = new AesAlgorithm();
    var iv = new byte[8];
    Assert.Throws<Std.ArgumentException>(() => {
        aes.IV = ReadOnlySpan<byte>.FromArray(in iv);
    });
}

testcase Given_aes_algorithm_requires_key_and_iv_for_transform_When_executed_Then_aes_algorithm_requires_key_and_iv_for_transform()
{
    var aes = new AesAlgorithm();
    Assert.Throws<Std.InvalidOperationException>(() => {
        let _ = aes.CreateEncryptor();
    });
}

testcase Given_aes_algorithm_rejects_unsupported_mode_When_executed_Then_aes_algorithm_rejects_unsupported_mode()
{
    var aes = new AesAlgorithm();
    aes.Mode = CipherMode.GCM;
    Assert.Throws<Std.NotSupportedException>(() => {
        let _ = aes.CreateDecryptor();
    });
}

testcase Given_aes_algorithm_accepts_key_and_iv_When_executed_Then_aes_algorithm_accepts_key_and_iv()
{
    var aes = new AesAlgorithm();
    var key = new byte[16];
    var iv = new byte[16];
    aes.Key = ReadOnlySpan<byte>.FromArray(in key);
    aes.IV = ReadOnlySpan<byte>.FromArray(in iv);
    let keySpan = aes.Key;
    let ivSpan = aes.IV;
    Assert.That(keySpan.Length).IsEqualTo(16usize);
}

testcase Given_aes_algorithm_accepts_iv_length_When_executed_Then_aes_algorithm_accepts_iv_length()
{
    var aes = new AesAlgorithm();
    var key = new byte[16];
    var iv = new byte[16];
    aes.Key = ReadOnlySpan<byte>.FromArray(in key);
    aes.IV = ReadOnlySpan<byte>.FromArray(in iv);
    let ivSpan = aes.IV;
    Assert.That(ivSpan.Length).IsEqualTo(16usize);
}

testcase Given_aes_factory_generate_key_length_When_executed_Then_aes_factory_generate_key_length()
{
    let key = Aes.GenerateKey(16);
    Assert.That(key.Length).IsEqualTo(16);
}

testcase Given_aes_factory_generate_iv_length_When_executed_Then_aes_factory_generate_iv_length()
{
    let iv = Aes.GenerateIV();
    Assert.That(iv.Length).IsEqualTo(16);
}

testcase Given_aes_factory_rejects_invalid_key_size_When_executed_Then_aes_factory_rejects_invalid_key_size()
{
    Assert.Throws<Std.ArgumentException>(() => {
        let _ = Aes.GenerateKey(20);
    });
}
