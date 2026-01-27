namespace Std;
import Std.Numeric;
import Std.Runtime;
import Std.Span;
import Std.Strings;
import Std.Text;
public static class Convert
{
    public static string ToBase64String(byte[] inArray) {
        return ToBase64String(inArray, Base64FormattingOptions.None);
    }
    public static string ToBase64String(byte[] inArray, Base64FormattingOptions options) {
        if (inArray == null)
        {
            throw new Std.ArgumentNullException("inArray");
        }
        return ToBase64String(inArray, 0, NumericUnchecked.ToInt32(inArray.Length), options);
    }
    public static string ToBase64String(byte[] inArray, int offset, int length) {
        return ToBase64String(inArray, offset, length, Base64FormattingOptions.None);
    }
    public static string ToBase64String(byte[] inArray, int offset, int length, Base64FormattingOptions options) {
        if (inArray == null)
        {
            throw new Std.ArgumentNullException("inArray");
        }
        ValidateRange(inArray.Length, offset, length, "offset", "length");
        let insertLineBreaks = ResolveFormatting(options);
        if (length == 0)
        {
            return StringRuntime.Create();
        }
        let start = NumericUnchecked.ToUSize(offset);
        let sliceLength = NumericUnchecked.ToUSize(length);
        let input = ReadOnlySpan <byte >.FromArray(in inArray).Slice(start, sliceLength);
        let requiredLength = Base64.GetEncodedLength(sliceLength, insertLineBreaks);
        var buffer = new byte[requiredLength];
        var written = 0usize;
        {
            let encoded = Span <byte >.FromArray(ref buffer);
            Base64.TryEncodeToBytes(input, encoded, out written, insertLineBreaks);
        }
        let result = ReadOnlySpan <byte >.FromArray(in buffer).Slice(0usize, written);
        return Utf8String.FromSpan(result);
    }
    public static string ToBase64String(ReadOnlySpan <byte >bytes, Base64FormattingOptions options = Base64FormattingOptions.None) {
        let insertLineBreaks = ResolveFormatting(options);
        if (bytes.Length == 0usize)
        {
            return StringRuntime.Create();
        }
        let requiredLength = Base64.GetEncodedLength(bytes.Length, insertLineBreaks);
        var buffer = new byte[requiredLength];
        var written = 0usize;
        {
            let encoded = Span <byte >.FromArray(ref buffer);
            Base64.TryEncodeToBytes(bytes, encoded, out written, insertLineBreaks);
        }
        let result = ReadOnlySpan <byte >.FromArray(in buffer).Slice(0usize, written);
        return Utf8String.FromSpan(result);
    }
    public static int ToBase64CharArray(byte[] inArray, int offsetIn, int length, char[] outArray, int offsetOut) {
        return ToBase64CharArray(inArray, offsetIn, length, outArray, offsetOut, Base64FormattingOptions.None);
    }
    public static int ToBase64CharArray(byte[] inArray, int offsetIn, int length, char[] outArray, int offsetOut, Base64FormattingOptions options) {
        if (inArray == null)
        {
            throw new Std.ArgumentNullException("inArray");
        }
        if (outArray == null)
        {
            throw new Std.ArgumentNullException("outArray");
        }
        ValidateRange(inArray.Length, offsetIn, length, "offsetIn", "length");
        if (offsetOut <0)
        {
            throw new Std.ArgumentOutOfRangeException("offsetOut");
        }
        let insertLineBreaks = ResolveFormatting(options);
        let inStart = NumericUnchecked.ToUSize(offsetIn);
        let inLength = NumericUnchecked.ToUSize(length);
        let outStart = NumericUnchecked.ToUSize(offsetOut);
        let requiredLength = Base64.GetEncodedLength(inLength, insertLineBreaks);
        if (outStart >outArray.Length || (outArray.Length - outStart) <requiredLength)
        {
            throw new Std.ArgumentOutOfRangeException("outArray");
        }
        let input = ReadOnlySpan <byte >.FromArray(in inArray).Slice(inStart, inLength);
        var outArrayRef = outArray;
        let destination = Span <char >.FromArray(ref outArrayRef).Slice(outStart, requiredLength);
        Base64.TryEncodeToChars(input, destination, out var written, insertLineBreaks);
        return NumericUnchecked.ToInt32(written);
    }
    public static bool TryToBase64Chars(ReadOnlySpan <byte >bytes, Span <char >chars, out int charsWritten, Base64FormattingOptions options = Base64FormattingOptions.None) {
        let insertLineBreaks = ResolveFormatting(options);
        let success = Base64.TryEncodeToChars(bytes, chars, out var written, insertLineBreaks);
        charsWritten = success ?NumericUnchecked.ToInt32(written) : 0;
        return success;
    }
    public static byte[] FromBase64String(string s) {
        if (s == null)
        {
            throw new Std.ArgumentNullException("s");
        }
        let input = s.AsSpan();
        if (! Base64.TryGetDecodedLength (input, out var decodedLength)) {
            throw new Std.FormatException("Input is not a valid Base64 string.");
        }
        var buffer = new byte[decodedLength];
        let destination = Span <byte >.FromArray(ref buffer);
        if (! Base64.TryDecode (input, destination, decodedLength, out var written) || written != decodedLength) {
            throw new Std.FormatException("Input is not a valid Base64 string.");
        }
        return buffer;
    }
    public static byte[] FromBase64CharArray(char[] inArray, int offset, int length) {
        if (inArray == null)
        {
            throw new Std.ArgumentNullException("inArray");
        }
        ValidateRange(inArray.Length, offset, length, "offset", "length");
        let start = NumericUnchecked.ToUSize(offset);
        let sliceLength = NumericUnchecked.ToUSize(length);
        let input = ReadOnlySpan <char >.FromArray(in inArray).Slice(start, sliceLength);
        if (! Base64.TryGetDecodedLength (input, out var decodedLength)) {
            throw new Std.FormatException("Input is not a valid Base64 char array.");
        }
        var buffer = new byte[decodedLength];
        let destination = Span <byte >.FromArray(ref buffer);
        if (! Base64.TryDecode (input, destination, decodedLength, out var written) || written != decodedLength) {
            throw new Std.FormatException("Input is not a valid Base64 char array.");
        }
        return buffer;
    }
    public static bool TryFromBase64Chars(ReadOnlySpan <char >chars, Span <byte >bytes, out int bytesWritten) {
        bytesWritten = 0;
        if (! Base64.TryGetDecodedLength (chars, out var decodedLength)) {
            return false;
        }
        if (bytes.Length <decodedLength)
        {
            return false;
        }
        if (! Base64.TryDecode (chars, bytes, decodedLength, out var written) || written != decodedLength) {
            return false;
        }
        bytesWritten = NumericUnchecked.ToInt32(written);
        return true;
    }
    private static void ValidateRange(usize arrayLength, int offset, int length, string offsetName, string lengthName) {
        if (offset <0)
        {
            throw new Std.ArgumentOutOfRangeException(offsetName);
        }
        if (length <0)
        {
            throw new Std.ArgumentOutOfRangeException(lengthName);
        }
        let start = NumericUnchecked.ToUSize(offset);
        let len = NumericUnchecked.ToUSize(length);
        if (start >arrayLength || (arrayLength - start) <len)
        {
            throw new Std.ArgumentOutOfRangeException(lengthName);
        }
    }
    private static bool ResolveFormatting(Base64FormattingOptions options) {
        if (options == Base64FormattingOptions.None)
        {
            return false;
        }
        if (options == Base64FormattingOptions.InsertLineBreaks)
        {
            return true;
        }
        throw new Std.ArgumentException("Invalid Base64FormattingOptions value.");
    }
}
