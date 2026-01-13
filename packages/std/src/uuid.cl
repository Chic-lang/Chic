namespace Std;
import Std.Numeric;
import Std.Span;
import Std.Strings;
import Std.Runtime.Collections;
internal enum UuidFormat
{
    D = 0, N = 1, B = 2, P = 3,
}
/// <summary>Represents a 128-bit RFC 4122 UUID (v4 supported for generation).</summary>
public readonly struct Uuid : IEquatable <Uuid >, Clone, Copy
{
    private const usize ByteLength = 16usize;
    private readonly ulong _high;
    private readonly ulong _low;
    private init(ulong high, ulong low) {
        _high = high;
        _low = low;
    }
    /// <summary>Gets the all-zero UUID value.</summary>
    public static Uuid Empty => new Uuid(0ul, 0ul);
    /// <summary>Creates a new random UUID v4 using a cryptographically secure RNG.</summary>
    public static Uuid NewUuid() {
        var bytes = Span <byte >.StackAlloc(ByteLength);
        FillRandom(bytes);
        bytes[6usize] = NumericUnchecked.ToByte((bytes[6usize] & 0x0Fu8) | 0x40u8);
        bytes[8usize] = NumericUnchecked.ToByte((bytes[8usize] & 0x3Fu8) | 0x80u8);
        return FromRfcBytes(bytes.AsReadOnly());
    }
    /// <summary>Initializes a UUID from a 16-byte buffer using RFC 4122 byte order.</summary>
    public init(byte[] bytes) {
        if (bytes == null)
        {
            throw new Std.ArgumentNullException(nameof(bytes));
        }
        let span = ReadOnlySpan <byte >.FromArray(in bytes);
        if (span.Length != ByteLength)
        {
            throw new Std.ArgumentException("bytes must be exactly 16 bytes");
        }
        Pack(span, out var high, out var low);
        _high = high;
        _low = low;
    }
    /// <summary>Initializes a UUID from a 16-byte read-only span using RFC 4122 byte order.</summary>
    public init(ReadOnlySpan <byte >bytes) {
        if (bytes.Length != ByteLength)
        {
            throw new Std.ArgumentException("bytes must be exactly 16 bytes");
        }
        Pack(bytes, out var high, out var low);
        _high = high;
        _low = low;
    }
    /// <summary>Initializes a UUID from individual fields in RFC 4122 order.</summary>
    public init(uint a, ushort b, ushort c, byte d, byte e, byte f, byte g, byte h, byte i, byte j, byte k) {
        var buffer = Span <byte >.StackAlloc(ByteLength);
        WriteUInt32BigEndian(buffer, 0usize, a);
        WriteUInt16BigEndian(buffer, 4usize, b);
        WriteUInt16BigEndian(buffer, 6usize, c);
        buffer[8usize] = d;
        buffer[9usize] = e;
        buffer[10usize] = f;
        buffer[11usize] = g;
        buffer[12usize] = h;
        buffer[13usize] = i;
        buffer[14usize] = j;
        buffer[15usize] = k;
        Pack(buffer.AsReadOnly(), out var high, out var low);
        _high = high;
        _low = low;
    }
    /// <summary>Writes the UUID bytes in RFC 4122 order to the destination span.</summary>
    public void WriteBytes(Span <byte >destination) {
        if (destination.Length <ByteLength)
        {
            throw new Std.ArgumentException("destination must be at least 16 bytes");
        }
        WriteRfcBytes(destination);
    }
    /// <summary>Attempts to write the UUID bytes in RFC 4122 order to the destination span.</summary>
    public bool TryWriteBytes(Span <byte >destination) {
        if (destination.Length <ByteLength)
        {
            return false;
        }
        WriteRfcBytes(destination);
        return true;
    }
    /// <summary>Creates a new array containing the UUID bytes in RFC 4122 order.</summary>
    public byte[] ToByteArray() {
        var buffer = new byte[NumericUnchecked.ToInt32(ByteLength)];
        let span = Span <byte >.FromArray(ref buffer);
        WriteRfcBytes(span);
        return buffer;
    }
    /// <summary>Formats the UUID using the default "D" format.</summary>
    public override string ToString() {
        return ToString(null);
    }
    /// <summary>Formats the UUID using the specified format specifier ("D", "N", "B", or "P").</summary>
    public string ToString(string format) {
        let resolved = ResolveFormat(format, true);
        let requiredLength = RequiredLength(resolved);
        var bytes = Span <byte >.StackAlloc(requiredLength);
        FormatAscii(bytes, resolved);
        return Utf8String.FromSpan(bytes.AsReadOnly());
    }
    /// <summary>Formats the UUID into a destination character span without allocations.</summary>
    public bool TryFormat(Span <char >destination, out int charsWritten, string ?format = null) {
        charsWritten = 0;
        let resolvedOpt = ResolveFormatOptional(format, true);
        if (! resolvedOpt.IsSome (out var resolved)) {
            return false;
        }
        let requiredLength = RequiredLength(resolved);
        if (destination.Length <requiredLength)
        {
            return false;
        }
        var bytes = Span <byte >.StackAlloc(requiredLength);
        FormatAscii(bytes, resolved);
        var idx = 0usize;
        while (idx <requiredLength)
        {
            destination[idx] = NumericUnchecked.ToChar(NumericUnchecked.ToInt64(bytes[idx]));
            idx += 1usize;
        }
        charsWritten = NumericUnchecked.ToInt32((long) requiredLength);
        return true;
    }
    /// <summary>Parses the provided string into a UUID using supported formats.</summary>
    public static Uuid Parse(string input) {
        if (input == null)
        {
            throw new Std.ArgumentNullException(nameof(input));
        }
        if (TryParse (input, out var value)) {
            return value;
        }
        throw new Std.FormatException("Input string was not recognized as a valid UUID.");
    }
    /// <summary>Attempts to parse the provided string into a UUID using supported formats.</summary>
    public static bool TryParse(string input, out Uuid value) {
        if (input == null)
        {
            value = Empty;
            return false;
        }
        let chars = Std.Span.ReadOnlySpan.FromStringChars(input);
        return TryParseCore(chars, Std.Option <UuidFormat >.None(), out value);
    }
    /// <summary>Parses a string using an exact format specifier.</summary>
    public static Uuid ParseExact(string input, string format) {
        if (input == null)
        {
            throw new Std.ArgumentNullException(nameof(input));
        }
        if (format == null)
        {
            throw new Std.ArgumentNullException(nameof(format));
        }
        let resolvedOpt = ResolveFormatOptional(format, false);
        if (resolvedOpt.IsSome (out var resolved) && TryParseCore(Std.Span.ReadOnlySpan.FromStringChars(input), Std.Option <UuidFormat >.Some(resolved),
        out var value)) {
            return value;
        }
        throw new Std.FormatException("Input string was not recognized as a valid UUID.");
    }
    /// <summary>Attempts to parse a string using an exact format specifier.</summary>
    public static bool TryParseExact(string input, string format, out Uuid value) {
        value = Empty;
        if (input == null || format == null)
        {
            return false;
        }
        let resolvedOpt = ResolveFormatOptional(format, false);
        if (! resolvedOpt.IsSome (out var resolved)) {
            return false;
        }
        let chars = Std.Span.ReadOnlySpan.FromStringChars(input);
        return TryParseCore(chars, Std.Option <UuidFormat >.Some(resolved), out value);
    }
    /// <summary>Checks for equality against another UUID.</summary>
    public bool Equals(Uuid other) {
        return _high == other._high && _low == other._low;
    }
    /// <summary>Checks for equality against another object.</summary>
    public override bool Equals(Object obj) {
        return false;
    }
    /// <summary>Produces a hash code based on the UUID value.</summary>
    public override int GetHashCode() {
        unchecked {
            let mixed = _high ^ _low;
            return NumericUnchecked.ToInt32((long) mixed) ^ NumericUnchecked.ToInt32(NumericUnchecked.ToInt64(mixed >> 32));
        }
    }
    /// <summary>Compares two UUID values using RFC 4122 byte ordering.</summary>
    public int CompareTo(Uuid other) {
        if (_high <other._high)
        {
            return - 1;
        }
        if (_high >other._high)
        {
            return 1;
        }
        if (_low <other._low)
        {
            return - 1;
        }
        if (_low >other._low)
        {
            return 1;
        }
        return 0;
    }
    public Self Clone() => this;
    /// <summary>Equality operator.</summary>
    public static bool operator == (Uuid left, Uuid right) => left.Equals(right);
    /// <summary>Inequality operator.</summary>
    public static bool operator != (Uuid left, Uuid right) => ! left.Equals(right);
    private static Uuid FromRfcBytes(ReadOnlySpan <byte >bytes) {
        Pack(bytes, out var high, out var low);
        return new Uuid(high, low);
    }
    private static void Pack(ReadOnlySpan <byte >source, out ulong high, out ulong low) {
        high = ReadUInt64BigEndian(source, 0usize);
        low = ReadUInt64BigEndian(source, 8usize);
    }
    private void WriteRfcBytes(Span <byte >destination) {
        WriteUInt64BigEndian(destination, 0usize, _high);
        WriteUInt64BigEndian(destination, 8usize, _low);
    }
    private static ulong ReadUInt64BigEndian(ReadOnlySpan <byte >source, usize start) {
        return(NumericUnchecked.ToUInt64(source[start]) << 56) | (NumericUnchecked.ToUInt64(source[start + 1usize]) << 48) | (NumericUnchecked.ToUInt64(source[start + 2usize]) << 40) | (NumericUnchecked.ToUInt64(source[start + 3usize]) << 32) | (NumericUnchecked.ToUInt64(source[start + 4usize]) << 24) | (NumericUnchecked.ToUInt64(source[start + 5usize]) << 16) | (NumericUnchecked.ToUInt64(source[start + 6usize]) << 8) | NumericUnchecked.ToUInt64(source[start + 7usize]);
    }
    private static void WriteUInt64BigEndian(Span <byte >destination, usize start, ulong value) {
        destination[start] = NumericUnchecked.ToByte((long)(value >> 56));
        destination[start + 1usize] = NumericUnchecked.ToByte((long)(value >> 48));
        destination[start + 2usize] = NumericUnchecked.ToByte((long)(value >> 40));
        destination[start + 3usize] = NumericUnchecked.ToByte((long)(value >> 32));
        destination[start + 4usize] = NumericUnchecked.ToByte((long)(value >> 24));
        destination[start + 5usize] = NumericUnchecked.ToByte((long)(value >> 16));
        destination[start + 6usize] = NumericUnchecked.ToByte((long)(value >> 8));
        destination[start + 7usize] = NumericUnchecked.ToByte((long) value);
    }
    private static void WriteUInt32BigEndian(Span <byte >destination, usize start, uint value) {
        destination[start] = NumericUnchecked.ToByte((long)(value >> 24));
        destination[start + 1usize] = NumericUnchecked.ToByte((long)(value >> 16));
        destination[start + 2usize] = NumericUnchecked.ToByte((long)(value >> 8));
        destination[start + 3usize] = NumericUnchecked.ToByte((long) value);
    }
    private static void WriteUInt16BigEndian(Span <byte >destination, usize start, ushort value) {
        destination[start] = NumericUnchecked.ToByte((long)(value >> 8));
        destination[start + 1usize] = NumericUnchecked.ToByte((long) value);
    }
    private static UuidFormat ResolveFormat(string format, bool allowDefault) {
        let resolved = ResolveFormatOptional(format, allowDefault);
        if (resolved.IsSome (out var fmt)) {
            return fmt;
        }
        throw new Std.FormatException("Format specifier was not recognized for UUID.");
    }
    private static Std.Option <UuidFormat >ResolveFormatOptional(string format, bool allowDefault) {
        if (format == null)
        {
            if (allowDefault)
            {
                return Std.Option <UuidFormat >.Some(UuidFormat.D);
            }
            return Std.Option <UuidFormat >.None();
        }
        let span = Std.Span.ReadOnlySpan.FromStringChars(format);
        if (span.Length == 0usize)
        {
            if (allowDefault)
            {
                return Std.Option <UuidFormat >.Some(UuidFormat.D);
            }
            return Std.Option <UuidFormat >.None();
        }
        if (span.Length != 1usize)
        {
            return Std.Option <UuidFormat >.None();
        }
        let c = span[0usize];
        if (c == 'D' || c == 'd')
        {
            return Std.Option <UuidFormat >.Some(UuidFormat.D);
        }
        if (c == 'N' || c == 'n')
        {
            return Std.Option <UuidFormat >.Some(UuidFormat.N);
        }
        if (c == 'B' || c == 'b')
        {
            return Std.Option <UuidFormat >.Some(UuidFormat.B);
        }
        if (c == 'P' || c == 'p')
        {
            return Std.Option <UuidFormat >.Some(UuidFormat.P);
        }
        return Std.Option <UuidFormat >.None();
    }
    private static usize RequiredLength(UuidFormat format) {
        if (format == UuidFormat.N)
        {
            return 32usize;
        }
        if (format == UuidFormat.D)
        {
            return 36usize;
        }
        return 38usize;
    }
    private static bool TryParseCore(ReadOnlySpan <char >chars, Std.Option <UuidFormat >format, out Uuid value) {
        value = Empty;
        if (format.IsSome (out var forced)) {
            return TryParseWithFormat(chars, forced, out value);
        }
        if (chars.Length == 32usize)
        {
            return TryParseN(chars, out value);
        }
        if (chars.Length == 36usize)
        {
            return TryParseD(chars, out value);
        }
        if (chars.Length == 38usize)
        {
            if (chars[0usize] == '{' && chars[37usize] == '}')
            {
                return TryParseWrapped(chars, '{', '}', out value);
            }
            if (chars[0usize] == '(' && chars[37usize] == ')')
            {
                return TryParseWrapped(chars, '(', ')', out value);
            }
        }
        return false;
    }
    private static bool TryParseWithFormat(ReadOnlySpan <char >chars, UuidFormat format, out Uuid value) {
        if (format == UuidFormat.N)
        {
            return TryParseN(chars, out value);
        }
        if (format == UuidFormat.D)
        {
            return TryParseD(chars, out value);
        }
        if (format == UuidFormat.B)
        {
            return TryParseWrapped(chars, '{', '}', out value);
        }
        return TryParseWrapped(chars, '(', ')', out value);
    }
    private static bool TryParseN(ReadOnlySpan <char >chars, out Uuid value) {
        value = Empty;
        if (chars.Length != 32usize)
        {
            return false;
        }
        var bytes = Span <byte >.StackAlloc(ByteLength);
        if (! TryParseHexBlock (chars, 0usize, 32usize, bytes, 0usize))
        {
            return false;
        }
        value = FromRfcBytes(bytes.AsReadOnly());
        return true;
    }
    private static bool TryParseD(ReadOnlySpan <char >chars, out Uuid value) {
        value = Empty;
        if (chars.Length != 36usize)
        {
            return false;
        }
        if (chars[8usize] != '-' || chars[13usize] != '-' || chars[18usize] != '-' || chars[23usize] != '-')
        {
            return false;
        }
        var bytes = Span <byte >.StackAlloc(ByteLength);
        if (! TryParseHexBlock (chars, 0usize, 8usize, bytes, 0usize))
        {
            return false;
        }
        if (! TryParseHexBlock (chars, 9usize, 4usize, bytes, 4usize))
        {
            return false;
        }
        if (! TryParseHexBlock (chars, 14usize, 4usize, bytes, 6usize))
        {
            return false;
        }
        if (! TryParseHexBlock (chars, 19usize, 4usize, bytes, 8usize))
        {
            return false;
        }
        if (! TryParseHexBlock (chars, 24usize, 12usize, bytes, 10usize))
        {
            return false;
        }
        value = FromRfcBytes(bytes.AsReadOnly());
        return true;
    }
    private static bool TryParseWrapped(ReadOnlySpan <char >chars, char open, char close, out Uuid value) {
        value = Empty;
        if (chars.Length != 38usize)
        {
            return false;
        }
        if (chars[0usize] != open || chars[37usize] != close)
        {
            return false;
        }
        return TryParseD(chars.Slice(1usize, 36usize), out value);
    }
    private static bool TryParseHexBlock(ReadOnlySpan <char >chars, usize start, usize count, Span <byte >bytes, usize writeStart) {
        if ( (count & 1usize) != 0usize)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <count)
        {
            if (! TryGetHex (chars[start + idx], out var hi) || ! TryGetHex(chars[start + idx + 1usize], out var lo)) {
                return false;
            }
            let combined = NumericUnchecked.ToByte((long)((hi << 4) | lo));
            bytes[writeStart + (idx / 2usize)] = combined;
            idx += 2usize;
        }
        return true;
    }
    private static bool TryGetHex(char c, out byte value) {
        if (c >= '0' && c <= '9')
        {
            value = NumericUnchecked.ToByte(NumericUnchecked.ToInt64(c - '0'));
            return true;
        }
        if (c >= 'a' && c <= 'f')
        {
            value = NumericUnchecked.ToByte(NumericUnchecked.ToInt64(c - 'a' + 10));
            return true;
        }
        if (c >= 'A' && c <= 'F')
        {
            value = NumericUnchecked.ToByte(NumericUnchecked.ToInt64(c - 'A' + 10));
            return true;
        }
        value = 0u8;
        return false;
    }
    private void FormatAscii(Span <byte >destination, UuidFormat format) {
        var data = Span <byte >.StackAlloc(ByteLength);
        WriteRfcBytes(data);
        let hex = "0123456789abcdef".AsUtf8Span();
        var destIdx = 0usize;
        if (format == UuidFormat.B)
        {
            destination[destIdx] = NumericUnchecked.ToByte('{');
            destIdx += 1usize;
        }
        else if (format == UuidFormat.P)
        {
            destination[destIdx] = NumericUnchecked.ToByte('(');
            destIdx += 1usize;
        }
        WriteHexByte(data[0usize], destination, ref destIdx, hex);
        WriteHexByte(data[1usize], destination, ref destIdx, hex);
        WriteHexByte(data[2usize], destination, ref destIdx, hex);
        WriteHexByte(data[3usize], destination, ref destIdx, hex);
        if (format != UuidFormat.N)
        {
            destination[destIdx] = NumericUnchecked.ToByte('-');
            destIdx += 1usize;
        }
        WriteHexByte(data[4usize], destination, ref destIdx, hex);
        WriteHexByte(data[5usize], destination, ref destIdx, hex);
        if (format != UuidFormat.N)
        {
            destination[destIdx] = NumericUnchecked.ToByte('-');
            destIdx += 1usize;
        }
        WriteHexByte(data[6usize], destination, ref destIdx, hex);
        WriteHexByte(data[7usize], destination, ref destIdx, hex);
        if (format != UuidFormat.N)
        {
            destination[destIdx] = NumericUnchecked.ToByte('-');
            destIdx += 1usize;
        }
        WriteHexByte(data[8usize], destination, ref destIdx, hex);
        WriteHexByte(data[9usize], destination, ref destIdx, hex);
        if (format != UuidFormat.N)
        {
            destination[destIdx] = NumericUnchecked.ToByte('-');
            destIdx += 1usize;
        }
        WriteHexByte(data[10usize], destination, ref destIdx, hex);
        WriteHexByte(data[11usize], destination, ref destIdx, hex);
        WriteHexByte(data[12usize], destination, ref destIdx, hex);
        WriteHexByte(data[13usize], destination, ref destIdx, hex);
        WriteHexByte(data[14usize], destination, ref destIdx, hex);
        WriteHexByte(data[15usize], destination, ref destIdx, hex);
        if (format == UuidFormat.B)
        {
            destination[destIdx] = NumericUnchecked.ToByte('}');
        }
        else if (format == UuidFormat.P)
        {
            destination[destIdx] = NumericUnchecked.ToByte(')');
        }
    }
    private static void WriteHexByte(byte value, Span <byte >destination, ref usize idx, ReadOnlySpan <byte >hex) {
        destination[idx] = hex[(value >> 4) & 0x0Fu8];
        destination[idx + 1usize] = hex[value & 0x0Fu8];
        idx += 2usize;
    }
    private static void FillRandom(Span <byte >bytes) {
        if (bytes.Length == 0usize)
        {
            return;
        }
        let raw = bytes.Raw;
        let ok = UuidRuntime.chic_rt_random_fill(raw.Data.Pointer, bytes.Length);
        if (! ok)
        {
            throw new Std.InvalidOperationException("cryptographic RNG unavailable");
        }
    }
}
internal static class UuidRuntime
{
    @extern("C") public static extern bool chic_rt_random_fill(* mut @expose_address byte buffer, usize length);
}
