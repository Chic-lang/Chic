namespace Std.Runtime.Native;
// Self-contained Chic-native string runtime that can build without the bootstrap Std.
@repr(c) public struct ChicStr
{
    public * const @readonly @expose_address byte ptr;
    public usize len;
}
@repr(c) public struct ChicCharSpan
{
    public * const @readonly @expose_address char ptr;
    public usize len;
}
@repr(c) public struct StringInlineBytes64
{
    public byte b00;
    public byte b01;
    public byte b02;
    public byte b03;
    public byte b04;
    public byte b05;
    public byte b06;
    public byte b07;
    public byte b08;
    public byte b09;
    public byte b10;
    public byte b11;
    public byte b12;
    public byte b13;
    public byte b14;
    public byte b15;
    public byte b16;
    public byte b17;
    public byte b18;
    public byte b19;
    public byte b20;
    public byte b21;
    public byte b22;
    public byte b23;
    public byte b24;
    public byte b25;
    public byte b26;
    public byte b27;
    public byte b28;
    public byte b29;
    public byte b30;
    public byte b31;
    public byte b32;
    public byte b33;
    public byte b34;
    public byte b35;
    public byte b36;
    public byte b37;
    public byte b38;
    public byte b39;
    public byte b40;
    public byte b41;
    public byte b42;
    public byte b43;
    public byte b44;
    public byte b45;
    public byte b46;
    public byte b47;
    public byte b48;
    public byte b49;
    public byte b50;
    public byte b51;
    public byte b52;
    public byte b53;
    public byte b54;
    public byte b55;
    public byte b56;
    public byte b57;
    public byte b58;
    public byte b59;
    public byte b60;
    public byte b61;
    public byte b62;
    public byte b63;
}
@repr(c) public struct ChicString
{
    public * mut @expose_address byte ptr;
    public usize len;
    public usize cap;
    public StringInlineBytes32 inline_data;
}
public enum StringError
{
    Success = 0, Utf8 = 1, CapacityOverflow = 2, AllocationFailed = 3, InvalidPointer = 4, OutOfBounds = 5,
}
@repr(c) public struct NumericFormatSpec
{
    public byte flags;
    public byte floatKind;
    public usize width;
    public usize precision;
}
public static class StringRuntime
{
    private const usize INLINE_CAPACITY = 32usize;
    private const usize INLINE_BYTES32_SIZE = sizeof(StringInlineBytes32);
    private const usize INLINE_BYTES64_SIZE = sizeof(StringInlineBytes64);
    private const usize FLOAT_TMP_CAP = 64;
    private const int STR_SUCCESS = 0;
    private const int STR_UTF8 = 1;
    private const int STR_CAPACITY = 2;
    private const int STR_ALLOCATION_FAILED = 3;
    private const int STR_INVALID_POINTER = 4;
    private const int STR_OUT_OF_BOUNDS = 5;
    private const byte ASCII_ZERO = 48;
    private const byte ASCII_DASH = 45;
    private const byte ASCII_DOT = 46;
    private const byte ASCII_SPACE = 32;
    private const byte ASCII_N = 110;
    private const byte ASCII_A = 97;
    private static StringInlineBytes32 ZeroInline32() {
        return new StringInlineBytes32 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0,
        }
        ;
    }
    private static StringInlineBytes64 ZeroInline64() {
        return new StringInlineBytes64 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0, b32 = 0, b33 = 0, b34 = 0, b35 = 0, b36 = 0, b37 = 0, b38 = 0, b39 = 0, b40 = 0, b41 = 0, b42 = 0, b43 = 0, b44 = 0, b45 = 0, b46 = 0, b47 = 0, b48 = 0, b49 = 0, b50 = 0, b51 = 0, b52 = 0, b53 = 0, b54 = 0, b55 = 0, b56 = 0, b57 = 0, b58 = 0, b59 = 0, b60 = 0, b61 = 0, b62 = 0, b63 = 0,
        }
        ;
    }
    private const byte ASCII_A_UPPER = 65;
    private const byte ASCII_I = 105;
    private const byte ASCII_F = 102;
    private const byte ASCII_F_UPPER = 70;
    private const byte ASCII_C = 99;
    private const byte ASCII_D_UPPER = 68;
    private const byte ASCII_P = 112;
    private const byte ASCII_O = 111;
    private const byte ASCII_O_UPPER = 79;
    private const byte ASCII_V = 118;
    private const byte ASCII_V_UPPER = 86;
    private const byte ASCII_L = 108;
    private const byte ASCII_L_UPPER = 76;
    private const byte ASCII_D = 100;
    private const byte ASCII_R_UPPER = 82;
    private const byte ASCII_W = 119;
    private const byte ASCII_B = 98;
    private const byte ASCII_R = 114;
    private const byte ASCII_T = 116;
    private const byte ASCII_T_UPPER = 84;
    private const byte ASCII_U = 117;
    private const byte ASCII_U_UPPER = 85;
    private const byte ASCII_E = 101;
    private const byte ASCII_E_UPPER = 69;
    private const byte ASCII_G = 103;
    private const byte ASCII_G_UPPER = 71;
    private const byte ASCII_S = 115;
    private const byte ASCII_S_UPPER = 83;
    private const byte ASCII_EIGHT = 56;
    private const byte ASCII_SIX = 54;
    private const byte ASCII_NINE = 57;
    private const byte ASCII_A_LOWER = 97;
    private const byte ASCII_B_LOWER = 98;
    private const byte ASCII_C_LOWER = 99;
    private const byte ASCII_D_LOWER = 100;
    private const byte ASCII_E_LOWER = 101;
    private const byte ASCII_F_LOWER = 102;
    private const byte ASCII_PERCENT = 37;
    private const byte ASCII_LPAREN = 40;
    private const byte ASCII_RPAREN = 41;
    private const byte ASCII_X = 88;
    private const byte ASCII_X_LOWER = 120;
    private const usize UTF8_ERROR_LEN = 39usize;
    private const usize CAPACITY_ERROR_LEN = 17usize;
    private const usize ALLOCATION_ERROR_LEN = 17usize;
    private const usize INVALID_POINTER_LEN = 15usize;
    private const usize OUT_OF_BOUNDS_LEN = 13usize;
    private const byte NUM_FMT_HEX = 1u8;
    private const byte NUM_FMT_UPPER = 2u8;
    private const byte NUM_FMT_FLOAT = 4u8;
    private const byte NUM_FMT_HAS_WIDTH = 8u8;
    private const byte NUM_FMT_HAS_PRECISION = 16u8;
    private static usize InlineTag() {
        let shift = ((usize) sizeof(usize) * 8usize) - 1usize;
        return((usize) 1) << shift;
    }
    private static usize CapMask() {
        return InlineTag() - 1usize;
    }
    @extern("C") private unsafe static extern int snprintf(* mut @expose_address byte buffer, usize size, * const @readonly @expose_address byte fmt,
    f64 value);
    private unsafe static ValueConstPtr MakeConstPtr(* const @readonly @expose_address byte ptr, usize len) {
        return new ValueConstPtr {
            Pointer = ptr, Size = len, Alignment = 1
        }
        ;
    }
    private unsafe static ValueMutPtr MakeMutPtr(* mut @expose_address byte ptr, usize len) {
        return new ValueMutPtr {
            Pointer = ptr, Size = len, Alignment = 1
        }
        ;
    }
    private unsafe static ValueMutPtr LocalByteMut(ref byte value) {
        var * mut @expose_address byte raw = & value;
        return new ValueMutPtr {
            Pointer = NativePtr.AsByteMut(raw), Size = 1, Alignment = 1
        }
        ;
    }
    private unsafe static ValueConstPtr LocalByteConst(ref byte value) {
        var * const @readonly @expose_address byte raw = & value;
        return new ValueConstPtr {
            Pointer = NativePtr.AsByteConst(raw), Size = 1, Alignment = 1
        }
        ;
    }
    private unsafe static byte LoadByte(* const @readonly @expose_address byte ptr) {
        if (ptr == null)
        {
            return 0u8;
        }
        return * ptr;
    }
    private unsafe static ValueMutPtr MakeStringMut(* mut ChicString ptr) {
        var * mut @expose_address byte raw = ptr;
        return new ValueMutPtr {
            Pointer = NativePtr.AsByteMut(raw), Size = sizeof(ChicString), Alignment = 1,
        }
        ;
    }
    private unsafe static ValueConstPtr MakeStringConst(* const @readonly ChicString ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return new ValueConstPtr {
            Pointer = NativePtr.AsByteConst(raw), Size = sizeof(ChicString), Alignment = 1,
        }
        ;
    }
    private unsafe static * mut @expose_address byte AsBytePtr(* mut ChicString ptr) {
        var * mut @expose_address byte raw = ptr;
        return NativePtr.AsByteMut(raw);
    }
    private unsafe static * const @readonly @expose_address byte AsBytePtrConst(* const @readonly ChicString ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.AsByteConst(raw);
    }
    private unsafe static ChicString LoadStringRaw(* const @readonly ChicString ptr) {
        var tmp = new ChicString {
            ptr = NativePtr.NullMut(), len = 0, cap = 0, inline_data = ZeroInline32(),
        }
        ;
        if (ptr != null)
        {
            NativeAlloc.Copy(MakeStringMut(& tmp), MakeStringConst(ptr), sizeof(ChicString));
        }
        return tmp;
    }
    private unsafe static ChicString LoadStringAdjusted(* const @readonly ChicString ptr) {
        var tmp = LoadStringRaw(ptr);
        if ( (tmp.cap & InlineTag ()) != 0)
        {
            tmp.ptr = (* mut @expose_address byte)(& mut tmp.inline_data.b00);
        }
        return tmp;
    }
    private unsafe static void StoreString(* mut ChicString dest, ChicString value) {
        if (dest == null)
        {
            return;
        }
        var adjusted = value;
        if ( (adjusted.cap & InlineTag ()) != 0)
        {
            adjusted.ptr = InlinePtr(dest);
        }
        NativeAlloc.Copy(MakeStringMut(dest), MakeStringConst(& adjusted), sizeof(ChicString));
    }
    public unsafe static void StoreByte(* mut @expose_address byte ptr, byte value) {
        if (NativePtr.IsNull (ptr))
        {
            return;
        }
        NativeAlloc.Set(MakeMutPtr(ptr, 1), value, 1);
    }
    private unsafe static usize InlineOffsetBytes() {
        var tmp = new ChicString {
            ptr = NativePtr.NullMut(), len = 0, cap = 0, inline_data = ZeroInline32(),
        }
        ;
        let base = NativePtr.ToIsize(AsBytePtr(& tmp));
        let inlinePtr = NativePtr.ToIsize((* mut @expose_address byte)(& mut tmp.inline_data.b00));
        return(usize)(inlinePtr - base);
    }
    private unsafe static * mut @expose_address byte InlinePtr(* mut ChicString value) {
        if (value == null)
        {
            return NativePtr.NullMut();
        }
        return NativePtr.OffsetMut(AsBytePtr(value), (isize) InlineOffsetBytes());
    }
    private unsafe static * const @readonly @expose_address byte InlinePtrConst(* const @readonly ChicString value) {
        if (value == null)
        {
            return NativePtr.NullConst();
        }
        return NativePtr.OffsetConst(AsBytePtrConst(value), (isize) InlineOffsetBytes());
    }
    private unsafe static void NormalizeInlinePtr(* mut ChicString value) {
        if (value == null)
        {
            return;
        }
        let base_ptr = AsBytePtr(value);
        let base_addr = NativePtr.ToIsize(base_ptr);
        let end_addr = base_addr + (isize) sizeof(ChicString);
        let inline_addr = NativePtr.ToIsize(InlinePtr(value));
        var local = LoadStringRaw(value);
        let cap = local.cap;
        let ptr_addr = NativePtr.ToIsize(local.ptr);
        if ( (cap & InlineTag ()) != 0)
        {
            if (ptr_addr != inline_addr)
            {
                local.ptr = (* mut @expose_address byte)(& mut local.inline_data.b00);
                StoreString(value, local);
            }
            return;
        }
        if (ptr_addr != 0 && ptr_addr >= base_addr && ptr_addr <end_addr)
        {
            local.ptr = (* mut @expose_address byte)(& mut local.inline_data.b00);
            local.cap = (cap & CapMask()) | InlineTag();
            StoreString(value, local);
        }
    }
    private unsafe static bool IsInlinePtr(* const @readonly ChicString value) {
        if (value == null)
        {
            return false;
        }
        var local = LoadStringRaw(value);
        return(local.cap & InlineTag()) != 0;
    }
    private unsafe static usize HeapCapacityPtr(* const @readonly ChicString value) {
        if (value == null)
        {
            return 0;
        }
        var local = LoadStringRaw(value);
        return local.cap & CapMask();
    }
    private unsafe static void InitInline(* mut ChicString value) {
        if (value == null)
        {
            return;
        }
        var local = new ChicString {
            ptr = NativePtr.NullMut(), len = 0, cap = 0, inline_data = ZeroInline32(),
        }
        ;
        local.ptr = (* mut @expose_address byte)(& mut local.inline_data.b00);
        local.len = 0;
        let tagged_cap128 = ((u128) InlineTag()) | (u128) INLINE_CAPACITY;
        local.cap = (usize) tagged_cap128;
        StoreString(value, local);
    }
    private unsafe static * const @readonly @expose_address byte DataPtrConst(* const @readonly ChicString value) {
        if (value == null)
        {
            return NativePtr.NullConst();
        }
        var local = LoadStringRaw(value);
        return(local.cap & InlineTag()) != 0 ?InlinePtrConst(value) : NativePtr.AsConstPtr(local.ptr);
    }
    private unsafe static * mut @expose_address byte DataPtrMut(* mut ChicString value) {
        if (value == null)
        {
            return NativePtr.NullMut();
        }
        var local = LoadStringRaw(value);
        return(local.cap & InlineTag()) != 0 ?InlinePtr(value) : local.ptr;
    }
    private unsafe static * mut @expose_address byte AddMut(* mut @expose_address byte ptr, usize off) {
        return NativePtr.OffsetMut(ptr, (isize) off);
    }
    private unsafe static * const @readonly @expose_address byte AddConst(* const @readonly @expose_address byte ptr, usize off) {
        return NativePtr.OffsetConst(ptr, (isize) off);
    }
    private unsafe static int AppendAlignedBytes(* mut ChicString target, * const @readonly @expose_address byte src, usize len,
    int alignment, int has_alignment) {
        if (target == null)
        {
            return STR_INVALID_POINTER;
        }
        if (len >0 && NativePtr.IsNullConst (src))
        {
            return STR_INVALID_POINTER;
        }
        if (has_alignment == 0 || alignment == 0)
        {
            var slice = new ChicStr {
                ptr = src, len = len
            }
            ;
            return chic_rt_string_push_slice(target, slice);
        }
        let width = (usize)(alignment <0 ?- alignment : alignment);
        let pad = width >len ?width - len : 0usize;
        let leading = alignment >0 ?pad : 0usize;
        let trailing = alignment <0 ?pad : 0usize;
        let total = leading + len + trailing;
        if (total == 0)
        {
            return STR_SUCCESS;
        }
        if (!EnsureCapacity (target, total))
        {
            return STR_ALLOCATION_FAILED;
        }
        var local = LoadStringAdjusted(target);
        var * mut @expose_address byte base_ptr = (local.cap & InlineTag()) != 0 ?(* mut @expose_address byte)(& mut local.inline_data.b00) : local.ptr;
        let start = local.len;
        if (leading >0)
        {
            NativeAlloc.Set(MakeMutPtr(AddMut(base_ptr, start), leading), ASCII_SPACE, leading);
        }
        if (len >0)
        {
            NativeAlloc.Copy(MakeMutPtr(AddMut(base_ptr, start + leading), len), MakeConstPtr(src, len), len);
        }
        if (trailing >0)
        {
            NativeAlloc.Set(MakeMutPtr(AddMut(base_ptr, start + leading + len), trailing), ASCII_SPACE, trailing);
        }
        local.len = local.len + total;
        StoreString(target, local);
        return STR_SUCCESS;
    }
    @allow(all) private unsafe static byte NarrowByteU32(u32 value) {
        var masked = value & 0xFFu32;
        unchecked {
            return(byte) masked;
        }
    }
    @allow(all) private unsafe static byte NarrowByteU128(u128 value) {
        var masked = value & 0xFFu128;
        unchecked {
            return(byte) masked;
        }
    }
    @allow(all) private unsafe static byte NarrowByteI128(i128 value) {
        var masked = value & 0xFFi128;
        unchecked {
            return(byte) masked;
        }
    }
    @allow(all) private unsafe static u128 ToU128Unchecked(i128 value) {
        unchecked {
            return(u128) value;
        }
    }
    private unsafe static u128 AbsI128ToU128(i128 value) {
        if (value >= 0)
        {
            return ToU128Unchecked(value);
        }
        var offset = value + 1i128;
        var magnitude = ToU128Unchecked(0i128 - offset);
        return magnitude + 1u128;
    }
    private unsafe static byte ByteAdd(byte lhs, byte rhs) {
        var sum = (u32) lhs + (u32) rhs;
        return NarrowByteU32(sum);
    }
    private unsafe static byte ParseBoolFormatKind(ChicStr format) {
        var kind = (byte) 0;
        if (!NativePtr.IsNullConst (format.ptr) && format.len >0)
        {
            var start = 0usize;
            var end = format.len;
            while (start <end && LoadByte (AddConst (format.ptr, start)) == ASCII_SPACE)
            {
                start += 1;
            }
            while (end >start && LoadByte (AddConst (format.ptr, end - 1usize)) == ASCII_SPACE)
            {
                end -= 1;
            }
            if (end >start)
            {
                let first = LoadByte(AddConst(format.ptr, start));
                if (first == ASCII_U_UPPER || first == ASCII_U)
                {
                    kind = 1;
                }
                else if (first == ASCII_L_UPPER || first == ASCII_L)
                {
                    kind = 2;
                }
            }
        }
        return kind;
    }
    private static u128 MaskUnsigned(u128 value, u32 bits) {
        if (bits == 0u32)
        {
            return value;
        }
        if (bits >= 128u32)
        {
            return value;
        }
        let one = (u128) 1;
        let mask = (one << bits) - one;
        return value & mask;
    }
    private static u64 MaskLower64(u32 bits) {
        if (bits == 0u32)
        {
            return 0xFFFFFFFF_FFFFFFFFu64;
        }
        if (bits >= 64u32)
        {
            return 0xFFFFFFFF_FFFFFFFFu64;
        }
        let one = (u64) 1;
        return(one << bits) - one;
    }
    private static u32 EffectiveMaskBits(u32 bits, bool hasWidth, usize width) {
        if (bits != 0u32)
        {
            return bits;
        }
        if (hasWidth)
        {
            let candidate = width * 4usize;
            if (candidate >= 128usize)
            {
                return 128u32;
            }
            return(u32) candidate;
        }
        return 0u32;
    }
    private unsafe static usize WriteErrorMessage(int code, * mut @expose_address byte dst) {
        if (code == STR_UTF8)
        {
            StoreByte(dst, 111);
            StoreByte(AddMut(dst, 1), 112);
            StoreByte(AddMut(dst, 2), 101);
            StoreByte(AddMut(dst, 3), 114);
            StoreByte(AddMut(dst, 4), 97);
            StoreByte(AddMut(dst, 5), 116);
            StoreByte(AddMut(dst, 6), 105);
            StoreByte(AddMut(dst, 7), 111);
            StoreByte(AddMut(dst, 8), 110);
            StoreByte(AddMut(dst, 9), 32);
            StoreByte(AddMut(dst, 10), 119);
            StoreByte(AddMut(dst, 11), 111);
            StoreByte(AddMut(dst, 12), 117);
            StoreByte(AddMut(dst, 13), 108);
            StoreByte(AddMut(dst, 14), 100);
            StoreByte(AddMut(dst, 15), 32);
            StoreByte(AddMut(dst, 16), 114);
            StoreByte(AddMut(dst, 17), 101);
            StoreByte(AddMut(dst, 18), 115);
            StoreByte(AddMut(dst, 19), 117);
            StoreByte(AddMut(dst, 20), 108);
            StoreByte(AddMut(dst, 21), 116);
            StoreByte(AddMut(dst, 22), 32);
            StoreByte(AddMut(dst, 23), 105);
            StoreByte(AddMut(dst, 24), 110);
            StoreByte(AddMut(dst, 25), 32);
            StoreByte(AddMut(dst, 26), 105);
            StoreByte(AddMut(dst, 27), 110);
            StoreByte(AddMut(dst, 28), 118);
            StoreByte(AddMut(dst, 29), 97);
            StoreByte(AddMut(dst, 30), 108);
            StoreByte(AddMut(dst, 31), 105);
            StoreByte(AddMut(dst, 32), 100);
            StoreByte(AddMut(dst, 33), 32);
            StoreByte(AddMut(dst, 34), 85);
            StoreByte(AddMut(dst, 35), 84);
            StoreByte(AddMut(dst, 36), 70);
            StoreByte(AddMut(dst, 37), 45);
            StoreByte(AddMut(dst, 38), 56);
            return UTF8_ERROR_LEN;
        }
        if (code == STR_CAPACITY)
        {
            StoreByte(dst, 99);
            StoreByte(AddMut(dst, 1), 97);
            StoreByte(AddMut(dst, 2), 112);
            StoreByte(AddMut(dst, 3), 97);
            StoreByte(AddMut(dst, 4), 99);
            StoreByte(AddMut(dst, 5), 105);
            StoreByte(AddMut(dst, 6), 116);
            StoreByte(AddMut(dst, 7), 121);
            StoreByte(AddMut(dst, 8), 32);
            StoreByte(AddMut(dst, 9), 111);
            StoreByte(AddMut(dst, 10), 118);
            StoreByte(AddMut(dst, 11), 101);
            StoreByte(AddMut(dst, 12), 114);
            StoreByte(AddMut(dst, 13), 102);
            StoreByte(AddMut(dst, 14), 108);
            StoreByte(AddMut(dst, 15), 111);
            StoreByte(AddMut(dst, 16), 119);
            return CAPACITY_ERROR_LEN;
        }
        if (code == STR_ALLOCATION_FAILED)
        {
            StoreByte(dst, 97);
            StoreByte(AddMut(dst, 1), 108);
            StoreByte(AddMut(dst, 2), 108);
            StoreByte(AddMut(dst, 3), 111);
            StoreByte(AddMut(dst, 4), 99);
            StoreByte(AddMut(dst, 5), 97);
            StoreByte(AddMut(dst, 6), 116);
            StoreByte(AddMut(dst, 7), 105);
            StoreByte(AddMut(dst, 8), 111);
            StoreByte(AddMut(dst, 9), 110);
            StoreByte(AddMut(dst, 10), 32);
            StoreByte(AddMut(dst, 11), 102);
            StoreByte(AddMut(dst, 12), 97);
            StoreByte(AddMut(dst, 13), 105);
            StoreByte(AddMut(dst, 14), 108);
            StoreByte(AddMut(dst, 15), 101);
            StoreByte(AddMut(dst, 16), 100);
            return ALLOCATION_ERROR_LEN;
        }
        if (code == STR_INVALID_POINTER)
        {
            StoreByte(dst, 105);
            StoreByte(AddMut(dst, 1), 110);
            StoreByte(AddMut(dst, 2), 118);
            StoreByte(AddMut(dst, 3), 97);
            StoreByte(AddMut(dst, 4), 108);
            StoreByte(AddMut(dst, 5), 105);
            StoreByte(AddMut(dst, 6), 100);
            StoreByte(AddMut(dst, 7), 32);
            StoreByte(AddMut(dst, 8), 112);
            StoreByte(AddMut(dst, 9), 111);
            StoreByte(AddMut(dst, 10), 105);
            StoreByte(AddMut(dst, 11), 110);
            StoreByte(AddMut(dst, 12), 116);
            StoreByte(AddMut(dst, 13), 101);
            StoreByte(AddMut(dst, 14), 114);
            return INVALID_POINTER_LEN;
        }
        if (code == STR_OUT_OF_BOUNDS)
        {
            StoreByte(dst, 111);
            StoreByte(AddMut(dst, 1), 117);
            StoreByte(AddMut(dst, 2), 116);
            StoreByte(AddMut(dst, 3), 32);
            StoreByte(AddMut(dst, 4), 111);
            StoreByte(AddMut(dst, 5), 102);
            StoreByte(AddMut(dst, 6), 32);
            StoreByte(AddMut(dst, 7), 98);
            StoreByte(AddMut(dst, 8), 111);
            StoreByte(AddMut(dst, 9), 117);
            StoreByte(AddMut(dst, 10), 110);
            StoreByte(AddMut(dst, 11), 100);
            StoreByte(AddMut(dst, 12), 115);
            return OUT_OF_BOUNDS_LEN;
        }
        return 0;
    }
    private unsafe static bool EnsureCapacity(* mut ChicString value, usize additional) {
        if (value == null)
        {
            return false;
        }
        NormalizeInlinePtr(value);
        if (additional == 0)
        {
            return true;
        }
        var local = LoadStringAdjusted(value);
        let oldLen = local.len;
        let needed = oldLen + additional;
        if (needed <oldLen)
        {
            return false;
        }
        let isInline = (local.cap & InlineTag()) != 0;
        if (needed <= INLINE_CAPACITY)
        {
            if (!isInline)
            {
                if (!NativePtr.IsNull (local.ptr) && oldLen >0)
                {
                    NativeAlloc.Copy(MakeMutPtr((* mut @expose_address byte)(& mut local.inline_data.b00), oldLen), MakeConstPtr(local.ptr,
                    oldLen), oldLen);
                }
                local.ptr = (* mut @expose_address byte)(& mut local.inline_data.b00);
                let tagged_cap128 = ((u128) InlineTag()) | (u128) INLINE_CAPACITY;
                local.cap = (usize) tagged_cap128;
                StoreString(value, local);
            }
            else
            {
                StoreString(value, local);
            }
            return true;
        }
        if (isInline)
        {
            let newCap = needed <INLINE_CAPACITY * 2 ?INLINE_CAPACITY * 2 : needed;
            var alloc = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = newCap, Alignment = 1
            }
            ;
            if (NativeAlloc.Alloc (newCap, 1, out alloc) != NativeAllocationError.Success) {
                return false;
            }
            if (oldLen >0)
            {
                NativeAlloc.Copy(alloc, MakeConstPtr((* const @readonly @expose_address byte)(& local.inline_data.b00), oldLen),
                oldLen);
            }
            local.ptr = alloc.Pointer;
            local.cap = newCap;
            StoreString(value, local);
            return true;
        }
        let current = local.cap & CapMask();
        if (needed <= current)
        {
            StoreString(value, local);
            return true;
        }
        let newCap2 = current == 0 ?needed : (current * 2 >needed ?current * 2 : needed);
        var alloc2 = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newCap2, Alignment = 1
        }
        ;
        if (NativeAlloc.Alloc (newCap2, 1, out alloc2) != NativeAllocationError.Success) {
            return false;
        }
        if (!NativePtr.IsNull (local.ptr) && oldLen >0)
        {
            NativeAlloc.Copy(alloc2, MakeConstPtr(local.ptr, oldLen), oldLen);
        }
        // Intentionally leak the previous buffer while the native string layout stabilises
        // to avoid freeing mis-tagged inline pointers.
        local.ptr = alloc2.Pointer;
        local.cap = newCap2;
        StoreString(value, local);
        return true;
    }
    private unsafe static void DropHeap(* mut ChicString value) {
        if (value == null)
        {
            return;
        }
        // Temporarily skip freeing to avoid invalid pointer frees while the native
        // string layout is stabilised.
        InitInline(value);
    }
    private unsafe static usize EncodeCodePoint(u32 cp, * mut @expose_address byte dst) {
        if (cp <= 0x7F)
        {
            StoreByte(dst, NarrowByteU32(cp));
            return 1;
        }
        if (cp <= 0x7FF)
        {
            StoreByte(dst, NarrowByteU32(0xC0u32 | (cp >> 6)));
            StoreByte(AddMut(dst, 1), NarrowByteU32(0x80u32 | (cp & 0x3Fu32)));
            return 2;
        }
        if (cp <= 0xFFFF)
        {
            StoreByte(dst, NarrowByteU32(0xE0u32 | (cp >> 12)));
            StoreByte(AddMut(dst, 1), NarrowByteU32(0x80u32 | ((cp >> 6) & 0x3Fu32)));
            StoreByte(AddMut(dst, 2), NarrowByteU32(0x80u32 | (cp & 0x3Fu32)));
            return 3;
        }
        if (cp <= 0x10FFFF)
        {
            StoreByte(dst, NarrowByteU32(0xF0u32 | (cp >> 18)));
            StoreByte(AddMut(dst, 1), NarrowByteU32(0x80u32 | ((cp >> 12) & 0x3Fu32)));
            StoreByte(AddMut(dst, 2), NarrowByteU32(0x80u32 | ((cp >> 6) & 0x3Fu32)));
            StoreByte(AddMut(dst, 3), NarrowByteU32(0x80u32 | (cp & 0x3Fu32)));
            return 4;
        }
        return 0;
    }
    private unsafe static usize FormatUnsigned(u128 value, usize minWidth, * mut @expose_address byte dst) {
        var scratch = ZeroInline64();
        var * mut @expose_address byte buf = & scratch.b00;
        var current = value;
        var count = 0usize;
        if (current == 0)
        {
            StoreByte(buf, ASCII_ZERO);
            count = 1;
        }
        else
        {
            while (current >0)
            {
                let q = current / 10;
                let r = current - (q * 10);
                let digit = NarrowByteU128(r);
                let outv = ByteAdd(ASCII_ZERO, digit);
                StoreByte(AddMut(buf, count), outv);
                count += 1;
                current = q;
            }
        }
        while (count <minWidth)
        {
            StoreByte(AddMut(buf, count), ASCII_ZERO);
            count += 1;
        }
        var i = 0usize;
        while (i <count)
        {
            let src = AddConst(buf, (count - 1usize) - i);
            let dest = AddMut(dst, i);
            NativeAlloc.Copy(MakeMutPtr(dest, 1), MakeConstPtr(src, 1), 1);
            i += 1;
        }
        return count;
    }
    private unsafe static usize FormatSigned(i128 value, usize minWidth, * mut @expose_address byte dst) {
        if (value <0)
        {
            StoreByte(dst, ASCII_DASH);
            let magnitude = AbsI128ToU128(value);
            let needed = minWidth >0 && minWidth >1usize ?minWidth - 1usize : 0usize;
            let inner = FormatUnsigned(magnitude, needed, AddMut(dst, 1));
            return inner + 1;
        }
        return FormatUnsigned(AbsI128ToU128(value), minWidth, dst);
    }
    private unsafe static usize FormatHex64(u64 value, bool upper, usize minWidth, * mut @expose_address byte dst) {
        let letterBase = upper ?ASCII_A_UPPER : ASCII_A_LOWER;
        var digitCount = 0usize;
        var tmp = value;
        let shiftMask = MaskLower64(60u32);
        while (tmp != 0u64)
        {
            digitCount += 1usize;
            tmp = (tmp >> 4u32) & shiftMask;
        }
        if (digitCount == 0usize)
        {
            digitCount = 1usize;
        }
        let width = digitCount <minWidth ?minWidth : digitCount;
        var pos = width;
        var current = value;
        if (current == 0u64)
        {
            pos -= 1usize;
            StoreByte(AddMut(dst, pos), ASCII_ZERO);
        }
        else
        {
            while (current != 0u64)
            {
                pos -= 1usize;
                let nibble = (byte)(current & 0xFu64);
                let digit = nibble <10u8 ?ByteAdd(ASCII_ZERO, nibble) : ByteAdd(letterBase, NarrowByteU32((u32) nibble - 10u32));
                StoreByte(AddMut(dst, pos), digit);
                current = (current >> 4u32) & shiftMask;
            }
        }
        while (pos >0usize)
        {
            pos -= 1usize;
            StoreByte(AddMut(dst, pos), ASCII_ZERO);
        }
        return width;
    }
    private unsafe static usize FormatHexParts(u64 hi, u64 lo, bool upper, usize minWidth, * mut @expose_address byte dst) {
        if (hi == 0u64)
        {
            let written = FormatHex64(lo, upper, minWidth, dst);
            return written;
        }
        let hi_written = FormatHex64(hi, upper, 0usize, dst);
        let low_min = minWidth >hi_written ?minWidth - hi_written : 16usize;
        let low_written = FormatHex64(lo, upper, low_min, AddMut(dst, hi_written));
        return hi_written + low_written;
    }
    private unsafe static usize FormatHex(u128 value, bool upper, usize minWidth, * mut @expose_address byte dst) {
        let lo = (u64) value;
        let hi = (u64)(value >> 64);
        return FormatHexParts(hi, lo, upper, minWidth, dst);
    }
    private unsafe static int ParseNumericFormat(* const @readonly @expose_address byte format_ptr, usize format_len, * mut NumericFormatSpec spec) {
        var specPtr = spec;
        if (!NativePtr.IsNull (specPtr))
        {
            (* specPtr).flags = 0u8;
            (* specPtr).floatKind = 0u8;
            (* specPtr).width = 0usize;
            (* specPtr).precision = 0usize;
        }
        if (NativePtr.IsNullConst (format_ptr) || format_len == 0usize)
        {
            return STR_SUCCESS;
        }
        var flags = 0u8;
        var idx = 0usize;
        let first = * format_ptr;
        var floatKind = 0u8;
        if (first == ASCII_X)
        {
            flags = flags | NUM_FMT_HEX | NUM_FMT_UPPER;
            idx = 1usize;
        }
        else if (first == ASCII_X_LOWER)
        {
            flags = flags | NUM_FMT_HEX;
            idx = 1usize;
        }
        else if (first == ASCII_E_UPPER || first == ASCII_E)
        {
            flags = flags | NUM_FMT_FLOAT;
            if (first == ASCII_E_UPPER)
            {
                flags = flags | NUM_FMT_UPPER;
            }
            floatKind = 2u8;
            idx = 1usize;
        }
        else if (first == ASCII_G_UPPER || first == ASCII_G)
        {
            flags = flags | NUM_FMT_FLOAT;
            if (first == ASCII_G_UPPER)
            {
                flags = flags | NUM_FMT_UPPER;
            }
            floatKind = 3u8;
            idx = 1usize;
        }
        else if (first == ASCII_F_UPPER || first == ASCII_F)
        {
            flags = flags | NUM_FMT_FLOAT;
            if (first == ASCII_F_UPPER)
            {
                flags = flags | NUM_FMT_UPPER;
            }
            floatKind = 1u8;
            idx = 1usize;
        }
        else
        {
            // Unknown format token: accept and leave defaults.
            if (!NativePtr.IsNull (specPtr))
            {
                (* specPtr).flags = flags;
                (* specPtr).floatKind = floatKind;
            }
            return STR_SUCCESS;
        }
        var width = 0usize;
        while (idx <format_len)
        {
            let ch = * NativePtr.OffsetConst(format_ptr, (isize) idx);
            if (ch <48u8 || ch >57u8)
            {
                return STR_INVALID_POINTER;
            }
            width = width * 10usize + (usize)(ch - 48u8);
            idx = idx + 1usize;
        }
        if (width >0usize)
        {
            if ( (flags & NUM_FMT_FLOAT) != 0u8)
            {
                flags = flags | NUM_FMT_HAS_PRECISION;
                if (!NativePtr.IsNull (specPtr))
                {
                    (* specPtr).precision = width;
                }
            }
            else
            {
                flags = flags | NUM_FMT_HAS_WIDTH;
                if (!NativePtr.IsNull (specPtr))
                {
                    (* specPtr).width = width;
                }
            }
        }
        if (!NativePtr.IsNull (specPtr))
        {
            (* specPtr).flags = flags;
            (* specPtr).floatKind = floatKind;
        }
        return STR_SUCCESS;
    }
    private static double Pow10(usize exp) {
        var result = 1.0;
        var i = 0usize;
        while (i <exp)
        {
            result = result * 10.0;
            i += 1;
        }
        return result;
    }
    private static double Pow2I32(i32 exp) {
        if (exp >1024)
        {
            return 1.0 / 0.0;
        }
        if (exp <- 1024)
        {
            return 0.0;
        }
        var result = 1.0;
        if (exp >= 0)
        {
            var i = 0i32;
            while (i <exp)
            {
                result = result * 2.0;
                i += 1;
            }
        }
        else
        {
            var i2 = exp;
            while (i2 <0)
            {
                result = result * 0.5;
                i2 += 1;
            }
        }
        return result;
    }
    private unsafe static usize WriteWithAlignment(* const @readonly @expose_address byte src, usize len, int alignment,
    int has_alignment, * mut @expose_address byte dst) {
        let width = has_alignment != 0 ?(usize)(alignment <0 ?- alignment : alignment) : 0usize;
        let pad = width >len ?width - len : 0usize;
        let leading = has_alignment != 0 && alignment >0 ?pad : 0usize;
        let trailing = has_alignment != 0 && alignment <0 ?pad : 0usize;
        var i = 0usize;
        while (i <leading)
        {
            StoreByte(AddMut(dst, i), ASCII_SPACE);
            i += 1;
        }
        NativeAlloc.Move(MakeMutPtr(AddMut(dst, leading), len), MakeConstPtr(src, len), len);
        var t = 0usize;
        while (t <trailing)
        {
            StoreByte(AddMut(dst, leading + len + t), ASCII_SPACE);
            t += 1;
        }
        return leading + len + trailing;
    }
    private unsafe static bool IsNegativeZeroF64(f64 value) {
        return value == 0.0 && (1.0 / value) <0.0;
    }
    @allow(all) private unsafe static usize FormatFloatFixedPositive(f64 absValue, usize precision, * mut @expose_address byte dst) {
        let clamped = precision >18usize ?18usize : precision;
        let scale = Pow10(clamped);
        var whole : u128 = 0u128;
        var fracInt : u128 = 0u128;
        var wholePart : u128 = 0u128;
        unchecked {
            whole = (u128)(u64) absValue;
            let frac = absValue - (f64)(u64) absValue;
            let mutScaled = (frac * scale) + 0.5;
            fracInt = (u128)(u64) mutScaled;
        }
        wholePart = whole;
        unchecked {
            if (fracInt >= (u128) (u64) scale)
            {
                fracInt = fracInt - (u128)(u64) scale;
                wholePart = wholePart + 1u128;
            }
        }
        var offset = 0usize;
        offset += FormatUnsigned(wholePart, 0usize, AddMut(dst, offset));
        StoreByte(AddMut(dst, offset), ASCII_DOT);
        offset += 1;
        offset += FormatUnsigned(fracInt, clamped, AddMut(dst, offset));
        return offset;
    }
    private unsafe static usize FormatFloatFixed(f64 value, usize precision, * mut @expose_address byte dst) {
        let negative = value <0.0 || IsNegativeZeroF64(value);
        let absValue = negative ?- value : value;
        var offset = 0usize;
        if (negative && !IsNegativeZeroF64 (value))
        {
            StoreByte(AddMut(dst, offset), ASCII_DASH);
            offset += 1;
        }
        else if (IsNegativeZeroF64 (value))
        {
            StoreByte(AddMut(dst, offset), ASCII_DASH);
            offset += 1;
        }
        offset += FormatFloatFixedPositive(absValue, precision, AddMut(dst, offset));
        return offset;
    }
    private unsafe static usize FormatFloatExponent(f64 value, usize precision, bool upper, * mut @expose_address byte dst) {
        let negative = value <0.0 || IsNegativeZeroF64(value);
        var mutAbs = negative ?- value : value;
        var exponent = 0i32;
        if (mutAbs >0.0)
        {
            while (mutAbs >= 10.0)
            {
                mutAbs = mutAbs / 10.0;
                exponent += 1;
            }
            while (mutAbs <1.0)
            {
                mutAbs = mutAbs * 10.0;
                exponent -= 1;
            }
        }
        var offset = 0usize;
        if (negative)
        {
            StoreByte(AddMut(dst, offset), ASCII_DASH);
            offset += 1;
        }
        offset += FormatFloatFixedPositive(mutAbs, precision, AddMut(dst, offset));
        StoreByte(AddMut(dst, offset), upper ?ASCII_E_UPPER : ASCII_E);
        offset += 1;
        StoreByte(AddMut(dst, offset), exponent >= 0 ?ASCII_PLUS : ASCII_DASH);
        offset += 1;
        let expAbs = exponent >= 0 ?(u32) exponent : (u32)(- exponent);
        offset += FormatUnsigned(expAbs, 2usize, AddMut(dst, offset));
        return offset;
    }
    private unsafe static u64 F64Bits(f64 value) {
        var tmp = value;
        return * (* const @readonly @expose_address u64)(& tmp);
    }
    private unsafe static bool IsNaN64(f64 value) {
        let bits = F64Bits(value);
        let exp = (bits >> 52u64) & 0x7FFu64;
        let mantissa = bits & 0x000FFFFFFFFFFFFFu64;
        return exp == 0x7FFu64 && mantissa != 0u64;
    }
    private unsafe static bool IsPosInf64(f64 value) {
        let bits = F64Bits(value);
        let sign = (bits >> 63u64) != 0u64;
        let exp = (bits >> 52u64) & 0x7FFu64;
        let mantissa = bits & 0x000FFFFFFFFFFFFFu64;
        return !sign && exp == 0x7FFu64 && mantissa == 0u64;
    }
    private unsafe static bool IsNegInf64(f64 value) {
        let bits = F64Bits(value);
        let sign = (bits >> 63u64) != 0u64;
        let exp = (bits >> 52u64) & 0x7FFu64;
        let mantissa = bits & 0x000FFFFFFFFFFFFFu64;
        return sign && exp == 0x7FFu64 && mantissa == 0u64;
    }
    private unsafe static usize FormatFloatValue(f64 value, byte floatKind, bool hasPrecision, usize precision, bool upper,
    * mut @expose_address byte dst) {
        if (IsNaN64 (value))
        {
            StoreByte(dst, ASCII_N);
            StoreByte(AddMut(dst, 1), ASCII_A);
            StoreByte(AddMut(dst, 2), ASCII_N);
            return 3;
        }
        if (IsPosInf64 (value))
        {
            StoreByte(dst, ASCII_I);
            StoreByte(AddMut(dst, 1), ASCII_N);
            StoreByte(AddMut(dst, 2), ASCII_F);
            return 3;
        }
        if (IsNegInf64 (value))
        {
            StoreByte(dst, ASCII_DASH);
            StoreByte(AddMut(dst, 1), ASCII_I);
            StoreByte(AddMut(dst, 2), ASCII_N);
            StoreByte(AddMut(dst, 3), ASCII_F);
            return 4;
        }
        var kind = floatKind;
        let effPrecision = hasPrecision ?precision : (kind == 2 || kind == 3 ?6usize : 2usize);
        if (kind == 2)
        {
            return FormatFloatExponent(value, effPrecision, upper, dst);
        }
        if (kind == 3)
        {
            // General format: choose exponent for very small/large magnitudes, otherwise fixed.
            let absVal = value <0.0 ?- value : value;
            if ( (absVal != 0.0 && absVal <0.0001) || absVal >= Pow10 (effPrecision + 1usize))
            {
                return FormatFloatExponent(value, effPrecision, upper, dst);
            }
            return FormatFloatFixed(value, effPrecision, dst);
        }
        return FormatFloatFixed(value, effPrecision, dst);
    }
    private unsafe static usize FormatBool(bool value, int alignment, int has_alignment, ChicStr format, * mut @expose_address byte dst) {
        let formatKind = ParseBoolFormatKind(format);
        var b0 = (byte) 0;
        var b1 = (byte) 0;
        var b2 = (byte) 0;
        var b3 = (byte) 0;
        var b4 = (byte) 0;
        if (value)
        {
            if (formatKind == 1)
            {
                b0 = ASCII_T_UPPER;
                b1 = ASCII_R_UPPER;
                b2 = ASCII_U_UPPER;
                b3 = ASCII_E_UPPER;
            }
            else if (formatKind == 2)
            {
                b0 = ASCII_T;
                b1 = ASCII_R;
                b2 = ASCII_U;
                b3 = ASCII_E;
            }
            else
            {
                b0 = ASCII_T_UPPER;
                b1 = ASCII_R;
                b2 = ASCII_U;
                b3 = ASCII_E;
            }
        }
        else
        {
            if (formatKind == 1)
            {
                b0 = ASCII_F_UPPER;
                b1 = ASCII_A_UPPER;
                b2 = ASCII_L_UPPER;
                b3 = ASCII_S_UPPER;
                b4 = ASCII_E_UPPER;
            }
            else if (formatKind == 2)
            {
                b0 = ASCII_F;
                b1 = ASCII_A;
                b2 = ASCII_L;
                b3 = ASCII_S;
                b4 = ASCII_E;
            }
            else
            {
                b0 = ASCII_F_UPPER;
                b1 = ASCII_A;
                b2 = ASCII_L;
                b3 = ASCII_S;
                b4 = ASCII_E;
            }
        }
        let word_len = value ?4usize : 5usize;
        let width = has_alignment != 0 ?(usize)(alignment <0 ?- alignment : alignment) : 0usize;
        let pad = width >word_len ?width - word_len : 0usize;
        let leading = has_alignment != 0 && alignment >0 ?pad : 0usize;
        let trailing = has_alignment != 0 && alignment <0 ?pad : 0usize;
        var i = 0usize;
        while (i <leading)
        {
            StoreByte(AddMut(dst, i), ASCII_SPACE);
            i += 1;
        }
        var offset = leading;
        StoreByte(AddMut(dst, offset), b0);
        StoreByte(AddMut(dst, offset + 1usize), b1);
        StoreByte(AddMut(dst, offset + 2usize), b2);
        StoreByte(AddMut(dst, offset + 3usize), b3);
        if (!value)
        {
            StoreByte(AddMut(dst, offset + 4usize), b4);
        }
        offset += word_len;
        var t = 0usize;
        while (t <trailing)
        {
            StoreByte(AddMut(dst, offset + t), ASCII_SPACE);
            t += 1;
        }
        return leading + word_len + trailing;
    }
    @allow(all) private unsafe static f32 HalfToF32(u16 bits) {
        let sign = (bits & 0x8000) != 0;
        let exponent = (bits >> 10) & 0x1F;
        let mantissa = bits & 0x3FF;
        if (exponent == 0 && mantissa == 0)
        {
            return sign ?- 0.0f32 : 0.0f32;
        }
        var value = 0.0f32;
        if (exponent == 0)
        {
            // Subnormal: exponent is fixed at -14 with no implicit leading 1.
            var mutValue = 0.0f32;
            unchecked {
                mutValue = (f32) mantissa * (1.0f32 / (f32)(1u32 << 10));
            }
            var i = 0i32;
            while (i <14)
            {
                mutValue = mutValue * 0.5f32;
                i += 1;
            }
            value = mutValue;
        }
        else if (exponent == 0x1F)
        {
            return sign ?- (0.0f32 / 0.0f32) : (0.0f32 / 0.0f32);
        }
        else
        {
            let exp = (i32) exponent - 15;
            var base = 1.0f32;
            unchecked {
                base = 1.0f32 + ((f32) mantissa * (1.0f32 / (f32)(1u32 << 10)));
            }
            var scale = 1.0f32;
            if (exp >0)
            {
                var i2 = 0i32;
                while (i2 <exp)
                {
                    scale = scale * 2.0f32;
                    i2 += 1;
                }
            }
            else if (exp <0)
            {
                var i3 = exp;
                while (i3 <0)
                {
                    scale = scale * 0.5f32;
                    i3 += 1;
                }
            }
            value = base * scale;
        }
        return sign ?- value : value;
    }
    private unsafe static usize FormatHalf(u16 bits, byte floatKind, bool hasPrecision, usize precision, bool upper, * mut @expose_address byte dst) {
        let sign = (bits & 0x8000) != 0;
        let exponent = (bits >> 10) & 0x1F;
        let mantissa = bits & 0x3FF;
        if (exponent == 0 && mantissa == 0)
        {
            if (sign)
            {
                StoreByte(dst, ASCII_DASH);
                StoreByte(AddMut(dst, 1), ASCII_ZERO);
                StoreByte(AddMut(dst, 2), ASCII_DOT);
                StoreByte(AddMut(dst, 3), ASCII_ZERO);
                return 4;
            }
            StoreByte(dst, ASCII_ZERO);
            StoreByte(AddMut(dst, 1), ASCII_DOT);
            StoreByte(AddMut(dst, 2), ASCII_ZERO);
            return 3;
        }
        var value = HalfToF32(bits);
        return FormatFloatValue((f64) value, floatKind, hasPrecision, precision, upper, dst);
    }
    private unsafe static usize FormatF128Nan(u128 bits, * mut @expose_address byte dst) {
        StoreByte(dst, ASCII_N);
        StoreByte(AddMut(dst, 1), ASCII_A);
        StoreByte(AddMut(dst, 2), ASCII_N);
        StoreByte(AddMut(dst, 3), ASCII_LPAREN);
        StoreByte(AddMut(dst, 4), ASCII_ZERO);
        StoreByte(AddMut(dst, 5), ASCII_X_LOWER);
        let written = FormatHex(bits, false, 32usize, AddMut(dst, 6));
        StoreByte(AddMut(dst, 6 + written), ASCII_RPAREN);
        return 6 + written + 1usize;
    }
    @allow(all) private unsafe static usize FormatF128(u128 bits, byte floatKind, bool hasPrecision, usize precision, bool upper,
    * mut @expose_address byte dst) {
        let sign = (bits & (1u128 << 127)) != 0;
        let exponent = (u32)((bits >> 112) & 0x7FFFu32);
        let mantissa = bits & 0x0000FFFF_FFFFFFFF_FFFFFFFF_FFFFFFFFu128;
        if (exponent == 0x7FFFu32)
        {
            if (mantissa == 0u128)
            {
                // Infinity.
                var offset = 0usize;
                if (sign)
                {
                    StoreByte(dst, ASCII_DASH);
                    offset = 1usize;
                }
                StoreByte(AddMut(dst, offset), ASCII_I);
                StoreByte(AddMut(dst, offset + 1usize), ASCII_N);
                StoreByte(AddMut(dst, offset + 2usize), ASCII_F);
                return offset + 3usize;
            }
            return FormatF128Nan(bits, dst);
        }
        var value = 0.0;
        if (exponent == 0u32)
        {
            // Subnormal treated as zero for formatting purposes.
            value = 0.0;
        }
        else
        {
            let exp = (i32) exponent - 16383;
            let mantissa_high = (u64)(mantissa >> (112 - 52));
            var fraction = 0.0;
            unchecked {
                fraction = (f64) mantissa_high / (f64)(1u64 << 52);
            }
            let base = 1.0 + fraction;
            let scale = Pow2I32(exp);
            value = base * scale;
        }
        if (sign)
        {
            value = - value;
        }
        return FormatFloatValue(value, floatKind, hasPrecision, precision, upper, dst);
    }
    @extern("C") @export("chic_rt_string_get_ptr") public unsafe static * mut @expose_address byte chic_rt_string_get_ptr(* const @readonly ChicString value) {
        if (value == null)
        {
            return NativePtr.NullMut();
        }
        var local = LoadStringRaw(value);
        return(local.cap & InlineTag()) != 0 ?NativePtr.AsMutPtr(InlinePtrConst(value)) : local.ptr;
    }
    @extern("C") @export("chic_rt_string_set_ptr") public unsafe static void chic_rt_string_set_ptr(* mut ChicString value,
    * mut @expose_address byte ptr) {
        if (value == null)
        {
            return;
        }
        var local = LoadStringRaw(value);
        local.ptr = ptr;
        StoreString(value, local);
    }
    @extern("C") @export("chic_rt_string_get_len") public unsafe static usize chic_rt_string_get_len(* const @readonly ChicString value) {
        if (value == null)
        {
            return 0;
        }
        var local = LoadStringRaw(value);
        return local.len;
    }
    @extern("C") @export("chic_rt_string_set_len") public unsafe static void chic_rt_string_set_len(* mut ChicString value,
    usize len) {
        if (value == null)
        {
            return;
        }
        var local = LoadStringRaw(value);
        local.len = len;
        StoreString(value, local);
    }
    @extern("C") @export("chic_rt_string_get_cap") public unsafe static usize chic_rt_string_get_cap(* const @readonly ChicString value) {
        if (value == null)
        {
            return 0;
        }
        var local = LoadStringRaw(value);
        return local.cap & CapMask();
    }
    @extern("C") @export("chic_rt_string_set_cap") public unsafe static void chic_rt_string_set_cap(* mut ChicString value,
    usize cap) {
        if (value == null)
        {
            return;
        }
        var local = LoadStringRaw(value);
        let masked_cap = cap & CapMask();
        var tagged_cap = masked_cap;
        if ( (local.cap & InlineTag ()) != 0)
        {
            let tag128 = ((u128) InlineTag()) | (u128) masked_cap;
            tagged_cap = (usize) tag128;
        }
        local.cap = tagged_cap;
        StoreString(value, local);
    }
    @extern("C") @export("chic_rt_string_inline_ptr") public unsafe static * mut @expose_address byte chic_rt_string_inline_ptr(* mut ChicString value) {
        if (value == null)
        {
            return NativePtr.NullMut();
        }
        return InlinePtr(value);
    }
    @extern("C") @export("chic_rt_string_inline_capacity") public unsafe static usize chic_rt_string_inline_capacity() {
        return INLINE_CAPACITY;
    }
    @extern("C") @export("chic_rt_string_as_slice") public unsafe static ChicStr chic_rt_string_as_slice(* const @readonly ChicString value) {
        if (value == null)
        {
            return new ChicStr {
                ptr = NativePtr.NullConst(), len = 0
            }
            ;
        }
        let local = LoadStringRaw(value);
        if (local.len == 0usize)
        {
            return new ChicStr {
                ptr = NativePtr.NullConst(), len = 0
            }
            ;
        }
        let raw_ptr = NativePtr.AsConstPtr(local.ptr);
        if (raw_ptr == null)
        {
            return new ChicStr {
                ptr = NativePtr.NullConst(), len = 0
            }
            ;
        }
        // Strings use inline storage for small values. Returning a pointer directly into the
        // string struct becomes invalid as soon as the string is passed by value (stack copy).
        // Prefer an out-of-struct pointer when we have one (string ABI currently forwards `ptr`
        // even when the inline buffer isn't copied). Otherwise, copy into a thread-local scratch
        // buffer and return that view.
        if ( (local.cap & InlineTag ()) != 0)
        {
            let inline_ptr = InlinePtrConst(value);
            let inline_first = LoadByte(inline_ptr);
            if (inline_first == 0u8)
            {
                return new ChicStr {
                    ptr = raw_ptr, len = local.len
                }
                ;
            }
            let idx = NextUtf8SliceScratch();
            let buffer = EnsureUtf8SliceScratchByIdx(idx, local.len);
            if (buffer == null)
            {
                return new ChicStr {
                    ptr = NativePtr.NullConst(), len = 0
                }
                ;
            }
            NativeAlloc.Copy(MakeMutPtr(buffer, local.len), MakeConstPtr(inline_ptr, local.len), local.len);
            return new ChicStr {
                ptr = NativePtr.AsConstPtr(buffer), len = local.len
            }
            ;
        }
        return new ChicStr {
            ptr = raw_ptr, len = local.len
        }
        ;
    }
    // UTF-8 slice scratch buffers used to back `chic_rt_string_as_slice` for inline strings.
    @threadlocal private static uint _utf8_slice_scratch_cursor;
    @threadlocal private static * mut @expose_address byte _utf8_slice_scratch0_ptr;
    @threadlocal private static usize _utf8_slice_scratch0_size;
    @threadlocal private static * mut @expose_address byte _utf8_slice_scratch1_ptr;
    @threadlocal private static usize _utf8_slice_scratch1_size;
    @threadlocal private static * mut @expose_address byte _utf8_slice_scratch2_ptr;
    @threadlocal private static usize _utf8_slice_scratch2_size;
    @threadlocal private static * mut @expose_address byte _utf8_slice_scratch3_ptr;
    @threadlocal private static usize _utf8_slice_scratch3_size;
    private static uint NextUtf8SliceScratch() {
        let idx = _utf8_slice_scratch_cursor & 3u;
        _utf8_slice_scratch_cursor = _utf8_slice_scratch_cursor + 1u;
        return idx;
    }
    private unsafe static * mut @expose_address byte EnsureUtf8SliceScratchByIdx(uint idx, usize required_bytes) {
        if (required_bytes == 0usize)
        {
            return NativePtr.NullMut();
        }
        if (idx == 0u)
        {
            return EnsureUtf8SliceScratch0(required_bytes);
        }
        if (idx == 1u)
        {
            return EnsureUtf8SliceScratch1(required_bytes);
        }
        if (idx == 2u)
        {
            return EnsureUtf8SliceScratch2(required_bytes);
        }
        return EnsureUtf8SliceScratch3(required_bytes);
    }
    private unsafe static * mut @expose_address byte EnsureUtf8SliceScratch0(usize required_bytes) {
        let align = 1usize;
        if (_utf8_slice_scratch0_ptr != null && _utf8_slice_scratch0_size >= required_bytes)
        {
            return _utf8_slice_scratch0_ptr;
        }
        if (_utf8_slice_scratch0_ptr != null && _utf8_slice_scratch0_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf8_slice_scratch0_ptr, Size = _utf8_slice_scratch0_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf8_slice_scratch0_ptr = NativePtr.NullMut();
            _utf8_slice_scratch0_size = 0usize;
            return NativePtr.NullMut();
        }
        _utf8_slice_scratch0_ptr = alloc.Pointer;
        _utf8_slice_scratch0_size = required_bytes;
        return alloc.Pointer;
    }
    private unsafe static * mut @expose_address byte EnsureUtf8SliceScratch1(usize required_bytes) {
        let align = 1usize;
        if (_utf8_slice_scratch1_ptr != null && _utf8_slice_scratch1_size >= required_bytes)
        {
            return _utf8_slice_scratch1_ptr;
        }
        if (_utf8_slice_scratch1_ptr != null && _utf8_slice_scratch1_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf8_slice_scratch1_ptr, Size = _utf8_slice_scratch1_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf8_slice_scratch1_ptr = NativePtr.NullMut();
            _utf8_slice_scratch1_size = 0usize;
            return NativePtr.NullMut();
        }
        _utf8_slice_scratch1_ptr = alloc.Pointer;
        _utf8_slice_scratch1_size = required_bytes;
        return alloc.Pointer;
    }
    private unsafe static * mut @expose_address byte EnsureUtf8SliceScratch2(usize required_bytes) {
        let align = 1usize;
        if (_utf8_slice_scratch2_ptr != null && _utf8_slice_scratch2_size >= required_bytes)
        {
            return _utf8_slice_scratch2_ptr;
        }
        if (_utf8_slice_scratch2_ptr != null && _utf8_slice_scratch2_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf8_slice_scratch2_ptr, Size = _utf8_slice_scratch2_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf8_slice_scratch2_ptr = NativePtr.NullMut();
            _utf8_slice_scratch2_size = 0usize;
            return NativePtr.NullMut();
        }
        _utf8_slice_scratch2_ptr = alloc.Pointer;
        _utf8_slice_scratch2_size = required_bytes;
        return alloc.Pointer;
    }
    private unsafe static * mut @expose_address byte EnsureUtf8SliceScratch3(usize required_bytes) {
        let align = 1usize;
        if (_utf8_slice_scratch3_ptr != null && _utf8_slice_scratch3_size >= required_bytes)
        {
            return _utf8_slice_scratch3_ptr;
        }
        if (_utf8_slice_scratch3_ptr != null && _utf8_slice_scratch3_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf8_slice_scratch3_ptr, Size = _utf8_slice_scratch3_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf8_slice_scratch3_ptr = NativePtr.NullMut();
            _utf8_slice_scratch3_size = 0usize;
            return NativePtr.NullMut();
        }
        _utf8_slice_scratch3_ptr = alloc.Pointer;
        _utf8_slice_scratch3_size = required_bytes;
        return alloc.Pointer;
    }
    // UTF-8 -> UTF-16 decode scratch buffers.
    //
    // These are thread-local ring buffers used to back `chic_rt_string_as_chars`/`chic_rt_str_as_chars`.
    // The returned spans remain valid until the next call that reuses the same slot on the same thread.
    // This is a bootstrap-friendly compromise until `string` grows a stable UTF-16 backing store.
    @threadlocal private static uint _utf16_scratch_cursor;
    @threadlocal private static * mut @expose_address byte _utf16_scratch0_ptr;
    @threadlocal private static usize _utf16_scratch0_size;
    @threadlocal private static * mut @expose_address byte _utf16_scratch1_ptr;
    @threadlocal private static usize _utf16_scratch1_size;
    @threadlocal private static * mut @expose_address byte _utf16_scratch2_ptr;
    @threadlocal private static usize _utf16_scratch2_size;
    @threadlocal private static * mut @expose_address byte _utf16_scratch3_ptr;
    @threadlocal private static usize _utf16_scratch3_size;
    private static uint NextUtf16Scratch() {
        let idx = _utf16_scratch_cursor & 3u;
        _utf16_scratch_cursor = _utf16_scratch_cursor + 1u;
        return idx;
    }
    private unsafe static * mut @expose_address char EnsureUtf16ScratchByIdx(uint idx, usize required_units) {
        if (required_units == 0usize)
        {
            return(* mut @expose_address char) NativePtr.NullMut();
        }
        if (idx == 0u)
        {
            return EnsureUtf16Scratch0(required_units);
        }
        if (idx == 1u)
        {
            return EnsureUtf16Scratch1(required_units);
        }
        if (idx == 2u)
        {
            return EnsureUtf16Scratch2(required_units);
        }
        return EnsureUtf16Scratch3(required_units);
    }
    private unsafe static * mut @expose_address char EnsureUtf16Scratch0(usize required_units) {
        let align = sizeof(char);
        let required_bytes = required_units * align;
        if (_utf16_scratch0_ptr != null && _utf16_scratch0_size >= required_bytes)
        {
            return(* mut @expose_address char) _utf16_scratch0_ptr;
        }
        if (_utf16_scratch0_ptr != null && _utf16_scratch0_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf16_scratch0_ptr, Size = _utf16_scratch0_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf16_scratch0_ptr = NativePtr.NullMut();
            _utf16_scratch0_size = 0usize;
            return(* mut @expose_address char) NativePtr.NullMut();
        }
        _utf16_scratch0_ptr = alloc.Pointer;
        _utf16_scratch0_size = required_bytes;
        return(* mut @expose_address char) alloc.Pointer;
    }
    private unsafe static * mut @expose_address char EnsureUtf16Scratch1(usize required_units) {
        let align = sizeof(char);
        let required_bytes = required_units * align;
        if (_utf16_scratch1_ptr != null && _utf16_scratch1_size >= required_bytes)
        {
            return(* mut @expose_address char) _utf16_scratch1_ptr;
        }
        if (_utf16_scratch1_ptr != null && _utf16_scratch1_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf16_scratch1_ptr, Size = _utf16_scratch1_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf16_scratch1_ptr = NativePtr.NullMut();
            _utf16_scratch1_size = 0usize;
            return(* mut @expose_address char) NativePtr.NullMut();
        }
        _utf16_scratch1_ptr = alloc.Pointer;
        _utf16_scratch1_size = required_bytes;
        return(* mut @expose_address char) alloc.Pointer;
    }
    private unsafe static * mut @expose_address char EnsureUtf16Scratch2(usize required_units) {
        let align = sizeof(char);
        let required_bytes = required_units * align;
        if (_utf16_scratch2_ptr != null && _utf16_scratch2_size >= required_bytes)
        {
            return(* mut @expose_address char) _utf16_scratch2_ptr;
        }
        if (_utf16_scratch2_ptr != null && _utf16_scratch2_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf16_scratch2_ptr, Size = _utf16_scratch2_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf16_scratch2_ptr = NativePtr.NullMut();
            _utf16_scratch2_size = 0usize;
            return(* mut @expose_address char) NativePtr.NullMut();
        }
        _utf16_scratch2_ptr = alloc.Pointer;
        _utf16_scratch2_size = required_bytes;
        return(* mut @expose_address char) alloc.Pointer;
    }
    private unsafe static * mut @expose_address char EnsureUtf16Scratch3(usize required_units) {
        let align = sizeof(char);
        let required_bytes = required_units * align;
        if (_utf16_scratch3_ptr != null && _utf16_scratch3_size >= required_bytes)
        {
            return(* mut @expose_address char) _utf16_scratch3_ptr;
        }
        if (_utf16_scratch3_ptr != null && _utf16_scratch3_size >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = _utf16_scratch3_ptr, Size = _utf16_scratch3_size, Alignment = align,
            }
            );
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = required_bytes, Alignment = align,
        }
        ;
        if (NativeAlloc.Alloc (required_bytes, align, out alloc) != NativeAllocationError.Success) {
            _utf16_scratch3_ptr = NativePtr.NullMut();
            _utf16_scratch3_size = 0usize;
            return(* mut @expose_address char) NativePtr.NullMut();
        }
        _utf16_scratch3_ptr = alloc.Pointer;
        _utf16_scratch3_size = required_bytes;
        return(* mut @expose_address char) alloc.Pointer;
    }
    private unsafe static void StoreChar(* mut @expose_address char base, usize index, char value) {
        let offset = (isize)(index * sizeof(char));
        let ptr = (* mut @expose_address char) NativePtr.OffsetMut((* mut @expose_address byte) base, offset);
        * ptr = value;
    }
    private unsafe static usize DecodeUtf8ToUtf16(* const @readonly @expose_address byte src, usize src_len, * mut @expose_address char dest,
    usize dest_cap) {
        if (src == null || src_len == 0usize || dest == null || dest_cap == 0usize)
        {
            return 0usize;
        }
        let replacement = (char) 0xFFFDu16;
        var i = 0usize;
        var written = 0usize;
        while (i <src_len && written <dest_cap)
        {
            let b0 = (uint) LoadByte(AddConst(src, i));
            if (b0 <128u)
            {
                StoreChar(dest, written, (char)(ushort) b0);
                written = written + 1usize;
                i = i + 1usize;
                continue;
            }
            if ( (b0 & 224u) == 192u && i + 1usize <src_len)
            {
                let b1 = (uint) LoadByte(AddConst(src, i + 1usize));
                if ( (b1 & 192u) != 128u)
                {
                    StoreChar(dest, written, replacement);
                    written = written + 1usize;
                    i = i + 1usize;
                    continue;
                }
                let cp = ((b0 & 31u) << 6) | (b1 & 63u);
                if (cp <128u)
                {
                    StoreChar(dest, written, replacement);
                    written = written + 1usize;
                    i = i + 1usize;
                    continue;
                }
                StoreChar(dest, written, (char)(ushort) cp);
                written = written + 1usize;
                i = i + 2usize;
                continue;
            }
            if ( (b0 & 240u) == 224u && i + 2usize <src_len)
            {
                let b1 = (uint) LoadByte(AddConst(src, i + 1usize));
                let b2 = (uint) LoadByte(AddConst(src, i + 2usize));
                if ( (b1 & 192u) != 128u || (b2 & 192u) != 128u)
                {
                    StoreChar(dest, written, replacement);
                    written = written + 1usize;
                    i = i + 1usize;
                    continue;
                }
                let cp = ((b0 & 15u) << 12) | ((b1 & 63u) << 6) | (b2 & 63u);
                if (cp <2048u || (cp >= 55296u && cp <= 57343u))
                {
                    StoreChar(dest, written, replacement);
                    written = written + 1usize;
                    i = i + 1usize;
                    continue;
                }
                StoreChar(dest, written, (char)(ushort) cp);
                written = written + 1usize;
                i = i + 3usize;
                continue;
            }
            if ( (b0 & 248u) == 240u && i + 3usize <src_len)
            {
                let b1 = (uint) LoadByte(AddConst(src, i + 1usize));
                let b2 = (uint) LoadByte(AddConst(src, i + 2usize));
                let b3 = (uint) LoadByte(AddConst(src, i + 3usize));
                if ( (b1 & 192u) != 128u || (b2 & 192u) != 128u || (b3 & 192u) != 128u)
                {
                    StoreChar(dest, written, replacement);
                    written = written + 1usize;
                    i = i + 1usize;
                    continue;
                }
                let cp = ((b0 & 7u) << 18) | ((b1 & 63u) << 12) | ((b2 & 63u) << 6) | (b3 & 63u);
                if (cp <65536u || cp >1114111u || written + 1usize >= dest_cap)
                {
                    StoreChar(dest, written, replacement);
                    written = written + 1usize;
                    i = i + 1usize;
                    continue;
                }
                let scalar = cp - 65536u;
                let high = 55296u + (scalar >> 10);
                let low = 56320u + (scalar & 1023u);
                StoreChar(dest, written, (char)(ushort) high);
                StoreChar(dest, written + 1usize, (char)(ushort) low);
                written = written + 2usize;
                i = i + 4usize;
                continue;
            }
            StoreChar(dest, written, replacement);
            written = written + 1usize;
            i = i + 1usize;
        }
        return written;
    }
    @extern("C") @export("chic_rt_string_as_chars") public unsafe static ChicCharSpan chic_rt_string_as_chars(* const @readonly ChicString value) {
        if (value == null)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        let local = LoadStringRaw(value);
        if (local.len == 0usize)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        let raw_ptr = NativePtr.AsConstPtr(local.ptr);
        if (raw_ptr == null)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        var raw = raw_ptr;
        if ( (local.cap & InlineTag ()) != 0)
        {
            let inline_ptr = InlinePtrConst(value);
            let inline_first = LoadByte(inline_ptr);
            if (inline_first != 0u8)
            {
                raw = inline_ptr;
            }
        }
        let idx = NextUtf16Scratch();
        let buffer = EnsureUtf16ScratchByIdx(idx, local.len);
        if (buffer == null)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        let decoded = DecodeUtf8ToUtf16(raw, local.len, buffer, local.len);
        return new ChicCharSpan {
            ptr = (* const @readonly @expose_address char) buffer, len = decoded
        }
        ;
    }
    @extern("C") @export("chic_rt_str_as_chars") public unsafe static ChicCharSpan chic_rt_str_as_chars(ChicStr slice) {
        if (slice.len == 0)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        if (slice.ptr == null)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        let idx = NextUtf16Scratch();
        let buffer = EnsureUtf16ScratchByIdx(idx, slice.len);
        if (buffer == null)
        {
            return new ChicCharSpan {
                ptr = (* const @readonly @expose_address char) NativePtr.NullConst(), len = 0
            }
            ;
        }
        let decoded = DecodeUtf8ToUtf16(slice.ptr, slice.len, buffer, slice.len);
        return new ChicCharSpan {
            ptr = (* const @readonly @expose_address char) buffer, len = decoded
        }
        ;
    }
    @extern("C") @export("chic_rt_string_new") public unsafe static ChicString chic_rt_string_new() {
        var value = new ChicString {
            ptr = NativePtr.NullMut(), len = 0, cap = 0, inline_data = ZeroInline32(),
        }
        ;
        InitInline(& value);
        return value;
    }
    @extern("C") @export("chic_rt_string_with_capacity") public unsafe static ChicString chic_rt_string_with_capacity(usize capacity) {
        if (capacity == 0 || capacity <= INLINE_CAPACITY)
        {
            return chic_rt_string_new();
        }
        var value = chic_rt_string_new();
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = capacity, Alignment = 1
        }
        ;
        if (NativeAlloc.Alloc (capacity, 1, out alloc) != NativeAllocationError.Success) {
            return new ChicString {
                ptr = NativePtr.NullMut(), len = 0, cap = 0, inline_data = ZeroInline32(),
            }
            ;
        }
        value.ptr = alloc.Pointer;
        value.cap = capacity;
        return value;
    }
    @extern("C") @export("chic_rt_string_drop") public unsafe static void chic_rt_string_drop(* mut ChicString target) {
        if (target == null)
        {
            return;
        }
        NormalizeInlinePtr(target);
        let local = LoadStringRaw(target);
        let heap_cap = local.cap & CapMask();
        if ( (local.cap & InlineTag ()) == 0 && !NativePtr.IsNull (local.ptr) && heap_cap >0)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = local.ptr, Size = heap_cap, Alignment = 1,
            }
            );
        }
        InitInline(target);
    }
    @extern("C") @export("chic_rt_string_push_slice") public unsafe static int chic_rt_string_push_slice(* mut ChicString target,
    ChicStr slice) {
        if (target == null)
        {
            return STR_INVALID_POINTER;
        }
        if (slice.len == 0)
        {
            return STR_SUCCESS;
        }
        if (NativePtr.IsNullConst (slice.ptr))
        {
            return STR_INVALID_POINTER;
        }
        NormalizeInlinePtr(target);
        var local = LoadStringAdjusted(target);
        if (!EnsureCapacity (target, slice.len))
        {
            return STR_ALLOCATION_FAILED;
        }
        local = LoadStringAdjusted(target);
        var * mut @expose_address byte base_ptr = (local.cap & InlineTag()) != 0 ?(* mut @expose_address byte)(& mut local.inline_data.b00) : local.ptr;
        NativeAlloc.Copy(MakeMutPtr(AddMut(base_ptr, local.len), slice.len), MakeConstPtr(slice.ptr, slice.len), slice.len);
        local.len = local.len + slice.len;
        StoreString(target, local);
        return STR_SUCCESS;
    }
    @extern("C") @export("chic_rt_string_from_slice") public unsafe static ChicString chic_rt_string_from_slice(ChicStr slice) {
        var str = chic_rt_string_new();
        chic_rt_string_push_slice(& str, slice);
        return str;
    }
    @extern("C") @export("chic_rt_string_from_char") public unsafe static ChicString chic_rt_string_from_char(u32 value) {
        var str = chic_rt_string_new();
        var buf = ZeroInline32();
        var outPtr = & buf.b00;
        var written = EncodeCodePoint(value, outPtr);
        if (written >0)
        {
            var * const @readonly @expose_address byte raw = & buf.b00;
            var slice = new ChicStr {
                ptr = raw, len = written
            }
            ;
            chic_rt_string_push_slice(& str, slice);
        }
        return str;
    }
    @extern("C") @export("chic_rt_string_error_message") public unsafe static ChicStr chic_rt_string_error_message(int code) {
        var tmp = ZeroInline64();
        let len = WriteErrorMessage(code, & tmp.b00);
        if (len == 0)
        {
            return new ChicStr {
                ptr = NativePtr.NullConst(), len = 0
            }
            ;
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = len, Alignment = 1
        }
        ;
        if (NativeAlloc.Alloc (len, 1, out alloc) != NativeAllocationError.Success || NativePtr.IsNull(alloc.Pointer)) {
            return new ChicStr {
                ptr = NativePtr.NullConst(), len = 0
            }
            ;
        }
        NativeAlloc.Copy(alloc, MakeConstPtr(& tmp.b00, len), len);
        return new ChicStr {
            ptr = NativePtr.AsConstPtr(alloc.Pointer), len = len
        }
        ;
    }
    @extern("C") @export("chic_rt_string_clone_slice") public unsafe static int chic_rt_string_clone_slice(* mut ChicString dest,
    ChicStr slice) {
        if (dest == null)
        {
            return 4;
        }
        InitInline(dest);
        return chic_rt_string_push_slice(dest, slice);
    }
    @extern("C") @export("chic_rt_string_append_slice") public unsafe static int chic_rt_string_append_slice(* mut ChicString target,
    ChicStr slice, int alignment, int has_alignment) {
        return AppendAlignedBytes(target, slice.ptr, slice.len, alignment, has_alignment);
    }
    @extern("C") @export("chic_rt_string_append_bool") public unsafe static int chic_rt_string_append_bool(* mut ChicString target,
    bool value, int alignment, int has_alignment, ChicStr format) {
        var tmp = ZeroInline32();
        let written = FormatBool(value, 0, 0, format, & tmp.b00);
        var * const @readonly @expose_address byte raw = & tmp.b00;
        return AppendAlignedBytes(target, raw, written, alignment, has_alignment);
    }
    @extern("C") @export("chic_rt_string_append_char") public unsafe static int chic_rt_string_append_char(* mut ChicString target,
    u32 value, int alignment, int has_alignment, ChicStr format) {
        var tmp = ZeroInline32();
        var * mut @expose_address byte outPtr = & tmp.b00;
        var written = EncodeCodePoint(value, outPtr);
        if (written == 0)
        {
            return STR_UTF8;
        }
        var * const @readonly @expose_address byte raw = & tmp.b00;
        return AppendAlignedBytes(target, raw, written, alignment, has_alignment);
    }
    @extern("C") @export("chic_rt_string_append_signed") public unsafe static int chic_rt_string_append_signed(* mut ChicString target,
    i64 low, i64 high, u32 bits, int alignment, int has_alignment, ChicStr format) {
        var value = ((i128) high << 64) | (i128)(u64) low;
        var fmt = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let parse = ParseNumericFormat(format.ptr, format.len, & fmt);
        if (parse != STR_SUCCESS)
        {
            return parse;
        }
        let isHex = (fmt.flags & NUM_FMT_HEX) != 0u8;
        let upper = (fmt.flags & NUM_FMT_UPPER) != 0u8;
        let hasWidth = (fmt.flags & NUM_FMT_HAS_WIDTH) != 0u8;
        let width = fmt.width;
        var tmp = ZeroInline64();
        var written = 0usize;
        if (isHex)
        {
            let maskBits = EffectiveMaskBits(bits, hasWidth, width);
            let masked = MaskUnsigned(ToU128Unchecked(value), maskBits);
            let minWidth = hasWidth ?width : 0usize;
            let clampedWidth = minWidth >FLOAT_TMP_CAP ?FLOAT_TMP_CAP : minWidth;
            let lo_masked = (u64) masked;
            let hi_masked = (u64)(masked >> 64);
            written = FormatHexParts(hi_masked, lo_masked, upper, clampedWidth, & tmp.b00);
        }
        else
        {
            let minWidth = hasWidth ?width : 0usize;
            let clampedWidth = minWidth >FLOAT_TMP_CAP ?FLOAT_TMP_CAP : minWidth;
            written = FormatSigned(value, clampedWidth, & tmp.b00);
        }
        var * const @readonly @expose_address byte raw = & tmp.b00;
        return AppendAlignedBytes(target, raw, written, alignment, has_alignment);
    }
    @extern("C") @export("chic_rt_string_append_unsigned") public unsafe static int chic_rt_string_append_unsigned(* mut ChicString target,
    u64 low, u64 high, u32 bits, int alignment, int has_alignment, ChicStr format) {
        var fmt = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let parse = ParseNumericFormat(format.ptr, format.len, & fmt);
        if (parse != STR_SUCCESS)
        {
            return parse;
        }
        let isHex = (fmt.flags & NUM_FMT_HEX) != 0u8;
        let upper = (fmt.flags & NUM_FMT_UPPER) != 0u8;
        let hasWidth = (fmt.flags & NUM_FMT_HAS_WIDTH) != 0u8;
        let width = fmt.width;
        var tmp = ZeroInline64();
        let minWidth = hasWidth ?width : 0usize;
        let clampedWidth = minWidth >FLOAT_TMP_CAP ?FLOAT_TMP_CAP : minWidth;
        let lo_u = low;
        let hi_u = high;
        var lo_masked = lo_u;
        var hi_masked = hi_u;
        if (bits >0u32 && bits <128u32)
        {
            if (bits >= 64u32)
            {
                let hi_bits = bits - 64u32;
                hi_masked = hi_u & MaskLower64(hi_bits);
            }
            else
            {
                lo_masked = lo_u & MaskLower64(bits);
                hi_masked = 0u64;
            }
        }
        var written = 0usize;
        if (isHex)
        {
            written = FormatHexParts(hi_masked, lo_masked, upper, clampedWidth, & tmp.b00);
        }
        else
        {
            let combined = ((u128) hi_masked << 64) | (u128) lo_masked;
            written = FormatUnsigned(combined, clampedWidth, & tmp.b00);
        }
        if (isHex && clampedWidth >written)
        {
            let pad = clampedWidth - written;
            if (written >0)
            {
                NativeAlloc.Move(MakeMutPtr(AddMut(& tmp.b00, pad), written), MakeConstPtr(& tmp.b00, written), written);
            }
            var p = 0usize;
            while (p <pad)
            {
                StoreByte(AddMut(& tmp.b00, p), ASCII_ZERO);
                p += 1usize;
            }
            written = clampedWidth;
        }
        var * const @readonly @expose_address byte raw = & tmp.b00;
        return AppendAlignedBytes(target, raw, written, alignment, has_alignment);
    }
    @extern("C") @export("chic_rt_string_append_f32") public unsafe static int chic_rt_string_append_f32(* mut ChicString target,
    f32 value, int alignment, int has_alignment, ChicStr format) {
        var fmt = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let parse = ParseNumericFormat(format.ptr, format.len, & fmt);
        if (parse != STR_SUCCESS)
        {
            return parse;
        }
        let upper = (fmt.flags & NUM_FMT_UPPER) != 0u8;
        let hasWidth = (fmt.flags & NUM_FMT_HAS_WIDTH) != 0u8;
        let width = fmt.width;
        let hasPrecision = (fmt.flags & NUM_FMT_HAS_PRECISION) != 0u8;
        let precision = fmt.precision;
        let floatKind = fmt.floatKind;
        var tmp = ZeroInline64();
        var written = FormatFloatValue((f64) value, floatKind, hasPrecision, precision, upper, & tmp.b00);
        var * const @readonly @expose_address byte raw = & tmp.b00;
        var fmtAlignment = alignment;
        var fmtHasAlignment = has_alignment;
        if (hasWidth && has_alignment == 0)
        {
            fmtAlignment = (int) width;
            fmtHasAlignment = 1;
        }
        return AppendAlignedBytes(target, raw, written, fmtAlignment, fmtHasAlignment);
    }
    @extern("C") @export("chic_rt_string_append_f16") public unsafe static int chic_rt_string_append_f16(* mut ChicString target,
    u16 bits, int alignment, int has_alignment, ChicStr format) {
        var fmt = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let parse = ParseNumericFormat(format.ptr, format.len, & fmt);
        if (parse != STR_SUCCESS)
        {
            return parse;
        }
        let upper = (fmt.flags & NUM_FMT_UPPER) != 0u8;
        let hasWidth = (fmt.flags & NUM_FMT_HAS_WIDTH) != 0u8;
        let width = fmt.width;
        let hasPrecision = (fmt.flags & NUM_FMT_HAS_PRECISION) != 0u8;
        let precision = fmt.precision;
        let floatKind = fmt.floatKind;
        var tmp = ZeroInline64();
        var written = FormatHalf(bits, floatKind, hasPrecision, precision, upper, & tmp.b00);
        var * const @readonly @expose_address byte raw = & tmp.b00;
        var fmtAlignment = alignment;
        var fmtHasAlignment = has_alignment;
        if (hasWidth && has_alignment == 0)
        {
            fmtAlignment = (int) width;
            fmtHasAlignment = 1;
        }
        return AppendAlignedBytes(target, raw, written, fmtAlignment, fmtHasAlignment);
    }
    @extern("C") @export("chic_rt_string_append_f128") public unsafe static int chic_rt_string_append_f128(* mut ChicString target,
    u128 bits, int alignment, int has_alignment, ChicStr format) {
        var fmt = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let parse = ParseNumericFormat(format.ptr, format.len, & fmt);
        if (parse != STR_SUCCESS)
        {
            return parse;
        }
        let upper = (fmt.flags & NUM_FMT_UPPER) != 0u8;
        let hasWidth = (fmt.flags & NUM_FMT_HAS_WIDTH) != 0u8;
        let width = fmt.width;
        let hasPrecision = (fmt.flags & NUM_FMT_HAS_PRECISION) != 0u8;
        let precision = fmt.precision;
        let floatKind = fmt.floatKind;
        var tmp = ZeroInline64();
        var written = FormatF128(bits, floatKind, hasPrecision, precision, upper, & tmp.b00);
        var * const @readonly @expose_address byte raw = & tmp.b00;
        var fmtAlignment = alignment;
        var fmtHasAlignment = has_alignment;
        if (hasWidth && has_alignment == 0)
        {
            fmtAlignment = (int) width;
            fmtHasAlignment = 1;
        }
        return AppendAlignedBytes(target, raw, written, fmtAlignment, fmtHasAlignment);
    }
    @extern("C") @export("chic_rt_string_append_f64") public unsafe static int chic_rt_string_append_f64(* mut ChicString target,
    f64 value, int alignment, int has_alignment, ChicStr format) {
        var fmt = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let parse = ParseNumericFormat(format.ptr, format.len, & fmt);
        if (parse != STR_SUCCESS)
        {
            return parse;
        }
        let upper = (fmt.flags & NUM_FMT_UPPER) != 0u8;
        let hasWidth = (fmt.flags & NUM_FMT_HAS_WIDTH) != 0u8;
        let width = fmt.width;
        let hasPrecision = (fmt.flags & NUM_FMT_HAS_PRECISION) != 0u8;
        let precision = fmt.precision;
        let floatKind = fmt.floatKind;
        var tmp = ZeroInline64();
        var written = FormatFloatValue(value, floatKind, hasPrecision, precision, upper, & tmp.b00);
        var * const @readonly @expose_address byte raw = & tmp.b00;
        var fmtAlignment = alignment;
        var fmtHasAlignment = has_alignment;
        if (hasWidth && has_alignment == 0)
        {
            fmtAlignment = (int) width;
            fmtHasAlignment = 1;
        }
        return AppendAlignedBytes(target, raw, written, fmtAlignment, fmtHasAlignment);
    }
    @extern("C") @export("chic_rt_string_clone") public unsafe static int chic_rt_string_clone(* mut ChicString dest, * const @readonly ChicString src) {
        if (dest == null || src == null)
        {
            return STR_INVALID_POINTER;
        }
        InitInline(dest);
        let source = LoadStringAdjusted(src);
        let length = source.len;
        if (length == 0)
        {
            return STR_SUCCESS;
        }
        var src_ptr = (source.cap & InlineTag()) != 0 ?(* const @readonly @expose_address byte)(& source.inline_data.b00) : NativePtr.AsConstPtr(source.ptr);
        if (!EnsureCapacity (dest, length))
        {
            return STR_ALLOCATION_FAILED;
        }
        var local = LoadStringAdjusted(dest);
        var * mut @expose_address byte dst_ptr = (local.cap & InlineTag()) != 0 ?(* mut @expose_address byte)(& mut local.inline_data.b00) : local.ptr;
        NativeAlloc.Copy(MakeMutPtr(dst_ptr, length), MakeConstPtr(src_ptr, length), length);
        local.len = length;
        StoreString(dest, local);
        return STR_SUCCESS;
    }
    @extern("C") @export("chic_rt_string_truncate") public unsafe static int chic_rt_string_truncate(* mut ChicString target,
    usize newLen) {
        if (target == null)
        {
            return STR_INVALID_POINTER;
        }
        NormalizeInlinePtr(target);
        var local = LoadStringAdjusted(target);
        if (newLen >local.len)
        {
            return STR_OUT_OF_BOUNDS;
        }
        local.len = newLen;
        StoreString(target, local);
        return STR_SUCCESS;
    }
    @extern("C") @export("chic_rt_string_reserve") public unsafe static int chic_rt_string_reserve(* mut ChicString target,
    usize additional) {
        if (target == null)
        {
            return STR_INVALID_POINTER;
        }
        return EnsureCapacity(target, additional) ?STR_SUCCESS : STR_ALLOCATION_FAILED;
    }
    @extern("C") @export("chic_rt_string_allocations") public unsafe static usize chic_rt_string_allocations() {
        return 0;
    }
    @extern("C") @export("chic_rt_string_frees") public unsafe static usize chic_rt_string_frees() {
        return 0;
    }
    // Debug helper to ensure native string exports are emitted.
    @extern("C") @export("chic_rt_string_debug_ping") public static int chic_rt_string_debug_ping() {
        return 42;
    }
    public unsafe static void TestCoverageHelpers() {
        var spec = new NumericFormatSpec {
            flags = 0u8, floatKind = 0u8, width = 0usize, precision = 0usize
        }
        ;
        let _ = ParseNumericFormat(NativePtr.NullConst(), 0usize, & spec);
        var fmtUnknown = new StringInlineBytes32 {
            b00 = 113, b01 = 0,
        }
        ;
        let _ = ParseNumericFormat(NativePtr.AsConstPtr(& fmtUnknown.b00), 1usize, & spec);
        var fmtBad = new StringInlineBytes32 {
            b00 = 120, b01 = 90, b02 = 0,
        }
        ;
        let _ = ParseNumericFormat(NativePtr.AsConstPtr(& fmtBad.b00), 2usize, & spec);
        var fmtHex = new StringInlineBytes32 {
            b00 = 120, b01 = 52, b02 = 0,
        }
        ;
        let _ = ParseNumericFormat(NativePtr.AsConstPtr(& fmtHex.b00), 2usize, & spec);
        var fmtFloat = new StringInlineBytes32 {
            b00 = 102, b01 = 50, b02 = 0,
        }
        ;
        let _ = ParseNumericFormat(NativePtr.AsConstPtr(& fmtFloat.b00), 2usize, & spec);
        let _ = Pow10(0usize);
        let _ = Pow10(3usize);
        let _ = Pow2I32(0);
        let _ = Pow2I32(2);
        let _ = Pow2I32(2048);
        let _ = Pow2I32(- 2048);
        // Exercise masking helpers and pointer classifiers.
        let _ = MaskUnsigned(0u128, 0u32);
        let _ = MaskUnsigned(0u128, 1u32);
        let _ = MaskUnsigned(1u128, 1u32);
        let _ = MaskUnsigned(0xFFFFu128, 16u32);
        let _ = MaskUnsigned(0xFFFFu128, 128u32);
        let _ = EffectiveMaskBits(0u32, false, 0usize);
        let _ = EffectiveMaskBits(0u32, true, 0usize);
        let _ = EffectiveMaskBits(0u32, true, 2usize);
        let _ = EffectiveMaskBits(32u32, true, 0usize);
        let _ = IsInlinePtr((* const @readonly ChicString) NativePtr.NullConst());
        var tmpStr = chic_rt_string_new();
        let _ = IsInlinePtr(& tmpStr);
        let _ = HeapCapacityPtr(& tmpStr);
        let _ = DataPtrMut((* mut ChicString) NativePtr.NullMut());
        let _ = DataPtrMut(& tmpStr);
        let _ = DataPtrConst((* const @readonly ChicString) NativePtr.NullConst());
        let _ = DataPtrConst(& tmpStr);
        let emptyFmt = new ChicStr {
            ptr = NativePtr.NullConst(), len = 0usize
        }
        ;
        let _ = chic_rt_string_append_f32(& tmpStr, 1.25f, 0, 0, emptyFmt);
        chic_rt_string_drop(& tmpStr);
        var tmp = ZeroInline64();
        let _ = WriteWithAlignment(NativePtr.AsConstPtr(& tmp.b00), 0usize, 5, 1, & tmp.b00);
        let _ = WriteWithAlignment(NativePtr.AsConstPtr(& tmp.b00), 1usize, - 4, 1, & tmp.b00);
        let _ = FormatFloatFixed(12.34, 2usize, & tmp.b00);
        let _ = FormatFloatFixed(- 0.0, 3usize, & tmp.b00);
        let _ = FormatFloatExponent(1234.0, 2usize, true, & tmp.b00);
        let _ = FormatFloatValue(0.00005, 3u8, true, 4usize, false, & tmp.b00);
        let _ = FormatFloatValue(100000.0, 3u8, true, 2usize, true, & tmp.b00);
        let _ = FormatFloatValue(1.25, 1u8, false, 0usize, false, & tmp.b00);
        let _ = FormatFloatValue(1.25, 2u8, false, 0usize, false, & tmp.b00);
    }
    public unsafe static byte FirstByteViaByValue(string value) {
        let ptr = StringRuntime.chic_rt_string_get_ptr(& value);
        if (ptr == null)
        {
            return 0u8;
        }
        return * ptr;
    }
}
