namespace Std.Security.Cryptography;
import Std.Span;
import Std.Numeric;
/// <summary>HKDF key derivation (RFC 5869) with span-first APIs.</summary>
public static class HKDF
{
    public static int Extract(HashAlgorithmName hash, ReadOnlySpan <byte >ikm, ReadOnlySpan <byte >salt, Span <byte >destination) {
        let hmac = HmacFactory.Create(hash, out var digestSize);
        if (destination.Length <digestSize)
        {
            throw new Std.ArgumentException("destination too small for pseudorandom key");
        }
        var effectiveSalt = salt;
        if (effectiveSalt.Length == 0usize)
        {
            var zeros = new byte[digestSize];
            effectiveSalt = ReadOnlySpan <byte >.FromArray(ref zeros);
        }
        hmac.SetKey(effectiveSalt);
        hmac.Append(ikm);
        let written = hmac.FinalizeHash(destination);
        return written;
    }
    public static int Expand(HashAlgorithmName hash, ReadOnlySpan <byte >prk, ReadOnlySpan <byte >info, Span <byte >outputKeyMaterial) {
        let hmacTemplate = HmacFactory.Create(hash, out var digestSize);
        let digestSizeU = NumericUnchecked.ToUSize(digestSize);
        if (prk.Length <digestSizeU)
        {
            throw new Std.ArgumentException("prk too small");
        }
        if (outputKeyMaterial.Length == 0)
        {
            return 0;
        }
        let maxLength = NumericUnchecked.ToUSize(255 * digestSize);
        if (outputKeyMaterial.Length >maxLength)
        {
            throw new Std.ArgumentException("output length too large for HKDF");
        }
        var previous = new byte[digestSizeU];
        var counter = new byte[1];
        var offset = 0usize;
        var blockIndex = 1u8;
        var previousLength = 0usize;
        while (offset <outputKeyMaterial.Length)
        {
            let hmac = HmacFactory.Create(hash, out var _);
            hmac.SetKey(prk);
            if (previousLength >0usize)
            {
                hmac.Append(ReadOnlySpan <byte >.FromArray(ref previous).Slice(0usize, previousLength));
            }
            if (info.Length >0usize)
            {
                hmac.Append(info);
            }
            counter[0] = blockIndex;
            hmac.Append(ReadOnlySpan <byte >.FromArray(ref counter));
            var block = new byte[digestSizeU];
            let written = 0;
            {
                let blockSpan = Span <byte >.FromArray(ref block);
                written = hmac.FinalizeHash(blockSpan);
            }
            let blockValid = NumericUnchecked.ToUSize(written);
            let toCopy = digestSizeU;
            let remaining = outputKeyMaterial.Length - offset;
            if (toCopy >remaining)
            {
                toCopy = remaining;
            }
            if (blockValid <toCopy)
            {
                toCopy = blockValid;
            }
            var copyIdx = 0usize;
            while (copyIdx <toCopy)
            {
                outputKeyMaterial[offset + copyIdx] = block[copyIdx];
                copyIdx += 1usize;
            }
            offset += toCopy;
            {
                var prevIdx = 0usize;
                while (prevIdx <blockValid)
                {
                    previous[prevIdx] = block[prevIdx];
                    prevIdx += 1usize;
                }
            }
            previousLength = NumericUnchecked.ToUSize(written);
            blockIndex = (byte)(blockIndex + 1u8);
        }
        return NumericUnchecked.ToInt32(offset);
    }
    public static byte[] DeriveKey(HashAlgorithmName hash, ReadOnlySpan <byte >ikm, ReadOnlySpan <byte >salt, ReadOnlySpan <byte >info,
    int length) {
        if (length <0)
        {
            throw new Std.ArgumentOutOfRangeException("length");
        }
        if (length == 0)
        {
            let empty = 0;
            return new byte[empty];
        }
        let hmac = HmacFactory.Create(hash, out var digestSize);
        var prk = new byte[digestSize];
        let prkWritten = Extract(hash, ikm, salt, Span <byte >.FromArray(ref prk));
        var okm = new byte[length];
        Expand(hash, ReadOnlySpan <byte >.FromArray(ref prk).Slice(0usize, NumericUnchecked.ToUSize(prkWritten)), info, Span <byte >.FromArray(ref okm));
        return okm;
    }
}
