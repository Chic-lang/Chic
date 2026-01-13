namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>PBKDF2 key derivation using HMAC.</summary>
public static class Rfc2898DeriveBytes
{
    public static byte[] Pbkdf2(ReadOnlySpan <byte >password, ReadOnlySpan <byte >salt, int iterations, int keyBytes, HashAlgorithmName hash) {
        if (iterations <= 0)
        {
            throw new Std.ArgumentOutOfRangeException("iterations");
        }
        if (keyBytes <= 0)
        {
            throw new Std.ArgumentOutOfRangeException("keyBytes");
        }
        var digestSize = 0;
        let hmac = HmacFactory.Create(hash, out digestSize);
        hmac.SetKey(password);
        var result = new byte[keyBytes];
        var blockCount = (keyBytes + digestSize - 1) / digestSize;
        var blockIndex = 1;
        var resultOffset = 0usize;
        var saltBlock = new byte[NumericUnchecked.ToInt32(salt.Length + 4usize)];
        if (salt.Length >0usize)
        {
            Span <byte >.FromArray(ref saltBlock).Slice(0usize, salt.Length).CopyFrom(salt);
        }
        while (blockIndex <= blockCount)
        {
            var t = new byte[digestSize];
            var u = Span <byte >.StackAlloc(NumericUnchecked.ToUSize(digestSize));
            WriteInt32BigEndian(blockIndex, Span <byte >.FromArray(ref saltBlock).Slice(NumericUnchecked.ToUSize(salt.Length),
            4usize));
            hmac.Reset();
            hmac.Append(ReadOnlySpan <byte >.FromArray(ref saltBlock).Slice(0usize, salt.Length + 4usize));
            hmac.FinalizeHash(u);
            Span <byte >.FromArray(ref t).Slice(0usize, NumericUnchecked.ToUSize(digestSize)).CopyFrom(u);
            var iter = 1;
            while (iter <iterations)
            {
                hmac.Reset();
                hmac.Append(u.Slice(0usize, NumericUnchecked.ToUSize(digestSize)));
                hmac.FinalizeHash(u);
                XorInto(t, u);
                iter += 1;
            }
            var toCopy = digestSize;
            let remaining = keyBytes - NumericUnchecked.ToInt32(resultOffset);
            if (toCopy >remaining)
            {
                toCopy = remaining;
            }
            Span <byte >.FromArray(ref result).Slice(resultOffset, NumericUnchecked.ToUSize(toCopy)).CopyFrom(ReadOnlySpan <byte >.FromArray(ref t).Slice(0usize,
            NumericUnchecked.ToUSize(toCopy)));
            resultOffset += NumericUnchecked.ToUSize(toCopy);
            blockIndex += 1;
        }
        return result;
    }
    private static void XorInto(byte[] target, ReadOnlySpan <byte >source) {
        var len = NumericUnchecked.ToUSize(target.Length);
        var idx = 0usize;
        while (idx <len && idx <source.Length)
        {
            target[idx] = (byte)(target[idx] ^ source[idx]);
            idx += 1usize;
        }
    }
    private static void WriteInt32BigEndian(int value, Span <byte >destination) {
        destination[0] = NumericUnchecked.ToByte((value >> 24) & 0xFF);
        destination[1] = NumericUnchecked.ToByte((value >> 16) & 0xFF);
        destination[2] = NumericUnchecked.ToByte((value >> 8) & 0xFF);
        destination[3] = NumericUnchecked.ToByte(value & 0xFF);
    }
}
