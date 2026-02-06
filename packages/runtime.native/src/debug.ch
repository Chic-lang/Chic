namespace Std.Runtime.Native;
// Debug mark export implemented directly in Chic. Uses libc `write` to emit a
// single formatted line to stderr so tooling can scrape runtime breadcrumbs.
@repr(c) internal struct InlineBytes128
{
    public InlineBytes64 lo;
    public InlineBytes64 hi;
}
public static class DebugMark
{
    @extern("C") private unsafe static extern isize write(int fd, * const @readonly @expose_address byte buf, usize len);
    private unsafe static InlineBytes64 ZeroInline64() {
        return new InlineBytes64 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0, b32 = 0, b33 = 0, b34 = 0, b35 = 0, b36 = 0, b37 = 0, b38 = 0, b39 = 0, b40 = 0, b41 = 0, b42 = 0, b43 = 0, b44 = 0, b45 = 0, b46 = 0, b47 = 0, b48 = 0, b49 = 0, b50 = 0, b51 = 0, b52 = 0, b53 = 0, b54 = 0, b55 = 0, b56 = 0, b57 = 0, b58 = 0, b59 = 0, b60 = 0, b61 = 0, b62 = 0, b63 = 0,
        }
        ;
    }
    private unsafe static InlineBytes128 ZeroInline128() {
        return new InlineBytes128 {
            lo = ZeroInline64(), hi = ZeroInline64()
        }
        ;
    }
    private unsafe static * mut @expose_address byte WriteChar(* mut @expose_address byte dest, char value) {
        * dest = (byte) value;
        return NativePtr.OffsetMut(dest, 1isize);
    }
    private unsafe static * mut @expose_address byte WriteU64(* mut @expose_address byte dest, u64 value) {
        // Render into a scratch buffer from the end and then copy forward.
        var scratch = ZeroInline64();
        let basePtr = (* mut @expose_address byte) & scratch;
        let endPtr = NativePtr.OffsetMut(basePtr, 64isize);
        var cursor = endPtr;
        var outPtr = dest;
        if (value == 0)
        {
            * outPtr = (byte) '0';
            return NativePtr.OffsetMut(outPtr, 1isize);
        }
        var remaining = value;
        while (remaining >0)
        {
            cursor = NativePtr.OffsetMut(cursor, - 1isize);
            let digit = (byte)(remaining % 10u64);
            * cursor = (byte)(digit + (byte) '0');
            remaining = remaining / 10u64;
        }
        while (NativePtr.ToIsize (cursor) <NativePtr.ToIsize (endPtr))
        {
            * outPtr = * cursor;
            outPtr = NativePtr.OffsetMut(outPtr, 1isize);
            cursor = NativePtr.OffsetMut(cursor, 1isize);
        }
        return outPtr;
    }
    @extern("C") @export("chic_rt_debug_mark") public unsafe static void chic_rt_debug_mark(u64 code, u64 a, u64 b, u64 c) {
        var buffer = ZeroInline128();
        let start = (* mut @expose_address byte) & buffer;
        var cursor = start;
        // prefix "[chic-debug code="
        cursor = WriteChar(cursor, '[');
        cursor = WriteChar(cursor, 'c');
        cursor = WriteChar(cursor, 'h');
        cursor = WriteChar(cursor, 'i');
        cursor = WriteChar(cursor, 'c');
        cursor = WriteChar(cursor, '-');
        cursor = WriteChar(cursor, 'd');
        cursor = WriteChar(cursor, 'e');
        cursor = WriteChar(cursor, 'b');
        cursor = WriteChar(cursor, 'u');
        cursor = WriteChar(cursor, 'g');
        cursor = WriteChar(cursor, ' ');
        cursor = WriteChar(cursor, 'c');
        cursor = WriteChar(cursor, 'o');
        cursor = WriteChar(cursor, 'd');
        cursor = WriteChar(cursor, 'e');
        cursor = WriteChar(cursor, '=');
        cursor = WriteU64(cursor, code);
        // append fields " a=<...>" etc
        cursor = WriteChar(cursor, ' ');
        cursor = WriteChar(cursor, 'a');
        cursor = WriteChar(cursor, '=');
        cursor = WriteU64(cursor, a);
        cursor = WriteChar(cursor, ' ');
        cursor = WriteChar(cursor, 'b');
        cursor = WriteChar(cursor, '=');
        cursor = WriteU64(cursor, b);
        cursor = WriteChar(cursor, ' ');
        cursor = WriteChar(cursor, 'c');
        cursor = WriteChar(cursor, '=');
        cursor = WriteU64(cursor, c);
        cursor = WriteChar(cursor, '\n');
        let totalLen = (usize)(NativePtr.ToIsize(cursor) - NativePtr.ToIsize(start));
        let _ = write(2, start, totalLen);
    }
}
