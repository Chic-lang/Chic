namespace Std.Security.Cryptography;
import Std;
import Std.Numeric;
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
testcase Given_aes_gcm_rejects_invalid_key_length_When_executed_Then_aes_gcm_rejects_invalid_key_length()
{
    var key = new byte[15];
    Assert.Throws <ArgumentException >(() => {
        let _ = new AesGcm(ReadOnlySpan <byte >.FromArray(in key));
    }
    );
}
testcase Given_aes_gcm_encrypt_decrypt_roundtrip_When_executed_Then_aes_gcm_encrypt_decrypt_roundtrip()
{
    var key = new byte[16];
    var nonce = new byte[12];
    let plaintext = ReadOnlySpan.FromString("gcm");
    var ciphertext = new byte[NumericUnchecked.ToInt32(plaintext.Length)];
    var tag = new byte[16];
    var gcm = new AesGcm(ReadOnlySpan <byte >.FromArray(in key));
    gcm.Encrypt(ReadOnlySpan <byte >.FromArray(in nonce), plaintext, Span <byte >.FromArray(ref ciphertext), Span <byte >.FromArray(ref tag));
    var decrypted = new byte[ciphertext.Length];
    gcm.Decrypt(ReadOnlySpan <byte >.FromArray(in nonce), ReadOnlySpan <byte >.FromArray(in ciphertext), ReadOnlySpan <byte >.FromArray(in tag),
    Span <byte >.FromArray(ref decrypted));
    Assert.That(BytesEqual(plaintext, ReadOnlySpan <byte >.FromArray(in decrypted))).IsTrue();
}
testcase Given_aes_gcm_rejects_invalid_tag_When_executed_Then_aes_gcm_rejects_invalid_tag()
{
    var key = new byte[16];
    var nonce = new byte[12];
    let plaintext = ReadOnlySpan.FromString("gcm");
    var ciphertext = new byte[NumericUnchecked.ToInt32(plaintext.Length)];
    var tag = new byte[16];
    var gcm = new AesGcm(ReadOnlySpan <byte >.FromArray(in key));
    gcm.Encrypt(ReadOnlySpan <byte >.FromArray(in nonce), plaintext, Span <byte >.FromArray(ref ciphertext), Span <byte >.FromArray(ref tag));
    tag[0] = (byte)(tag[0] ^ 0xFFu8);
    Assert.Throws <InvalidOperationException >(() => {
        var decrypted = new byte[ciphertext.Length]; gcm.Decrypt(ReadOnlySpan <byte >.FromArray(in nonce), ReadOnlySpan <byte >.FromArray(in ciphertext),
        ReadOnlySpan <byte >.FromArray(in tag), Span <byte >.FromArray(ref decrypted));
    }
    );
}
testcase Given_aes_gcm_rejects_small_buffers_When_executed_Then_aes_gcm_rejects_small_buffers()
{
    var key = new byte[16];
    var nonce = new byte[12];
    let plaintext = ReadOnlySpan.FromString("data");
    var gcm = new AesGcm(ReadOnlySpan <byte >.FromArray(in key));
    var tag = Span <byte >.StackAlloc(8usize);
    Assert.Throws <ArgumentException >(() => {
        var ciphertext = Span <byte >.StackAlloc(1usize); gcm.Encrypt(ReadOnlySpan <byte >.FromArray(in nonce), plaintext,
        ciphertext, tag);
    }
    );
}
testcase Given_aes_gcm_rejects_small_tag_buffer_When_executed_Then_aes_gcm_rejects_small_tag_buffer()
{
    var key = new byte[16];
    var nonce = new byte[12];
    let plaintext = ReadOnlySpan.FromString("data");
    var gcm = new AesGcm(ReadOnlySpan <byte >.FromArray(in key));
    var tag = Span <byte >.StackAlloc(8usize);
    Assert.Throws <ArgumentException >(() => {
        var ciphertext = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(plaintext.Length)); gcm.Encrypt(ReadOnlySpan <byte >.FromArray(in nonce),
        plaintext, ciphertext, tag);
    }
    );
}
