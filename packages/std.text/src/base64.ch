namespace Std.Text;
import Std.Numeric;
import Std.Span;
import Std.Testing;
/// <summary>Shared Base64 encoder/decoder with .NET-compatible semantics.</summary>
internal static class Base64
{
    private const usize LineBreakSize = 76usize;
    private static byte[] InitEncodeMap() {
        let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let chars = alphabet.AsSpan();
        var map = new byte[NumericUnchecked.ToUSize(chars.Length)];
        var idx = 0usize;
        while (idx <map.Length)
        {
            map[idx] = NumericUnchecked.ToByte(chars[idx]);
            idx += 1usize;
        }
        return map;
    }
    private static sbyte[] CreateDecodeMap(ReadOnlySpan <byte >encodeMap) {
        var map = new sbyte[256];
        var idx = 0usize;
        while (idx <map.Length)
        {
            map[idx] = - 1;
            idx += 1usize;
        }
        idx = 0usize;
        while (idx <encodeMap.Length)
        {
            let tableValue = encodeMap[idx];
            let slot = NumericUnchecked.ToUSize(tableValue);
            map[slot] = NumericUnchecked.ToSByte(idx);
            idx += 1usize;
        }
        return map;
    }
    private static bool IsWhitespace(char value) {
        return value == ' ' || value == '\t' || value == '\r' || value == '\n';
    }
    public static usize GetEncodedLength(usize inputLength, bool insertLineBreaks) {
        let baseLength = ((inputLength + 2usize) / 3usize) * 4usize;
        if (!insertLineBreaks || baseLength <= LineBreakSize)
        {
            return baseLength;
        }
        let fullLines = (baseLength - 1usize) / LineBreakSize;
        return baseLength + (fullLines * 2usize);
    }
    public static bool TryEncodeToBytes(ReadOnlySpan <byte >bytes, Span <byte >destination, out usize written, bool insertLineBreaks) {
        let encodeMap = InitEncodeMap();
        let requiredLength = GetEncodedLength(bytes.Length, insertLineBreaks);
        if (destination.Length <requiredLength)
        {
            written = 0usize;
            return false;
        }
        written = EncodeToBytes(bytes, encodeMap, destination, insertLineBreaks);
        return true;
    }
    public static bool TryEncodeToChars(ReadOnlySpan <byte >bytes, Span <char >destination, out usize written, bool insertLineBreaks) {
        let encodeMap = InitEncodeMap();
        let requiredLength = GetEncodedLength(bytes.Length, insertLineBreaks);
        if (destination.Length <requiredLength)
        {
            written = 0usize;
            return false;
        }
        written = EncodeToChars(bytes, encodeMap, destination, insertLineBreaks);
        return true;
    }
    private static usize EncodeToBytes(ReadOnlySpan <byte >bytes, ReadOnlySpan <byte >encodeMap, Span <byte >destination,
    bool insertLineBreaks) {
        var outIdx = 0usize;
        var lineCount = 0usize;
        var idx = 0usize;
        while (idx + 3usize <= bytes.Length)
        {
            if (insertLineBreaks && lineCount == LineBreakSize)
            {
                destination[outIdx] = 13u8;
                destination[outIdx + 1usize] = 10u8;
                outIdx += 2usize;
                lineCount = 0usize;
            }
            let b0 = bytes[idx];
            let b1 = bytes[idx + 1usize];
            let b2 = bytes[idx + 2usize];
            destination[outIdx] = encodeMap[NumericUnchecked.ToUSize(NumericUnchecked.ToUInt32(b0) >> 2)];
            destination[outIdx + 1usize] = encodeMap[NumericUnchecked.ToUSize(((b0 & 0x03u8) << 4) | (b1 >> 4))];
            destination[outIdx + 2usize] = encodeMap[NumericUnchecked.ToUSize(((b1 & 0x0Fu8) << 2) | (b2 >> 6))];
            destination[outIdx + 3usize] = encodeMap[NumericUnchecked.ToUSize(b2 & 0x3Fu8)];
            outIdx += 4usize;
            lineCount += 4usize;
            idx += 3usize;
        }
        let remaining = bytes.Length - idx;
        if (remaining >0usize)
        {
            if (insertLineBreaks && lineCount == LineBreakSize)
            {
                destination[outIdx] = 13u8;
                destination[outIdx + 1usize] = 10u8;
                outIdx += 2usize;
                lineCount = 0usize;
            }
            let b0 = bytes[idx];
            let b1 = remaining == 2usize ?bytes[idx + 1usize] : 0u8;
            destination[outIdx] = encodeMap[NumericUnchecked.ToUSize(NumericUnchecked.ToUInt32(b0) >> 2)];
            destination[outIdx + 1usize] = encodeMap[NumericUnchecked.ToUSize(((b0 & 0x03u8) << 4) | (b1 >> 4))];
            if (remaining == 2usize)
            {
                destination[outIdx + 2usize] = encodeMap[NumericUnchecked.ToUSize((b1 & 0x0Fu8) << 2)];
                destination[outIdx + 3usize] = 61u8;
            }
            else
            {
                destination[outIdx + 2usize] = 61u8;
                destination[outIdx + 3usize] = 61u8;
            }
            outIdx += 4usize;
            lineCount += 4usize;
        }
        return outIdx;
    }
    private static usize EncodeToChars(ReadOnlySpan <byte >bytes, ReadOnlySpan <byte >encodeMap, Span <char >destination,
    bool insertLineBreaks) {
        var outIdx = 0usize;
        var lineCount = 0usize;
        var idx = 0usize;
        while (idx + 3usize <= bytes.Length)
        {
            if (insertLineBreaks && lineCount == LineBreakSize)
            {
                destination[outIdx] = '\r';
                destination[outIdx + 1usize] = '\n';
                outIdx += 2usize;
                lineCount = 0usize;
            }
            let b0 = bytes[idx];
            let b1 = bytes[idx + 1usize];
            let b2 = bytes[idx + 2usize];
            destination[outIdx] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize(NumericUnchecked.ToUInt32(b0) >> 2)]);
            destination[outIdx + 1usize] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize(((b0 & 0x03u8) << 4) | (b1 >> 4))]);
            destination[outIdx + 2usize] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize(((b1 & 0x0Fu8) << 2) | (b2 >> 6))]);
            destination[outIdx + 3usize] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize(b2 & 0x3Fu8)]);
            outIdx += 4usize;
            lineCount += 4usize;
            idx += 3usize;
        }
        let remaining = bytes.Length - idx;
        if (remaining >0usize)
        {
            if (insertLineBreaks && lineCount == LineBreakSize)
            {
                destination[outIdx] = '\r';
                destination[outIdx + 1usize] = '\n';
                outIdx += 2usize;
                lineCount = 0usize;
            }
            let b0 = bytes[idx];
            let b1 = remaining == 2usize ?bytes[idx + 1usize] : 0u8;
            destination[outIdx] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize(NumericUnchecked.ToUInt32(b0) >> 2)]);
            destination[outIdx + 1usize] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize(((b0 & 0x03u8) << 4) | (b1 >> 4))]);
            if (remaining == 2usize)
            {
                destination[outIdx + 2usize] = NumericUnchecked.ToChar(encodeMap[NumericUnchecked.ToUSize((b1 & 0x0Fu8) << 2)]);
                destination[outIdx + 3usize] = '=';
            }
            else
            {
                destination[outIdx + 2usize] = '=';
                destination[outIdx + 3usize] = '=';
            }
            outIdx += 4usize;
            lineCount += 4usize;
        }
        return outIdx;
    }
    public static bool TryGetDecodedLength(ReadOnlySpan <char >chars, out usize decodedLength) {
        let decodeMap = CreateDecodeMap(InitEncodeMap());
        return TryGetDecodedLength(chars, decodeMap, out decodedLength);
    }
    private static bool TryGetDecodedLength(ReadOnlySpan <char >chars, ReadOnlySpan <sbyte >decodeMap, out usize decodedLength) {
        decodedLength = 0usize;
        var trimmed = 0usize;
        var paddingCount = 0usize;
        var paddingStart = - 1;
        var quartetIndex = 0usize;
        var idx = 0usize;
        while (idx <chars.Length)
        {
            let current = chars[idx];
            idx += 1usize;
            if (IsWhitespace (current))
            {
                continue;
            }
            let scalar = NumericUnchecked.ToInt32(current);
            if (scalar <0 || scalar >255)
            {
                decodedLength = 0usize;
                return false;
            }
            if (current == '=')
            {
                if (quartetIndex <2usize)
                {
                    decodedLength = 0usize;
                    return false;
                }
                if (paddingStart == - 1)
                {
                    paddingStart = NumericUnchecked.ToInt32(quartetIndex);
                }
                paddingCount += 1usize;
                if (paddingCount >2usize)
                {
                    decodedLength = 0usize;
                    return false;
                }
            }
            else
            {
                let value = decodeMap[NumericUnchecked.ToUSize(scalar)];
                if (value <0 || paddingCount != 0usize)
                {
                    decodedLength = 0usize;
                    return false;
                }
            }
            quartetIndex += 1usize;
            if (quartetIndex == 4usize)
            {
                quartetIndex = 0usize;
            }
            trimmed += 1usize;
        }
        if ( (trimmed % 4usize) != 0usize)
        {
            decodedLength = 0usize;
            return false;
        }
        if (paddingCount == 1usize && paddingStart != 3)
        {
            decodedLength = 0usize;
            return false;
        }
        if (paddingCount == 2usize && paddingStart != 2)
        {
            decodedLength = 0usize;
            return false;
        }
        decodedLength = (trimmed / 4usize) * 3usize;
        decodedLength -= paddingCount;
        return true;
    }
    public static bool TryDecode(ReadOnlySpan <char >chars, Span <byte >bytes, out usize written) {
        let encodeMap = InitEncodeMap();
        let decodeMap = CreateDecodeMap(encodeMap);
        if (!TryGetDecodedLength (chars, decodeMap, out var decodedLength)) {
            written = 0usize;
            return false;
        }
        return TryDecode(chars, decodeMap, bytes, decodedLength, out written);
    }
    public static bool TryDecode(ReadOnlySpan <char >chars, Span <byte >bytes, usize decodedLength, out usize written) {
        let decodeMap = CreateDecodeMap(InitEncodeMap());
        return TryDecode(chars, decodeMap, bytes, decodedLength, out written);
    }
    private static bool TryDecode(ReadOnlySpan <char >chars, ReadOnlySpan <sbyte >decodeMap, Span <byte >bytes, usize decodedLength,
    out usize written) {
        written = 0usize;
        if (decodedLength == 0usize)
        {
            return true;
        }
        if (bytes.Length <decodedLength)
        {
            return false;
        }
        var quartet = 0u32;
        var quartetIndex = 0usize;
        var idx = 0usize;
        while (idx <chars.Length)
        {
            let current = chars[idx];
            idx += 1usize;
            if (IsWhitespace (current))
            {
                continue;
            }
            if (current == '=')
            {
                if (quartetIndex == 2usize)
                {
                    quartet <<= 12;
                    bytes[written] = NumericUnchecked.ToByte((quartet >> 16) & 0xFFu32);
                    written += 1usize;
                }
                else if (quartetIndex == 3usize)
                {
                    quartet <<= 6;
                    bytes[written] = NumericUnchecked.ToByte((quartet >> 16) & 0xFFu32);
                    bytes[written + 1usize] = NumericUnchecked.ToByte((quartet >> 8) & 0xFFu32);
                    written += 2usize;
                }
                else
                {
                    written = 0usize;
                    return false;
                }
                break;
            }
            let scalar = NumericUnchecked.ToInt32(current);
            if (scalar <0 || scalar >255)
            {
                written = 0usize;
                return false;
            }
            let value = decodeMap[NumericUnchecked.ToUSize(scalar)];
            if (value <0)
            {
                written = 0usize;
                return false;
            }
            quartet = (quartet << 6) | NumericUnchecked.ToUInt32(NumericUnchecked.ToByte(value));
            quartetIndex += 1usize;
            if (quartetIndex == 4usize)
            {
                bytes[written] = NumericUnchecked.ToByte((quartet >> 16) & 0xFFu32);
                bytes[written + 1usize] = NumericUnchecked.ToByte((quartet >> 8) & 0xFFu32);
                bytes[written + 2usize] = NumericUnchecked.ToByte(quartet & 0xFFu32);
                written += 3usize;
                quartet = 0u32;
                quartetIndex = 0usize;
                if (written == decodedLength)
                {
                    break;
                }
            }
        }
        return written == decodedLength;
    }
}
testcase Given_base64_encode_decode_roundtrip_encode_ok_When_executed_Then_base64_encode_decode_roundtrip_encode_ok()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let ok = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    Assert.That(ok).IsTrue();
}
testcase Given_base64_encode_decode_roundtrip_written_length_When_executed_Then_base64_encode_decode_roundtrip_written_length()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    Assert.That(written).IsEqualTo(encodedLength);
}
testcase Given_base64_encode_decode_roundtrip_decode_ok_When_executed_Then_base64_encode_decode_roundtrip_decode_ok()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    var decoded = Span <byte >.StackAlloc(input.Length);
    let decodeOk = Base64.TryDecode(encodedChars.AsReadOnly(), decoded, out var decodedWritten);
    let _ = decodedWritten;
    Assert.That(decodeOk).IsTrue();
}
testcase Given_base64_encode_decode_roundtrip_decoded_length_When_executed_Then_base64_encode_decode_roundtrip_decoded_length()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    var decoded = Span <byte >.StackAlloc(input.Length);
    let _ = Base64.TryDecode(encodedChars.AsReadOnly(), decoded, out var decodedWritten);
    Assert.That(decodedWritten).IsEqualTo(input.Length);
}
testcase Given_base64_encode_decode_roundtrip_payload_matches_When_executed_Then_base64_encode_decode_roundtrip_payload_matches()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    var decoded = Span <byte >.StackAlloc(input.Length);
    let _ = Base64.TryDecode(encodedChars.AsReadOnly(), decoded, out var decodedWritten);
    let _ = decodedWritten;
    Assert.That(decoded.AsReadOnly()).IsEqualTo(input);
}
testcase Given_base64_handles_padding_encode_ok_When_executed_Then_base64_handles_padding_encode_ok()
{
    let input = ReadOnlySpan.FromString("f");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let ok = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    Assert.That(ok).IsTrue();
}
testcase Given_base64_handles_padding_has_padding_chars_When_executed_Then_base64_handles_padding_has_padding_chars()
{
    let input = ReadOnlySpan.FromString("f");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    let padded = encodedChars[2usize] == '=' && encodedChars[3usize] == '=';
    Assert.That(padded).IsTrue();
}
testcase Given_base64_handles_padding_decode_ok_When_executed_Then_base64_handles_padding_decode_ok()
{
    let input = ReadOnlySpan.FromString("f");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    var decoded = Span <byte >.StackAlloc(1usize);
    let decodeOk = Base64.TryDecode(encodedChars.AsReadOnly(), decoded, out var decodedWritten);
    let _ = decodedWritten;
    Assert.That(decodeOk).IsTrue();
}
testcase Given_base64_handles_padding_decoded_length_When_executed_Then_base64_handles_padding_decoded_length()
{
    let input = ReadOnlySpan.FromString("f");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    var decoded = Span <byte >.StackAlloc(1usize);
    let _ = Base64.TryDecode(encodedChars.AsReadOnly(), decoded, out var decodedWritten);
    Assert.That(decodedWritten).IsEqualTo(1usize);
}
testcase Given_base64_handles_padding_roundtrip_When_executed_Then_base64_handles_padding_roundtrip()
{
    let input = ReadOnlySpan.FromString("f");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength);
    let _ = Base64.TryEncodeToChars(input, encodedChars, out var written, false);
    let _ = written;
    var decoded = Span <byte >.StackAlloc(1usize);
    let _ = Base64.TryDecode(encodedChars.AsReadOnly(), decoded, out var decodedWritten);
    let _ = decodedWritten;
    Assert.That(decoded[0usize]).IsEqualTo(input[0usize]);
}
testcase Given_base64_rejects_invalid_length_ok_false_When_executed_Then_base64_rejects_invalid_length_ok_false()
{
    let chars = ReadOnlySpan.FromStringChars("abc");
    let ok = Base64.TryGetDecodedLength(chars, out var decodedLength);
    let _ = decodedLength;
    Assert.That(ok).IsFalse();
}
testcase Given_base64_rejects_invalid_length_zero_When_executed_Then_base64_rejects_invalid_length_zero()
{
    let chars = ReadOnlySpan.FromStringChars("abc");
    let _ = Base64.TryGetDecodedLength(chars, out var decodedLength);
    Assert.That(decodedLength).IsEqualTo(0usize);
}
testcase Given_base64_line_break_length_When_executed_Then_base64_line_break_length()
{
    let input = ReadOnlySpan.FromString("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ12345678");
    let encodedLength = Base64.GetEncodedLength(input.Length, true);
    Assert.That(encodedLength).IsEqualTo(82usize);
}
testcase Given_base64_decodes_with_whitespace_encode_ok_When_executed_Then_base64_decodes_with_whitespace_encode_ok()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength + 2usize);
    let ok = Base64.TryEncodeToChars(input, encodedChars.Slice(0, encodedLength), out var written, false);
    let _ = written;
    Assert.That(ok).IsTrue();
}
testcase Given_base64_decodes_with_whitespace_decode_ok_When_executed_Then_base64_decodes_with_whitespace_decode_ok()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength + 2usize);
    let _ = Base64.TryEncodeToChars(input, encodedChars.Slice(0, encodedLength), out var written, false);
    let _ = written;
    encodedChars[encodedLength] = '\r';
    encodedChars[encodedLength + 1usize] = '\n';
    let withWhitespace = encodedChars.Slice(0, encodedLength + 2usize).AsReadOnly();
    var decoded = Span <byte >.StackAlloc(input.Length);
    let decodeOk = Base64.TryDecode(withWhitespace, decoded, out var decodedWritten);
    let _ = decodedWritten;
    Assert.That(decodeOk).IsTrue();
}
testcase Given_base64_decodes_with_whitespace_decoded_length_When_executed_Then_base64_decodes_with_whitespace_decoded_length()
{
    let input = ReadOnlySpan.FromString("hello");
    let encodedLength = Base64.GetEncodedLength(input.Length, false);
    var encodedChars = Span <char >.StackAlloc(encodedLength + 2usize);
    let _ = Base64.TryEncodeToChars(input, encodedChars.Slice(0, encodedLength), out var written, false);
    let _ = written;
    encodedChars[encodedLength] = '\r';
    encodedChars[encodedLength + 1usize] = '\n';
    let withWhitespace = encodedChars.Slice(0, encodedLength + 2usize).AsReadOnly();
    var decoded = Span <byte >.StackAlloc(input.Length);
    let _ = Base64.TryDecode(withWhitespace, decoded, out var decodedWritten);
    Assert.That(decodedWritten).IsEqualTo(input.Length);
}
testcase Given_base64_rejects_invalid_character_ok_false_When_executed_Then_base64_rejects_invalid_character_ok_false()
{
    let chars = ReadOnlySpan.FromStringChars("AA@=");
    let ok = Base64.TryDecode(chars, Span <byte >.StackAlloc(4usize), out var written);
    let _ = written;
    Assert.That(ok).IsFalse();
}
testcase Given_base64_rejects_invalid_character_written_zero_When_executed_Then_base64_rejects_invalid_character_written_zero()
{
    let chars = ReadOnlySpan.FromStringChars("AA@=");
    let _ = Base64.TryDecode(chars, Span <byte >.StackAlloc(4usize), out var written);
    Assert.That(written).IsEqualTo(0usize);
}
