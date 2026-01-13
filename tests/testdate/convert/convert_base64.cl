namespace Exec.ConvertBase64;

import Std;
import Std.Numeric;
import Std.Span;
import Std.Strings;

public static class Program
{
    public static int Main()
    {
        if (!RoundTripDeterministic())
        {
            return 1;
        }
        if (!LineBreaksRoundTrip())
        {
            return 2;
        }
        if (!WhitespaceDecode())
        {
            return 3;
        }
        if (!InvalidInputs())
        {
            return 4;
        }
        if (!OffsetAndLengthOverloads())
        {
            return 5;
        }
        if (!TrySpanSizing())
        {
            return 6;
        }
        if (!DeterministicEncoding())
        {
            return 7;
        }
        return 0;
    }

    private static bool RoundTripDeterministic()
    {
        var data = new byte[128];
        FillDeterministic(Span<byte>.FromArray(ref data));
        let encoded = Std.Convert.ToBase64String(ReadOnlySpan<byte>.FromArray(ref data));
        let decoded = Std.Convert.FromBase64String(encoded);
        Assert(Matches(ReadOnlySpan<byte>.FromArray(ref data), ReadOnlySpan<byte>.FromArray(ref decoded)), "roundtrip matches");
        return true;
    }

    private static bool LineBreaksRoundTrip()
    {
        var data = new byte[120];
        FillDeterministic(Span<byte>.FromArray(ref data));
        let encoded = Std.Convert.ToBase64String(
            ReadOnlySpan<byte>.FromArray(ref data),
            Base64FormattingOptions.InsertLineBreaks
        );
        let encodedArrayOverload = Std.Convert.ToBase64String(
            data,
            Base64FormattingOptions.InsertLineBreaks
        );
        Assert(encodedArrayOverload == encoded, "array overload matches span overload with line breaks");
        Assert(ValidateLineBreaks(encoded, 2), "line breaks inserted every 76 chars");
        let decoded = Std.Convert.FromBase64String(encoded);
        Assert(Matches(ReadOnlySpan<byte>.FromArray(ref data), ReadOnlySpan<byte>.FromArray(ref decoded)), "line break roundtrip");
        return true;
    }

    private static bool WhitespaceDecode()
    {
        let noisy = " AAE CAwQF\r\nBgcI\tCQoL DA0O Dw== ";
        var expected = new byte[16];
        FillSequence(Span<byte>.FromArray(ref expected));
        let decoded = Std.Convert.FromBase64String(noisy);
        Assert(Matches(ReadOnlySpan<byte>.FromArray(ref expected), ReadOnlySpan<byte>.FromArray(ref decoded)), "whitespace ignored");

        var chars = noisy.AsSpan();
        var buffer = new byte[expected.Length];
        let ok = Std.Convert.TryFromBase64Chars(chars, Span<byte>.FromArray(ref buffer), out var written);
        Assert(ok && written == NumericUnchecked.ToInt32(expected.Length), "TryFromBase64Chars succeeds with whitespace");
        Assert(Matches(ReadOnlySpan<byte>.FromArray(ref expected), ReadOnlySpan<byte>.FromArray(ref buffer)), "TryFromBase64Chars output matches");
        return true;
    }

    private static bool InvalidInputs()
    {
        Assert(ThrowsFormatException("AAA"), "length not multiple of four fails");
        Assert(ThrowsFormatException("AB=C"), "padding in wrong place fails");
        Assert(ThrowsFormatException("A==="), "too much padding fails");
        Assert(ThrowsFormatException("AB*C"), "invalid character fails");

        var bad = "AB=C".AsSpan();
        var buf = new byte[8];
        let ok = Std.Convert.TryFromBase64Chars(bad, Span<byte>.FromArray(ref buf), out var written);
        Assert(!ok && written == 0, "TryFromBase64Chars returns false for invalid input");
        return true;
    }

