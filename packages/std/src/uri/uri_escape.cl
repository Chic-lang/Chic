namespace Std;
import Std.Numeric;
import Std.Runtime;
import Std.Strings;
import Std.Span;
import Std.Runtime.Collections;
import Foundation.Collections;
import FVec = Foundation.Collections.Vec;
import FVecIntrinsics = Foundation.Collections.VecIntrinsics;
internal enum UriEscapeComponent
{
    Path = 0, Query = 1, Fragment = 2, UserInfo = 3,
}
internal static class UriEscape
{
    internal static bool IsHexDigit(byte value) {
        return(value >= NumericUnchecked.ToByte('0') && value <= NumericUnchecked.ToByte('9')) || (value >= NumericUnchecked.ToByte('a') && value <= NumericUnchecked.ToByte('f')) || (value >= NumericUnchecked.ToByte('A') && value <= NumericUnchecked.ToByte('F'));
    }
    internal static int FromHex(byte value) {
        if (value >= NumericUnchecked.ToByte ('0') && value <= NumericUnchecked.ToByte ('9'))
        {
            return NumericUnchecked.ToInt32(value - NumericUnchecked.ToByte('0'));
        }
        if (value >= NumericUnchecked.ToByte ('a') && value <= NumericUnchecked.ToByte ('f'))
        {
            return 10 + NumericUnchecked.ToInt32(value - NumericUnchecked.ToByte('a'));
        }
        if (value >= NumericUnchecked.ToByte ('A') && value <= NumericUnchecked.ToByte ('F'))
        {
            return 10 + NumericUnchecked.ToInt32(value - NumericUnchecked.ToByte('A'));
        }
        return - 1;
    }
    internal static bool IsHexDigit(char value) {
        let b = NumericUnchecked.ToByte(value);
        return IsHexDigit(b);
    }
    internal static int FromHex(char value) {
        let b = NumericUnchecked.ToByte(value);
        return FromHex(b);
    }
    internal static bool IsUnreserved(byte value) {
        return(value >= NumericUnchecked.ToByte('A') && value <= NumericUnchecked.ToByte('Z')) || (value >= NumericUnchecked.ToByte('a') && value <= NumericUnchecked.ToByte('z')) || (value >= NumericUnchecked.ToByte('0') && value <= NumericUnchecked.ToByte('9')) || value == NumericUnchecked.ToByte('-') || value == NumericUnchecked.ToByte('.') || value == NumericUnchecked.ToByte('_') || value == NumericUnchecked.ToByte('~');
    }
    internal static bool IsSubDelim(byte value) {
        return value == NumericUnchecked.ToByte('!') || value == NumericUnchecked.ToByte('$') || value == NumericUnchecked.ToByte('&') || value == NumericUnchecked.ToByte('(') || value == NumericUnchecked.ToByte(')') || value == NumericUnchecked.ToByte('*') || value == NumericUnchecked.ToByte('+') || value == NumericUnchecked.ToByte(',') || value == NumericUnchecked.ToByte(';') || value == NumericUnchecked.ToByte('=') || value == NumericUnchecked.ToByte('\'');
    }
    internal static bool IsAllowedInPath(byte value) {
        return IsUnreserved(value) || IsSubDelim(value) || value == NumericUnchecked.ToByte(':') || value == NumericUnchecked.ToByte('@') || value == NumericUnchecked.ToByte('/');
    }
    internal static bool IsAllowedInQueryOrFragment(byte value) {
        return IsUnreserved(value) || IsSubDelim(value) || value == NumericUnchecked.ToByte(':') || value == NumericUnchecked.ToByte('@') || value == NumericUnchecked.ToByte('/') || value == NumericUnchecked.ToByte('?');
    }
    internal static bool IsAllowedInUserInfo(byte value) {
        return IsUnreserved(value) || IsSubDelim(value) || value == NumericUnchecked.ToByte(':');
    }
    internal static bool IsAllowedComponent(byte value, UriEscapeComponent component) {
        switch (component)
        {
            case UriEscapeComponent.Path:
                return IsAllowedInPath(value);
            case UriEscapeComponent.Query:
                return IsAllowedInQueryOrFragment(value);
            case UriEscapeComponent.Fragment:
                return IsAllowedInQueryOrFragment(value);
            case UriEscapeComponent.UserInfo:
                return IsAllowedInUserInfo(value);
            default :
                return false;
            }
        }
        internal static string EscapeComponent(string value, UriEscapeComponent component, bool preserveEscapes) {
            if (value == null)
            {
                return StringRuntime.Create();
            }
            let span = value.AsUtf8Span();
            if (span.Length == 0)
            {
                return value;
            }
            var buffer = FVec.WithCapacity <byte >(span.Length);
            var index = 0usize;
            while (index <span.Length)
            {
                let b = span[index];
                if (IsAllowedComponent (b, component))
                {
                    FVec.Push <byte >(ref buffer, b);
                    index += 1usize;
                    continue;
                }
                if (preserveEscapes && b == NumericUnchecked.ToByte ('%') && index + 2usize <span.Length && IsHexDigit (span[index + 1usize]) && IsHexDigit (span[index + 2usize]))
                {
                    AppendEscapeLiteral(ref buffer, span[index + 1usize], span[index + 2usize]);
                    index += 3usize;
                    continue;
                }
                AppendHexEscape(ref buffer, b);
                index += 1usize;
            }
            let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
            return result;
        }
        internal static string EscapeDataString(string value) {
            if (value == null)
            {
                return StringRuntime.Create();
            }
            let span = value.AsUtf8Span();
            var buffer = FVec.WithCapacity <byte >(span.Length);
            var index = 0usize;
            while (index <span.Length)
            {
                let b = span[index];
                if (IsUnreserved (b))
                {
                    FVec.Push <byte >(ref buffer, b);
                }
                else
                {
                    AppendHexEscape(ref buffer, b);
                }
                index += 1usize;
            }
            let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
            return result;
        }
        internal static string EscapeUriString(string value) {
            if (value == null)
            {
                return StringRuntime.Create();
            }
            return EscapeComponent(value, UriEscapeComponent.Path, true);
        }
        internal static string UnescapeString(string value, bool safe) {
            if (value == null)
            {
                return StringRuntime.Create();
            }
            let span = value.AsUtf8Span();
            if (span.Length == 0)
            {
                return value;
            }
            var buffer = FVec.WithCapacity <byte >(span.Length);
            var index = 0usize;
            while (index <span.Length)
            {
                let b = span[index];
                if (b == NumericUnchecked.ToByte ('%') && index + 2usize <span.Length && IsHexDigit (span[index + 1usize]) && IsHexDigit (span[index + 2usize]))
                {
                    let decoded = (FromHex(span[index + 1usize]) << 4) | FromHex(span[index + 2usize]);
                    if (safe && !IsUnreserved (NumericUnchecked.ToByte (decoded)))
                    {
                        AppendEscapeLiteral(ref buffer, span[index + 1usize], span[index + 2usize]);
                    }
                    else
                    {
                        FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte(decoded));
                    }
                    index += 3usize;
                    continue;
                }
                FVec.Push <byte >(ref buffer, b);
                index += 1usize;
            }
            let result = Utf8String.FromSpan(FVec.AsReadOnlySpan <byte >(in buffer));
            FVecIntrinsics.chic_rt_vec_drop(ref buffer);
            return result;
        }
        internal static string HexEscape(char character) {
            let value = NumericUnchecked.ToInt32(character);
            if (value <0 || value >255)
            {
                throw new UriFormatException("HexEscape expects a character in the 0-255 range");
            }
            var buffer = Span <byte >.StackAlloc(3);
            buffer[0] = NumericUnchecked.ToByte('%');
            WriteHexByte((byte) value, buffer.Slice(1, 2));
            return Utf8String.FromSpan(buffer.AsReadOnly());
        }
        internal static char HexUnescape(string pattern, ref int index) {
            if (pattern == null)
            {
                throw new ArgumentNullException("pattern");
            }
            if (index <0 || index >= pattern.Length)
            {
                throw new ArgumentOutOfRangeException("index");
            }
            let current = pattern[index];
            if (current == '%' && index + 2 <pattern.Length)
            {
                let hi = pattern[index + 1];
                let lo = pattern[index + 2];
                if (IsHexDigit (hi) && IsHexDigit (lo))
                {
                    let value = (FromHex(hi) << 4) | FromHex(lo);
                    index += 3;
                    return NumericUnchecked.ToChar(value);
                }
            }
            index += 1;
            return current;
        }
        internal static bool IsHexEncoding(string pattern, int index) {
            if (pattern == null)
            {
                return false;
            }
            if (index <0 || index + 2 >= pattern.Length)
            {
                return false;
            }
            return pattern[index] == '%' && IsHexDigit(pattern[index + 1]) && IsHexDigit(pattern[index + 2]);
        }
        internal static void AppendHexEscape(ref VecPtr buffer, byte value) {
            FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte('%'));
            var scratch = Span <byte >.StackAlloc(2);
            WriteHexByte(value, scratch);
            FVec.Push <byte >(ref buffer, scratch[0]);
            FVec.Push <byte >(ref buffer, scratch[1]);
        }
        internal static void AppendEscapeLiteral(ref VecPtr buffer, byte hi, byte lo) {
            FVec.Push <byte >(ref buffer, NumericUnchecked.ToByte('%'));
            FVec.Push <byte >(ref buffer, ToUpperHex(hi));
            FVec.Push <byte >(ref buffer, ToUpperHex(lo));
        }
        internal static byte ToUpperHex(byte value) {
            if (value >= NumericUnchecked.ToByte ('a') && value <= NumericUnchecked.ToByte ('f'))
            {
                return NumericUnchecked.ToByte(NumericUnchecked.ToInt32(value) - 32);
            }
            return value;
        }
        internal static void WriteHexByte(byte value, Span <byte >destination) {
            let hi = (value >> 4) & 0x0F;
            let lo = value & 0x0F;
            destination[0] = HexDigit(hi);
            destination[1] = HexDigit(lo);
        }
        internal static byte HexDigit(int value) {
            if (value <10)
            {
                return NumericUnchecked.ToByte(48 + value);
            }
            return NumericUnchecked.ToByte(55 + value);
        }
        }
