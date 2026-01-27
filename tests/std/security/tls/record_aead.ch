namespace Exec;

import Std.Security.Tls;
import Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;

testcase RecordAeadRoundTrips()
{
    let key = ParseHex("000102030405060708090a0b0c0d0e0f");
    let iv = ParseHex("0f0e0d0c0b0a090807060504");
    var protector = new TlsRecordAead(TlsCipherSuite.TlsAes128GcmSha256, ReadOnlySpan<byte>.FromArray(ref key), ReadOnlySpan<byte>.FromArray(ref iv));

    let plaintextBytes = new byte[] { 0x68u8, 0x65u8, 0x6cu8, 0x6cu8, 0x6fu8, 0x20u8, 0x74u8, 0x6cu8, 0x73u8 };
    let plaintext = ReadOnlySpan<byte>.FromArray(ref plaintextBytes);
    var record = new byte[5 + plaintext.Length + 16];
    let written = protector.EncryptRecord(1ul, TlsContentType.ApplicationData, plaintext, Span<byte>.FromArray(ref record));

    var decrypted = new byte[plaintext.Length];
    let read = protector.DecryptRecord(1ul, ReadOnlySpan<byte>.FromArray(ref record).Slice(0usize, NumericUnchecked.ToUSize(written)), Span<byte>.FromArray(ref decrypted), out var contentType);

    return contentType == TlsContentType.ApplicationData
        && read == plaintext.Length
        && Matches(ReadOnlySpan<byte>.FromArray(ref decrypted), plaintext);
}

testcase RecordAeadRejectsTamper()
{
    let key = ParseHex("1f1e1d1c1b1a19181716151413121110");
    let iv = ParseHex("a0a1a2a3a4a5a6a7a8a9aaab");
    var protector = new TlsRecordAead(TlsCipherSuite.TlsAes128GcmSha256, ReadOnlySpan<byte>.FromArray(ref key), ReadOnlySpan<byte>.FromArray(ref iv));

    let plaintextBytes = new byte[] { 0x74u8, 0x61u8, 0x6du8, 0x70u8, 0x65u8, 0x72u8, 0x20u8, 0x6du8, 0x65u8 };
    let plaintext = ReadOnlySpan<byte>.FromArray(ref plaintextBytes);
    var record = new byte[5 + plaintext.Length + 16];
    protector.EncryptRecord(5ul, TlsContentType.ApplicationData, plaintext, Span<byte>.FromArray(ref record));

    record[record.Length - 1usize] = (byte)(record[record.Length - 1usize] ^ 0xFFu8);
    var dest = new byte[plaintext.Length];
    try
    {
        protector.DecryptRecord(5ul, ReadOnlySpan<byte>.FromArray(ref record), Span<byte>.FromArray(ref dest), out var _);
    }
    catch (Std.Security.Tls.TlsAlertException)
    {
        return true;
    }
    catch (Std.InvalidOperationException)
    {
        return true;
    }
    return false;
}

    private static bool Matches(ReadOnlySpan<byte> left, ReadOnlySpan<byte> right)
    {
        if (left.Length != right.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx < left.Length)
        {
            if (left[idx] != right[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }

    private static byte[] ParseHex(string text)
    {
        if (text == null || (text.Length % 2) != 0)
        {
            return new byte[0];
        }
        var length = text.Length / 2;
        var output = new byte[length];
        var idx = 0;
        while (idx < length)
        {
            let high = ValueOf(text[idx * 2]);
            let low = ValueOf(text[idx * 2 + 1]);
            output[idx] = Std.Numeric.NumericUnchecked.ToByte((high << 4) | low);
            idx += 1;
        }
        return output;
    }

    private static int ValueOf(char c)
    {
        if (c >= '0' && c <= '9')
        {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f')
        {
            return 10 + (c - 'a');
        }
        if (c >= 'A' && c <= 'F')
        {
            return 10 + (c - 'A');
        }
        return 0;
    }
}
