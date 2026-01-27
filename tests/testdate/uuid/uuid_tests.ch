namespace Exec;

import Std;
import Std.Numeric;
import Std.Span;
import Std.Strings;

public static class Program
{
    public static int Main()
    {
        if (!TestNewUuidBits())
        {
            return 1;
        }
        if (!TestParseAndFormat())
        {
            return 2;
        }
        if (!TestInvalidInputs())
        {
            return 3;
        }
        if (!TestSpanFormatting())
        {
            return 4;
        }
        if (!TestByteLayout())
        {
            return 5;
        }
        return 0;
    }

    private static bool TestNewUuidBits()
    {
        var seen = new Uuid[12];
        var idx = 0usize;
        while (idx < seen.Length)
        {
            let id = Uuid.NewUuid();
            var bytes = Span<byte>.StackAlloc(16usize);
            id.WriteBytes(bytes);
            if ((bytes[6usize] & 0xF0u8) != 0x40u8)
            {
                return false;
            }
            if ((bytes[8usize] & 0xC0u8) != 0x80u8)
            {
                return false;
            }
            var dedup = 0usize;
            while (dedup < idx)
            {
                if (seen[dedup] == id)
                {
                    return false;
                }
                dedup += 1usize;
            }
            seen[idx] = id;
            idx += 1usize;
        }
        return true;
    }

    private static bool TestParseAndFormat()
    {
        let text = "00112233-4455-6677-8899-aabbccddeeff";
        let parsed = Uuid.Parse(text);
        if (parsed.ToString() != text)
        {
            return false;
        }
        if (parsed.ToString("N") != "00112233445566778899aabbccddeeff")
        {
            return false;
        }
        if (parsed.ToString("B") != "{00112233-4455-6677-8899-aabbccddeeff}")
        {
            return false;
        }
        if (parsed.ToString("P") != "(00112233-4455-6677-8899-aabbccddeeff)")
        {
            return false;
        }

        if (!Uuid.TryParse(text, out var tryParsed))
        {
            return false;
        }
        if (tryParsed != parsed)
        {
            return false;
        }
        if (!Uuid.TryParseExact("00112233445566778899aabbccddeeff", "N", out var nParsed))
        {
            return false;
        }
        if (!Uuid.TryParseExact("{00112233-4455-6677-8899-aabbccddeeff}", "B", out var bParsed))
        {
            return false;
        }
        return nParsed == parsed && bParsed == parsed;
    }

    private static bool TestInvalidInputs()
    {
        if (Uuid.TryParse("not-a-uuid", out var _))
        {
            return false;
        }
        if (Uuid.TryParse("00112233-4455-6677-8899-aabbccddeef", out var _))
        {
            return false;
        }
        if (Uuid.TryParseExact("00112233445566778899aabbccddeefg", "N", out var _))
        {
            return false;
        }
        if (Uuid.TryParseExact("(00112233-4455-6677-8899-aabbccddeeff)", "B", out var _))
        {
            return false;
        }
        return true;
    }

    private static bool TestSpanFormatting()
    {
        let parsed = Uuid.Parse("00112233-4455-6677-8899-aabbccddeeff");
        var buffer = Span<char>.StackAlloc(36usize);
        if (!parsed.TryFormat(buffer, out var written, "D"))
        {
            return false;
        }
        if (written != 36)
        {
            return false;
        }
        let expected = "00112233-4455-6677-8899-aabbccddeeff";
        let expectedSpan = expected.AsSpan();
        var ro = buffer.AsReadOnly();
        var idx = 0usize;
        while (idx < expectedSpan.Length)
        {
            if (ro[idx] != expectedSpan[idx])
            {
                return false;
            }
            idx += 1usize;
        }

        var small = Span<char>.StackAlloc(10usize);
        if (parsed.TryFormat(small, out var _, "D"))
        {
            return false;
        }
        return true;
    }

    private static bool TestByteLayout()
    {
        var bytes = new byte[]
        {
            0x00u8, 0x11u8, 0x22u8, 0x33u8,
            0x44u8, 0x55u8, 0x66u8, 0x77u8,
            0x88u8, 0x99u8, 0xaau8, 0xbbu8,
            0xccu8, 0xddu8, 0xeeu8, 0xffu8
        };
        let span = ReadOnlySpan<byte>.FromArray(in bytes);
        var id = new Uuid(span);
        var roundtrip = Span<byte>.StackAlloc(16usize);
        id.WriteBytes(roundtrip);
        let rt = roundtrip.AsReadOnly();
        var idx = 0usize;
        while (idx < span.Length)
        {
            if (rt[idx] != span[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        return id.ToString() == "00112233-4455-6677-8899-aabbccddeeff";
    }
}
