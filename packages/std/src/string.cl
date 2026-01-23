namespace Std;
import Std.Runtime.InteropServices;
import Std.Globalization;
import Std.Numeric;
// includes IEquatable, numeric helpers
import Std.Datetime;
import Std.Strings;
import Std.Span;
@Intrinsic @StructLayout(LayoutKind.Sequential) @primitive(primitive = "string", kind = "string", aliases = ["string", "String",
"Std.String", "System.String"], c_type = "struct chic_string") public readonly struct String : Clone, Copy, IConvertible, IEquatable <string >
{
    /// <summary>Gets the number of UTF-8 code units in the string.</summary>
    public int Length {
        get {
            let span = this.AsUtf8Span();
            return NumericUnchecked.ToInt32(span.Length);
        }
    }
    /// <summary>Gets the character at the specified index.</summary>
    public char this[int index] {
        get {
            let span = this.AsSpan();
            if (index <0 || NumericUnchecked.ToUSize (index) >= span.Length)
            {
                throw new Std.ArgumentOutOfRangeException("index");
            }
            return span[NumericUnchecked.ToUSize(index)];
        }
    }
    public override string ToString() => this;
    public static bool IsNullOrEmpty(string value) => value == null || value.Length == 0;
    public int IndexOf(char value) => IndexOf(value, 0);
    public int IndexOf(char value, int startIndex) {
        let chars = this.AsSpan();
        if (startIndex <0 || NumericUnchecked.ToUSize (startIndex) >chars.Length)
        {
            throw new Std.ArgumentOutOfRangeException("startIndex");
        }
        var idx = NumericUnchecked.ToUSize(startIndex);
        while (idx <chars.Length)
        {
            if (NumericUnchecked.ToUInt32 (chars[idx]) == NumericUnchecked.ToUInt32 (value))
            {
                return NumericUnchecked.ToInt32(idx);
            }
            idx += 1usize;
        }
        return - 1;
    }
    public int IndexOf(string value) => IndexOf(value, 0);
    public int IndexOf(string value, int startIndex) {
        if (value == null)
        {
            throw new Std.ArgumentNullException("value");
        }
        let haystack = this.AsUtf8Span();
        let needle = value.AsUtf8Span();
        if (startIndex <0 || NumericUnchecked.ToUSize (startIndex) >haystack.Length)
        {
            throw new Std.ArgumentOutOfRangeException("startIndex");
        }
        if (needle.Length == 0usize)
        {
            return startIndex;
        }
        var idx = NumericUnchecked.ToUSize(startIndex);
        while (idx + needle.Length <= haystack.Length)
        {
            if (Utf8Equals (haystack.Slice (idx, needle.Length), needle))
            {
                return NumericUnchecked.ToInt32(idx);
            }
            idx += 1usize;
        }
        return - 1;
    }
    public bool StartsWith(string value) {
        if (value == null)
        {
            throw new Std.ArgumentNullException("value");
        }
        let span = this.AsUtf8Span();
        let prefix = value.AsUtf8Span();
        if (prefix.Length >span.Length)
        {
            return false;
        }
        return Utf8Equals(span.Slice(0usize, prefix.Length), prefix);
    }
    public string Substring(int startIndex) {
        if (startIndex <0)
        {
            throw new Std.ArgumentOutOfRangeException("startIndex");
        }
        let utf8 = this.AsUtf8Span();
        let start = NumericUnchecked.ToUSize(startIndex);
        if (start >utf8.Length)
        {
            throw new Std.ArgumentOutOfRangeException("startIndex");
        }
        return Utf8String.FromSpan(utf8.Slice(start, utf8.Length - start));
    }
    public string Substring(int startIndex, int length) {
        if (startIndex <0 || length <0)
        {
            throw new Std.ArgumentOutOfRangeException("startIndex/length");
        }
        let utf8 = this.AsUtf8Span();
        let start = NumericUnchecked.ToUSize(startIndex);
        let len = NumericUnchecked.ToUSize(length);
        if (start + len >utf8.Length)
        {
            throw new Std.ArgumentOutOfRangeException("startIndex/length");
        }
        return Utf8String.FromSpan(utf8.Slice(start, len));
    }
    public bool Equals(string other) {
        if (other == null)
        {
            return false;
        }
        return Utf8Equals(this.AsUtf8Span(), other.AsUtf8Span());
    }
    public override bool Equals(Object other) {
        return false;
    }
    public override int GetHashCode() {
        let span = this.AsUtf8Span();
        var hash = 17;
        var idx = 0usize;
        while (idx <span.Length)
        {
            hash = (hash * 31) + NumericUnchecked.ToInt32(span[idx]);
            idx += 1usize;
        }
        return hash;
    }
    public static bool operator == (string left, string right) {
        if (left is null) {
            return right is null;
        }
        return left.Equals(right);
    }
    public static bool operator != (string left, string right) => !(left == right);
    private static bool Utf8Equals(ReadOnlySpan <byte >left, ReadOnlySpan <byte >right) {
        if (left.Length != right.Length)
        {
            return false;
        }
        var idx = 0usize;
        while (idx <left.Length)
        {
            if (left[idx] != right[idx])
            {
                return false;
            }
            idx += 1usize;
        }
        return true;
    }
    public Self Clone() => this;
    public string ToString(IFormatProvider provider) => this;
    public bool ToBoolean(IFormatProvider provider) {
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (this, "true"))
        {
            return true;
        }
        if (NumericCultureInfo.EqualsIgnoreAsciiCase (this, "false"))
        {
            return false;
        }
        throw new Std.FormatException("String was not recognized as a Boolean");
    }
    public char ToChar(IFormatProvider provider) {
        var text = this;
        let span = text.AsSpan();
        if (span.Length != 1)
        {
            throw new Std.FormatException("String must be exactly one character to convert to Char");
        }
        return span[0];
    }
    public sbyte ToSByte(IFormatProvider provider) {
        if (NumericParse.TryParseSByte (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as an SByte");
    }
    public byte ToByte(IFormatProvider provider) {
        if (NumericParse.TryParseByte (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as a Byte");
    }
    public short ToInt16(IFormatProvider provider) {
        if (NumericParse.TryParseInt16 (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as an Int16");
    }
    public ushort ToUInt16(IFormatProvider provider) {
        if (NumericParse.TryParseUInt16 (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as a UInt16");
    }
    public int ToInt32(IFormatProvider provider) {
        if (NumericParse.TryParseInt32 (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as an Int32");
    }
    public uint ToUInt32(IFormatProvider provider) {
        if (NumericParse.TryParseUInt32 (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as a UInt32");
    }
    public long ToInt64(IFormatProvider provider) {
        if (NumericParse.TryParseInt64 (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as an Int64");
    }
    public ulong ToUInt64(IFormatProvider provider) {
        if (NumericParse.TryParseUInt64 (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as a UInt64");
    }
    public nint ToNInt(IFormatProvider provider) => NumericUnchecked.ToNintFromInt64(ToInt64(provider));
    public nuint ToNUInt(IFormatProvider provider) => NumericUnchecked.ToNuintWiden(ToUInt64(provider));
    public isize ToISize(IFormatProvider provider) => (isize) ToNInt(provider);
    public usize ToUSize(IFormatProvider provider) => (usize) ToNUInt(provider);
    public Int128 ToInt128(IFormatProvider provider) {
        var status = ParseStatus.Invalid;
        if (NumericParse.TryParseInt128 (this, out var parsed, out status)) {
            return parsed;
        }
        NumericParse.ThrowParseException(status, "Int128");
        return new Int128(0);
    }
    public UInt128 ToUInt128(IFormatProvider provider) {
        var status = ParseStatus.Invalid;
        if (NumericParse.TryParseUInt128 (this, out var parsed, out status)) {
            return parsed;
        }
        NumericParse.ThrowParseException(status, "UInt128");
        return new UInt128(0u128);
    }
    public float ToSingle(IFormatProvider provider) {
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float32");
        return 0.0f;
    }
    public double ToDouble(IFormatProvider provider) {
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Float64");
        return 0.0d;
    }
    public Float128 ToFloat128(IFormatProvider provider) => new Float128(ToDouble(provider));
    public Decimal ToDecimal(IFormatProvider provider) {
        let culture = ConvertibleHelpers.ResolveCulture(provider);
        if (NumericParse.TryParseDecimal (this, culture, out var parsed)) {
            return parsed;
        }
        NumericParse.ThrowParseException(ParseStatus.Invalid, "Decimal");
        return 0m;
    }
    public Std.Datetime.DateTime ToDateTime(IFormatProvider provider) {
        if (DateTimeParsing.TryParseIso (this, out var parsed)) {
            return parsed;
        }
        throw new Std.FormatException("String was not recognized as a DateTime");
    }
    public Object ToType(Type targetType, IFormatProvider provider) => ConvertibleHelpers.ToTypeNotSupported("String");
}
