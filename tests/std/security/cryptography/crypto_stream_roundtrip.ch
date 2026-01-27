namespace Exec;

import Std.Security.Cryptography;
import Std.IO;
import Std.Span;
import Std.Numeric;
import Std.Async;
import Std.Strings;

public static class CryptoStreamRoundTrip
{
    public static int Main()
    {
        if (!RoundTrip("short".AsUtf8Span()))
        {
            return 1;
        }
        if (!RoundTrip("sixteen-byte-msg".AsUtf8Span())) // 16 bytes
        {
            return 2;
        }
        if (!RoundTripLarge())
        {
            return 3;
        }
        if (!AsyncCancellation())
        {
            return 4;
        }
        return 0;
    }

    private static bool RoundTrip(ReadOnlySpan<byte> plain)
    {
        var key = Hex.Parse("00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff");
        var iv = Hex.Parse("0f1e2d3c4b5a69788796a5b4c3d2e1f0");

        var encryptor = BuildEncryptor(key, iv);
        var ms = new MemoryStream();
        var cs = new CryptoStream(ms, encryptor, CryptoStreamMode.Write);
        cs.Write(plain);
        cs.FlushFinalBlock();
        let cipher = ms.ToArray();
        cs.dispose(ref cs);
        ms.dispose(ref ms);

        var decryptor = BuildDecryptor(key, iv);
        var cipherStream = new MemoryStream(cipher, false);
        var decryptStream = new CryptoStream(cipherStream, decryptor, CryptoStreamMode.Read);
        var output = new byte[NumericUnchecked.ToInt32(plain.Length + NumericUnchecked.ToUSize(encryptor.OutputBlockSize))];
        var total = 0usize;
        while (true)
        {
            let read = decryptStream.Read(Span<byte>.FromArray(ref output).Slice(
                total,
                NumericUnchecked.ToUSize(output.Length) - total
            ));
            if (read == 0)
            {
                break;
            }
            total += NumericUnchecked.ToUSize(read);
        }
        decryptStream.dispose(ref decryptStream);
        cipherStream.dispose(ref cipherStream);

        if (total != plain.Length)
        {
            return false;
        }
        var expected = new byte[NumericUnchecked.ToInt32(plain.Length)];
        Span<byte>.FromArray(ref expected).CopyFrom(plain);
        return Matches(ReadOnlySpan<byte>.FromArray(ref output).Slice(0usize, total), ReadOnlySpan<byte>.FromArray(ref expected));
    }

    private static bool RoundTripLarge()
    {
        var data = new byte[2048];
        var idx = 0usize;
        while (idx < NumericUnchecked.ToUSize(data.Length))
        {
            data[idx] = NumericUnchecked.ToByte((idx * 37usize) & 0xFFusize);
            idx += 1usize;
        }

        var key = Hex.Parse("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
        var iv = Hex.Parse("00112233445566778899aabbccddeeff");
        var encryptor = BuildEncryptor(key, iv);
        var ms = new MemoryStream();
        var cs = new CryptoStream(ms, encryptor, CryptoStreamMode.Write);

        var offset = 0usize;
        while (offset < NumericUnchecked.ToUSize(data.Length))
        {
            let chunk = 25usize;
            if (offset + chunk > NumericUnchecked.ToUSize(data.Length))
            {
                chunk = NumericUnchecked.ToUSize(data.Length) - offset;
            }
            cs.Write(ReadOnlySpan<byte>.FromArray(ref data).Slice(offset, chunk));
            offset += chunk;
        }
        cs.FlushFinalBlock();
        let cipher = ms.ToArray();
        cs.dispose(ref cs);
        ms.dispose(ref ms);

        var decryptor = BuildDecryptor(key, iv);
        var cipherStream = new MemoryStream(cipher, false);
        var decryptStream = new CryptoStream(cipherStream, decryptor, CryptoStreamMode.Read);
        var output = new byte[data.Length + 32];
        var total = 0usize;
        while (true)
        {
            let read = decryptStream.Read(Span<byte>.FromArray(ref output).Slice(
                total,
                NumericUnchecked.ToUSize(output.Length) - total
            ));
            if (read == 0)
            {
                break;
            }
            total += NumericUnchecked.ToUSize(read);
        }
        decryptStream.dispose(ref decryptStream);
        cipherStream.dispose(ref cipherStream);

        if (total != NumericUnchecked.ToUSize(data.Length))
        {
            return false;
        }
        return Matches(ReadOnlySpan<byte>.FromArray(ref output).Slice(0usize, total), ReadOnlySpan<byte>.FromArray(ref data));
    }

    private static bool AsyncCancellation()
    {
        var key = Hex.Parse("00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff");
        var iv = Hex.Parse("0f1e2d3c4b5a69788796a5b4c3d2e1f0");
        var encryptor = BuildEncryptor(key, iv);
        var ms = new MemoryStream();
        var cs = new CryptoStream(ms, encryptor, CryptoStreamMode.Write);
        var cts = CancellationTokenSource.Create();
        cts.Cancel();
        var buffer = new byte[1];
        var wrote = false;
        try
        {
            let _ = cs.WriteAsync(new ReadOnlyMemory<byte>(buffer), cts.Token());
            wrote = true;
        }
        catch (Std.TaskCanceledException)
        {
            wrote = false;
        }
        cs.dispose(ref cs);
        ms.dispose(ref ms);
        return !wrote;
    }

    private static ICryptoTransform BuildEncryptor(byte[] key, byte[] iv)
    {
        var aes = new AesAlgorithm();
        aes.Key = ReadOnlySpan<byte>.FromArray(ref key);
        aes.IV = ReadOnlySpan<byte>.FromArray(ref iv);
        aes.Padding = PaddingMode.PKCS7;
        return aes.CreateEncryptor();
    }

    private static ICryptoTransform BuildDecryptor(byte[] key, byte[] iv)
    {
        var aes = new AesAlgorithm();
        aes.Key = ReadOnlySpan<byte>.FromArray(ref key);
        aes.IV = ReadOnlySpan<byte>.FromArray(ref iv);
        aes.Padding = PaddingMode.PKCS7;
        return aes.CreateDecryptor();
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
}