    private static bool OffsetAndLengthOverloads()
    {
        var data = new byte[] { 0xFFu8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8 };
        let encoded = Std.Convert.ToBase64String(data, 1, 3);
        Assert(encoded == "AQID", "subset encoding matches");

        var charBuf = new char[8];
        let charsWritten = Std.Convert.ToBase64CharArray(data, 1, 3, charBuf, 1);
        Assert(charsWritten == 4, "char array length");
        Assert(charBuf[1] == 'A' && charBuf[2] == 'Q' && charBuf[3] == 'I' && charBuf[4] == 'D', "char array contents");

        let decoded = Std.Convert.FromBase64CharArray(charBuf, 1, 4);
        Assert(decoded.Length == 3, "decoded length from char array");
        Assert(decoded[0] == 0x01u8 && decoded[1] == 0x02u8 && decoded[2] == 0x03u8, "decoded contents from char array");
        return true;
    }

    private static bool TrySpanSizing()
    {
        var data = new byte[6];
        FillSequence(Span<byte>.FromArray(ref data));
        var smallChars = new char[2];
        let ok = Std.Convert.TryToBase64Chars(ReadOnlySpan<byte>.FromArray(ref data), Span<char>.FromArray(ref smallChars), out var writtenSmall, Base64FormattingOptions.None);
        Assert(!ok && writtenSmall == 0, "TryToBase64Chars fails when destination is too small");

        var dest = new char[12];
        ok = Std.Convert.TryToBase64Chars(ReadOnlySpan<byte>.FromArray(ref data), Span<char>.FromArray(ref dest), out var written);
        Assert(ok && written == 8, "TryToBase64Chars writes expected length");

        var bytes = new byte[6];
        let encodedSpan = ReadOnlySpan<char>.FromArray(ref dest).Slice(0usize, NumericUnchecked.ToUSize((isize)written));
        ok = Std.Convert.TryFromBase64Chars(encodedSpan, Span<byte>.FromArray(ref bytes), out var bytesWritten);
        Assert(ok && bytesWritten == 6, "TryFromBase64Chars reads back expected bytes");
        Assert(Matches(ReadOnlySpan<byte>.FromArray(ref data), ReadOnlySpan<byte>.FromArray(ref bytes)), "TryFromBase64Chars content matches");
        return true;
    }

    private static bool DeterministicEncoding()
    {
        var sequence = new byte[16];
        FillSequence(Span<byte>.FromArray(ref sequence));
        let encoded = Std.Convert.ToBase64String(ReadOnlySpan<byte>.FromArray(ref sequence));
        Assert(encoded == "AAECAwQFBgcICQoLDA0ODw==", "encoding deterministic across backends");
        return true;
    }

    private static void FillDeterministic(Span<byte> buffer)
    {
        var state = 0x1234ABCDu;
        var idx = 0usize;
        while (idx < buffer.Length)
        {
            state = (state * 1664525u) + 1013904223u;
            buffer[idx] = NumericUnchecked.ToByte((state >> 16) & 0xFFu32);
            idx += 1usize;
        }
    }

    private static void FillSequence(Span<byte> buffer)
    {
        var idx = 0usize;
        var value = 0u8;
        while (idx < buffer.Length)
        {
            buffer[idx] = value;
            value = NumericUnchecked.ToByte(NumericUnchecked.ToUInt32(value) + 1u32);
            idx += 1usize;
        }
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

    private static bool ValidateLineBreaks(string encoded, int expectedBreaks)
    {
        let span = encoded.AsSpan();
        var idx = 0usize;
        var lineLen = 0usize;
        var breaks = 0;
        while (idx < span.Length)
        {
            let ch = span[idx];
            if (ch == '\r')
            {
                if ((idx + 1usize) >= span.Length || span[idx + 1usize] != '\n')
                {
                    return false;
                }
                if (lineLen != 76usize)
                {
                    return false;
                }
                breaks += 1;
                idx += 2usize;
                lineLen = 0usize;
                continue;
            }
            if (ch == '\n')
            {
                return false;
            }
            lineLen += 1usize;
            if (lineLen > 76usize)
            {
                return false;
            }
            idx += 1usize;
        }
        return breaks == expectedBreaks && lineLen > 0usize && lineLen <= 76usize;
    }

    private static bool ThrowsFormatException(string value)
    {
        var threw = false;
        try
        {
            let _ = Std.Convert.FromBase64String(value);
        }
        catch (Std.FormatException)
        {
            threw = true;
        }
        return threw;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException("Base64 test failed: " + message);
        }
    }
}
