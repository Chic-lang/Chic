import Std.Core.Testing;
import Std.Core;
import Std.Numeric;
import Std.Runtime.Collections;
import Std.Span;
namespace Std.Strings
{
    internal static class DebugIntrinsics
    {
        @extern("C") public static extern void chic_rt_debug_mark(u64 code, u64 a, u64 b, u64 c);
        @extern("C") public unsafe static extern StrPtr chic_rt_string_as_slice(* const @readonly @expose_address string value);
        @extern("C") public static extern string chic_rt_string_from_slice(StrPtr slice);
        @extern("C") public unsafe static extern * mut @expose_address byte chic_rt_string_get_ptr(* const @readonly @expose_address string value);
        @extern("C") public unsafe static extern * mut @expose_address byte chic_rt_string_inline_ptr(* mut @expose_address string value);
    }
    public extension string
    {
        public ReadOnlySpan <char >AsSpan(this Self value) {
            return ReadOnlySpan.FromStringChars(value);
        }
        public ReadOnlySpan <byte >AsUtf8Span(this Self value) {
            return ReadOnlySpan.FromString(value);
        }
        public bool TryCopyUtf8(this Self value, Span <byte >destination, out usize written) {
            let utf8View = ReadOnlySpan.FromString(value);
            if (destination.Length <utf8View.Length)
            {
                written = 0;
                return false;
            }
            destination.CopyFrom(utf8View);
            written = utf8View.Length;
            return true;
        }
    }
    public static class Utf8StringExtensions
    {
        public static bool StartsWith(this string value, string prefix) {
            if (prefix == null)
            {
                throw new Std.ArgumentNullException("prefix");
            }
            let haystack = Std.Span.ReadOnlySpan.FromString(value);
            let needle = Std.Span.ReadOnlySpan.FromString(prefix);
            if (needle.Length >haystack.Length)
            {
                return false;
            }
            var idx = 0usize;
            while (idx <needle.Length)
            {
                if (haystack[idx] != needle[idx])
                {
                    return false;
                }
                idx += 1usize;
            }
            return true;
        }
        public static int IndexOf(this string value, char needle) => IndexOf(value, needle, 0);
        public static int IndexOf(this string value, char needle, int startIndex) {
            let haystack = Std.Span.ReadOnlySpan.FromString(value);
            if (startIndex <0 || (usize) startIndex >haystack.Length)
            {
                throw new Std.ArgumentOutOfRangeException("startIndex");
            }
            let needleValue = (uint) needle;
            if (needleValue >0xFFu)
            {
                return - 1;
            }
            let needleByte = (byte) needleValue;
            var idx = (usize) startIndex;
            while (idx <haystack.Length)
            {
                if (haystack[idx] == needleByte)
                {
                    return(int) idx;
                }
                idx += 1usize;
            }
            return - 1;
        }
        public static int IndexOf(this string value, string needle) => IndexOf(value, needle, 0);
        public static int IndexOf(this string value, string needle, int startIndex) {
            if (needle == null)
            {
                throw new Std.ArgumentNullException("needle");
            }
            let haystack = Std.Span.ReadOnlySpan.FromString(value);
            let needleBytes = Std.Span.ReadOnlySpan.FromString(needle);
            if (startIndex <0 || (usize) startIndex >haystack.Length)
            {
                throw new Std.ArgumentOutOfRangeException("startIndex");
            }
            if (needleBytes.Length == 0usize)
            {
                return startIndex;
            }
            var idx = (usize) startIndex;
            while (idx + needleBytes.Length <= haystack.Length)
            {
                var matched = true;
                var needleIdx = 0usize;
                while (needleIdx <needleBytes.Length)
                {
                    if (haystack[idx + needleIdx] != needleBytes[needleIdx])
                    {
                        matched = false;
                        break;
                    }
                    needleIdx += 1usize;
                }
                if (matched)
                {
                    return(int) idx;
                }
                idx += 1usize;
            }
            return - 1;
        }
        public static string Substring(this string value, int startIndex) {
            if (startIndex <0)
            {
                throw new Std.ArgumentOutOfRangeException("startIndex");
            }
            let utf8 = Std.Span.ReadOnlySpan.FromString(value);
            let start = (usize) startIndex;
            if (start >utf8.Length)
            {
                throw new Std.ArgumentOutOfRangeException("startIndex");
            }
            return Utf8String.FromSpan(utf8.Slice(start, utf8.Length - start));
        }
        public static string Substring(this string value, int startIndex, int length) {
            if (startIndex <0 || length <0)
            {
                throw new Std.ArgumentOutOfRangeException("startIndex/length");
            }
            let utf8 = Std.Span.ReadOnlySpan.FromString(value);
            let start = (usize) startIndex;
            let len = (usize) length;
            if (start + len >utf8.Length)
            {
                throw new Std.ArgumentOutOfRangeException("startIndex/length");
            }
            return Utf8String.FromSpan(utf8.Slice(start, len));
        }
    }
    public extension str
    {
        public ReadOnlySpan <char >AsSpan(this Self value) {
            var slice = value;
            return ReadOnlySpan.FromStr(slice);
        }
    }
    public static class Utf8String
    {
        public static string FromSpan(ReadOnlySpan <byte >span) {
            var handle = span.Raw;
            if (handle.ElementSize != 1)
            {
                throw new Std.InvalidOperationException("Utf8 spans must have a byte-sized element handle");
            }
            if (handle.ElementAlignment != 1)
            {
                throw new Std.InvalidOperationException("Utf8 spans must report byte alignment");
            }
            var slice = CoreIntrinsics.DefaultValue <StrPtr >();
            slice.Pointer = handle.Data.Pointer;
            slice.Length = span.Length;
            return SpanIntrinsics.chic_rt_string_from_slice(slice);
        }
    }
    testcase Given_utf8_string_try_copy_utf8_destination_too_small_When_executed_Then_returns_false()
    {
        let text = Utf8String.FromSpan(ReadOnlySpan.FromString("hello"));
        var buffer = Span <byte >.StackAlloc(1);
        let ok = text.TryCopyUtf8(buffer, out var written);
        Assert.That(ok).IsFalse();
        Assert.That(written == 0usize).IsTrue();
    }
    testcase Given_utf8_string_try_copy_utf8_destination_fits_When_executed_Then_copies_bytes()
    {
        let slice = StrPtr.FromStr("hello");
        let owned = DebugIntrinsics.chic_rt_string_from_slice(slice);
        unsafe {
            var * const @readonly @expose_address string ownedPtr = & owned;
            let rawSlice = DebugIntrinsics.chic_rt_string_as_slice(ownedPtr);
            let first = rawSlice.Pointer == null ?0u8 : * rawSlice.Pointer;
            let dataPtr = DebugIntrinsics.chic_rt_string_get_ptr(ownedPtr);
            let dataFirst = dataPtr == null ?0u8 : * dataPtr;
            var * mut @expose_address string ownedMutPtr = & owned;
            let inlinePtr = DebugIntrinsics.chic_rt_string_inline_ptr(ownedMutPtr);
            let inlineFirst = inlinePtr == null ?0u8 : * inlinePtr;
            let baseAddr = Std.Numeric.Pointer.HandleFrom(ownedMutPtr);
            let capAddr = baseAddr + (nuint) 16;
            var * const @readonly @expose_address u64 capPtr = (* const @readonly @expose_address u64) capAddr;
            let capValue = capPtr == null ?0u64 : * capPtr;
            DebugIntrinsics.chic_rt_debug_mark(
                198u64,
                (u64) Std.Numeric.Pointer.HandleFromConst(rawSlice.Pointer),
                (u64) rawSlice.Length,
                (u64) first
            );
            DebugIntrinsics.chic_rt_debug_mark(
                197u64,
                (u64) Std.Numeric.Pointer.HandleFrom(dataPtr),
                (u64) rawSlice.Length,
                (u64) dataFirst
            );
            DebugIntrinsics.chic_rt_debug_mark(
                196u64,
                (u64) Std.Numeric.Pointer.HandleFrom(inlinePtr),
                (u64) rawSlice.Length,
                (u64) inlineFirst
            );
            DebugIntrinsics.chic_rt_debug_mark(
                195u64,
                capValue,
                0u64,
                0u64
            );
        }
        let source = ReadOnlySpan.FromString(owned);
        unsafe {
            DebugIntrinsics.chic_rt_debug_mark(
                200u64,
                (u64) Std.Numeric.Pointer.HandleFromConst(source.Raw.Data.Pointer),
                (u64) source.Length,
                (u64) source.Raw.ElementSize
            );
            DebugIntrinsics.chic_rt_debug_mark(203u64, (u64) source[0usize], (u64) source[4usize], 0u64);
        }
        let text = Utf8String.FromSpan(source);
        var buffer = Span <byte >.StackAlloc(8);
        let ok = text.TryCopyUtf8(buffer, out var written);
        unsafe {
            DebugIntrinsics.chic_rt_debug_mark(201u64, ok ?1u64 : 0u64, (u64) written, 0u64);
            DebugIntrinsics.chic_rt_debug_mark(202u64, (u64) buffer[0usize], (u64) buffer[4usize], 0u64);
        }
        Assert.That(ok).IsTrue();
        Assert.That(written == 5usize).IsTrue();
        Assert.That(buffer[0usize] == (byte) 'h').IsTrue();
        Assert.That(buffer[4usize] == (byte) 'o').IsTrue();
    }
    testcase Given_utf8_string_starts_with_When_executed_Then_matches_prefix()
    {
        Assert.That("hello".StartsWith("he")).IsTrue();
        Assert.That("hello".StartsWith("lo")).IsFalse();
    }
    testcase Given_utf8_string_starts_with_null_prefix_When_executed_Then_throws_argument_null_exception()
    {
        var threw = false;
        try {
            let prefix = CoreIntrinsics.DefaultValue <string >();
            let _ = "hello".StartsWith(prefix);
        }
        catch(Std.ArgumentNullException) {
            threw = true;
        }
        Assert.That(threw).IsTrue();
    }
    testcase Given_utf8_string_index_of_char_When_executed_Then_returns_expected_index()
    {
        Assert.That("abc".IndexOf('b') == 1).IsTrue();
        Assert.That("abc".IndexOf('d') == - 1).IsTrue();
    }
    testcase Given_utf8_string_index_of_char_out_of_range_char_When_executed_Then_returns_minus_one()
    {
        Assert.That("abc".IndexOf('☃') == - 1).IsTrue();
    }
    testcase Given_utf8_string_index_of_char_start_index_out_of_range_When_executed_Then_throws()
    {
        var threw = false;
        try {
            let _ = "abc".IndexOf('a', 4);
        }
        catch(Std.ArgumentOutOfRangeException) {
            threw = true;
        }
        Assert.That(threw).IsTrue();
    }
    testcase Given_utf8_string_index_of_string_When_executed_Then_returns_expected_index()
    {
        Assert.That("hello".IndexOf("ell") == 1).IsTrue();
        Assert.That("hello".IndexOf("world") == - 1).IsTrue();
    }
    testcase Given_utf8_string_index_of_string_empty_needle_When_executed_Then_returns_start_index()
    {
        Assert.That("hello".IndexOf("") == 0).IsTrue();
        Assert.That("hello".IndexOf("", 3) == 3).IsTrue();
    }
    testcase Given_utf8_string_index_of_string_null_needle_When_executed_Then_throws()
    {
        var threw = false;
        try {
            let needle = CoreIntrinsics.DefaultValue <string >();
            let _ = "hello".IndexOf(needle);
        }
        catch(Std.ArgumentNullException) {
            threw = true;
        }
        Assert.That(threw).IsTrue();
    }
    testcase Given_utf8_string_substring_When_executed_Then_returns_expected_text()
    {
        Assert.That("hello".Substring(2) == "llo").IsTrue();
        Assert.That("hello".Substring(1, 3) == "ell").IsTrue();
    }
    testcase Given_utf8_string_substring_out_of_range_When_executed_Then_throws()
    {
        var threw = false;
        try {
            let _ = "hello".Substring(6);
        }
        catch(Std.ArgumentOutOfRangeException) {
            threw = true;
        }
        Assert.That(threw).IsTrue();
    }
    testcase Given_str_as_span_When_executed_Then_span_matches()
    {
        let slice = "hi";
        let span = slice.AsSpan();
        Assert.That(span.Length == 2usize).IsTrue();
        Assert.That(span[0usize] == 'h').IsTrue();
        Assert.That(span[1usize] == 'i').IsTrue();
    }
    testcase Given_utf8_string_from_span_roundtrip_When_executed_Then_roundtrips()
    {
        let bytes = ReadOnlySpan.FromString("abc");
        let text = Utf8String.FromSpan(bytes);
        Assert.That(text == "abc").IsTrue();
    }
    testcase Given_utf8_string_from_span_wrong_element_size_When_executed_Then_throws()
    {
        var threw = false;
        unsafe {
            var value = 0u8;
            var * mut @expose_address byte ptr = & value;
            let bytes = PointerIntrinsics.AsByteConstFromMut(ptr);
            let bad = ValuePointer.CreateConst(bytes, 2usize, 1usize);
            let span = ReadOnlySpan <byte >.FromValuePointer(bad, 1usize);
            try {
                let _ = Utf8String.FromSpan(span);
            }
            catch(Std.InvalidOperationException) {
                threw = true;
            }
        }
        Assert.That(threw).IsTrue();
    }
    testcase Given_utf8_string_from_span_wrong_alignment_When_executed_Then_throws()
    {
        var threw = false;
        unsafe {
            var value = 0u8;
            var * mut @expose_address byte ptr = & value;
            let bytes = PointerIntrinsics.AsByteConstFromMut(ptr);
            let bad = ValuePointer.CreateConst(bytes, 1usize, 2usize);
            let span = ReadOnlySpan <byte >.FromValuePointer(bad, 1usize);
            try {
                let _ = Utf8String.FromSpan(span);
            }
            catch(Std.InvalidOperationException) {
                threw = true;
            }
        }
        Assert.That(threw).IsTrue();
    }
}
