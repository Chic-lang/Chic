namespace Std.Runtime.Native;
// Chic-native Span/ReadOnlySpan runtime. Provides the stable
// `chic_rt_span_*` ABI using Chic implementations instead of the
// bootstrap runtime.
@repr(c) public struct ChicSpan
{
    public ValueMutPtr data;
    public usize len;
    public usize elem_size;
    public usize elem_align;
}
@repr(c) public struct ChicReadOnlySpan
{
    public ValueConstPtr data;
    public usize len;
    public usize elem_size;
    public usize elem_align;
}
@repr(c) public struct SpanLayoutInfo
{
    public usize size;
    public usize offset_data;
    public usize offset_reserved;
    public usize offset_len;
    public usize offset_elem_size;
    public usize offset_elem_align;
}
public enum SpanError
{
    Success = 0, NullPointer = 1, OutOfBounds = 2, InvalidStride = 3,
}
private static int SpanStatus(SpanError status) {
    let value = (int) status;
    if (value <0 || value >3)
    {
        return 255;
    }
    return value;
}
@extern("C") private static extern void abort();
private const usize SPAN_OFFSET_LEN = sizeof(ValueMutPtr);
private const usize SPAN_OFFSET_ELEM_SIZE = SPAN_OFFSET_LEN + sizeof(usize);
private const usize SPAN_OFFSET_ELEM_ALIGN = SPAN_OFFSET_ELEM_SIZE + sizeof(usize);
private const usize SPAN_SIZE_BYTES = SPAN_OFFSET_ELEM_ALIGN + sizeof(usize);
private unsafe static usize SpanLen(* const ChicSpan span) {
    return(* span).len;
}
private unsafe static usize SpanElemSize(* const ChicSpan span) {
    return(* span).elem_size;
}
private unsafe static usize SpanElemAlign(* const ChicSpan span) {
    return(* span).elem_align;
}
private unsafe static usize ReadonlySpanLen(* const ChicReadOnlySpan span) {
    return(* span).len;
}
private unsafe static usize ReadonlySpanElemSize(* const ChicReadOnlySpan span) {
    return(* span).elem_size;
}
private unsafe static usize ReadonlySpanElemAlign(* const ChicReadOnlySpan span) {
    return(* span).elem_align;
}
private unsafe static bool IsNullSpanConst(* const ChicSpan ptr) {
    var * const @readonly @expose_address byte raw = ptr;
    return NativePtr.IsNullConst(raw);
}
private unsafe static bool IsNullSpanMut(* mut ChicSpan ptr) {
    var * mut @expose_address byte raw = ptr;
    return NativePtr.IsNull(raw);
}
private unsafe static bool IsNullReadonlySpanConst(* const ChicReadOnlySpan ptr) {
    var * const @readonly @expose_address byte raw = ptr;
    return NativePtr.IsNullConst(raw);
}
private static bool IsPowerOfTwo(usize value) {
    return value != 0 && (value & (value - 1)) == 0;
}
private static SpanError ValidateStride(usize elem_size, usize elem_align) {
    if (elem_size == 0)
    {
        return SpanError.Success;
    }
    if (elem_align == 0 || !IsPowerOfTwo (elem_align))
    {
        return SpanError.InvalidStride;
    }
    return SpanError.Success;
}
private unsafe static ValueMutPtr DanglingMut(usize elem_size, usize elem_align) {
    return new ValueMutPtr {
        Pointer = NativePtr.FromIsize(1), Size = elem_size, Alignment = elem_align,
    }
    ;
}
private unsafe static ValueConstPtr DanglingConst(usize elem_size, usize elem_align) {
    return new ValueConstPtr {
        Pointer = NativePtr.FromIsizeConst(1), Size = elem_size, Alignment = elem_align,
    }
    ;
}
private unsafe static ChicSpan FailureMut(ValueMutPtr data) {
    return new ChicSpan {
        data = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = data.Size, Alignment = data.Alignment,
        }
        , len = 0, elem_size = data.Size, elem_align = data.Alignment,
    }
    ;
}
private unsafe static ChicReadOnlySpan FailureConst(ValueConstPtr data) {
    return new ChicReadOnlySpan {
        data = new ValueConstPtr {
            Pointer = NativePtr.NullConst(), Size = data.Size, Alignment = data.Alignment,
        }
        , len = 0, elem_size = data.Size, elem_align = data.Alignment,
    }
    ;
}
private static bool TryMultiply(usize lhs, usize rhs, out usize product) {
    var value = lhs * rhs;
    if (lhs != 0 && value / lhs != rhs)
    {
        product = 0;
        return false;
    }
    product = value;
    return true;
}
private unsafe static SpanError MakeSpan(ValueMutPtr data, usize len, out ChicSpan span) {
    var result = FailureMut(data);
    let validation = ValidateStride(data.Size, data.Alignment);
    if (validation != SpanError.Success)
    {
        span = result;
        return validation;
    }
    if (len == 0)
    {
        result.data = DanglingMut(data.Size, data.Alignment);
        result.len = 0;
        result.elem_size = data.Size;
        result.elem_align = data.Alignment;
        span = result;
        return SpanError.Success;
    }
    if (data.Size == 0)
    {
        result.data = DanglingMut(data.Size, data.Alignment);
        result.len = len;
        result.elem_size = data.Size;
        result.elem_align = data.Alignment;
        span = result;
        return SpanError.Success;
    }
    if (NativePtr.IsNull (data.Pointer))
    {
        span = result;
        return SpanError.NullPointer;
    }
    if (data.Alignment >1)
    {
        let address = (usize) NativePtr.ToIsize(data.Pointer);
        if (address % data.Alignment != 0)
        {
            span = result;
            return SpanError.InvalidStride;
        }
    }
    result.data = data;
    result.len = len;
    result.elem_size = data.Size;
    result.elem_align = data.Alignment;
    span = result;
    return SpanError.Success;
}
private unsafe static SpanError MakeReadonlySpan(ValueConstPtr data, usize len, out ChicReadOnlySpan span) {
    var result = FailureConst(data);
    let validation = ValidateStride(data.Size, data.Alignment);
    if (validation != SpanError.Success)
    {
        span = result;
        return validation;
    }
    if (len == 0)
    {
        result.data = DanglingConst(data.Size, data.Alignment);
        result.len = 0;
        result.elem_size = data.Size;
        result.elem_align = data.Alignment;
        span = result;
        return SpanError.Success;
    }
    if (data.Size == 0)
    {
        result.data = DanglingConst(data.Size, data.Alignment);
        result.len = len;
        result.elem_size = data.Size;
        result.elem_align = data.Alignment;
        span = result;
        return SpanError.Success;
    }
    if (NativePtr.IsNullConst (data.Pointer))
    {
        span = result;
        return SpanError.NullPointer;
    }
    if (data.Alignment >1)
    {
        let address = (usize) NativePtr.ToIsizeConst(data.Pointer);
        if (address % data.Alignment != 0)
        {
            span = result;
            return SpanError.InvalidStride;
        }
    }
    result.data = data;
    result.len = len;
    result.elem_size = data.Size;
    result.elem_align = data.Alignment;
    span = result;
    return SpanError.Success;
}
public static class SpanRuntime
{
    @extern("C") @export("chic_rt_span_layout_debug") public unsafe static void chic_rt_span_layout_debug(* mut @expose_address SpanLayoutInfo dest) {
        var * mut @expose_address byte dest_raw = dest;
        if (NativePtr.IsNull (dest_raw))
        {
            return;
        }
        let len_offset = sizeof(ValueMutPtr);
        let elem_size_offset = len_offset + sizeof(usize);
        let elem_align_offset = elem_size_offset + sizeof(usize);
        * dest = new SpanLayoutInfo {
            size = sizeof(ChicSpan), offset_data = 0, offset_reserved = len_offset, offset_len = len_offset, offset_elem_size = elem_size_offset, offset_elem_align = elem_align_offset,
        }
        ;
    }
    @extern("C") @export("chic_rt_span_from_raw_mut") public unsafe static ChicSpan chic_rt_span_from_raw_mut(* const @expose_address ValueMutPtr data,
    usize len) {
        var * const @readonly @expose_address byte data_raw = data;
        if (NativePtr.IsNullConst (data_raw))
        {
            return new ChicSpan {
                data = DanglingMut(0, 0), len = len, elem_size = 0, elem_align = 1,
            }
            ;
        }
        let handle = * data;
        var span = new ChicSpan {
            data = handle, len = len, elem_size = handle.Size, elem_align = handle.Alignment,
        }
        ;
        if (NativePtr.IsNull (handle.Pointer))
        {
            return span;
        }
        return span;
    }
    @extern("C") @export("chic_rt_span_from_raw_const") public unsafe static ChicReadOnlySpan chic_rt_span_from_raw_const(* const @expose_address ValueConstPtr data,
    usize len) {
        var * const @readonly @expose_address byte data_raw = data;
        if (NativePtr.IsNullConst (data_raw))
        {
            return new ChicReadOnlySpan {
                data = DanglingConst(0, 0), len = len, elem_size = 0, elem_align = 1,
            }
            ;
        }
        let handle = * data;
        var span = new ChicReadOnlySpan {
            data = handle, len = len, elem_size = handle.Size, elem_align = handle.Alignment,
        }
        ;
        if (NativePtr.IsNullConst (handle.Pointer))
        {
            return span;
        }
        return span;
    }
    @extern("C") @export("chic_rt_span_slice_mut") public unsafe static int chic_rt_span_slice_mut(* const @expose_address ChicSpan source,
    usize start, usize length, * mut @expose_address ChicSpan dest) {
        var * const @readonly @expose_address byte src_raw = source;
        var * mut @expose_address byte dest_raw = dest;
        if (NativePtr.IsNullConst (src_raw) || NativePtr.IsNull (dest_raw))
        {
            return SpanStatus(SpanError.NullPointer);
        }
        let span_len = SpanLen(source);
        if (start >span_len || length >span_len - start)
        {
            return SpanStatus(SpanError.OutOfBounds);
        }
        let elem_size = SpanElemSize(source);
        let elem_align = SpanElemAlign(source);
        var slice_data = new ValueMutPtr {
            Pointer = (* source).data.Pointer, Size = elem_size, Alignment = elem_align,
        }
        ;
        if (elem_size != 0)
        {
            var offset = start * elem_size;
            slice_data.Pointer = NativePtr.OffsetMut((* source).data.Pointer, (isize) offset);
        }
        var sliced = FailureMut(slice_data);
        let status = MakeSpan(slice_data, length, out sliced);
        if (status != SpanError.Success)
        {
            return SpanStatus(status);
        }
        * dest = sliced;
        return SpanStatus(SpanError.Success);
    }
    @extern("C") @export("chic_rt_span_slice_readonly") public unsafe static int chic_rt_span_slice_readonly(* const @expose_address ChicReadOnlySpan source,
    usize start, usize length, * mut @expose_address ChicReadOnlySpan dest) {
        var * const @readonly @expose_address byte src_raw = source;
        var * mut @expose_address byte dest_raw = dest;
        if (NativePtr.IsNullConst (src_raw) || NativePtr.IsNull (dest_raw))
        {
            return SpanStatus(SpanError.NullPointer);
        }
        let span_len = ReadonlySpanLen(source);
        if (start >span_len || length >span_len - start)
        {
            return SpanStatus(SpanError.OutOfBounds);
        }
        let elem_size = ReadonlySpanElemSize(source);
        let elem_align = ReadonlySpanElemAlign(source);
        var slice_data = new ValueConstPtr {
            Pointer = (* source).data.Pointer, Size = elem_size, Alignment = elem_align,
        }
        ;
        if (elem_size != 0)
        {
            var offset = start * elem_size;
            slice_data.Pointer = NativePtr.OffsetConst((* source).data.Pointer, (isize) offset);
        }
        var sliced = FailureConst(slice_data);
        let status = MakeReadonlySpan(slice_data, length, out sliced);
        if (status != SpanError.Success)
        {
            return SpanStatus(status);
        }
        * dest = sliced;
        return SpanStatus(SpanError.Success);
    }
    @extern("C") @export("chic_rt_span_to_readonly") public unsafe static ChicReadOnlySpan chic_rt_span_to_readonly(* const @expose_address ChicSpan span) {
        if (IsNullSpanConst (span))
        {
            return FailureConst(DanglingConst(0, 0));
        }
        var spanPtr = span;
        return new ChicReadOnlySpan {
            data = new ValueConstPtr {
                Pointer = NativePtr.AsConstPtr((* spanPtr).data.Pointer), Size = (* spanPtr).data.Size, Alignment = (* spanPtr).data.Alignment,
            }
            , len = SpanLen(spanPtr), elem_size = SpanElemSize(spanPtr), elem_align = SpanElemAlign(spanPtr),
        }
        ;
    }
    @extern("C") @export("chic_rt_span_copy_to") public unsafe static int chic_rt_span_copy_to(* const @expose_address ChicReadOnlySpan source,
    * const @expose_address ChicSpan dest) {
        if (IsNullReadonlySpanConst (source) || IsNullSpanConst (dest))
        {
            return SpanStatus(SpanError.NullPointer);
        }
        var sourcePtr = source;
        var destPtr = dest;
        let src_len = ReadonlySpanLen(sourcePtr);
        let dst_len = SpanLen(destPtr);
        let src_elem_size = ReadonlySpanElemSize(sourcePtr);
        let src_elem_align = ReadonlySpanElemAlign(sourcePtr);
        let dst_elem_size = SpanElemSize(destPtr);
        let dst_elem_align = SpanElemAlign(destPtr);
        if (src_len >dst_len)
        {
            return SpanStatus(SpanError.OutOfBounds);
        }
        if (src_elem_size != dst_elem_size)
        {
            return SpanStatus(SpanError.InvalidStride);
        }
        if (src_elem_size == 0 || src_len == 0)
        {
            return SpanStatus(SpanError.Success);
        }
        if (NativePtr.IsNullConst ( (* sourcePtr).data.Pointer) || NativePtr.IsNull ( (* destPtr).data.Pointer))
        {
            return SpanStatus(SpanError.NullPointer);
        }
        let byte_len = src_len * src_elem_size;
        if (byte_len / src_elem_size != src_len)
        {
            return SpanStatus(SpanError.InvalidStride);
        }
        var dst_handle = new ValueMutPtr {
            Pointer = (* destPtr).data.Pointer, Size = dst_elem_size, Alignment = dst_elem_align,
        }
        ;
        var src_handle = new ValueConstPtr {
            Pointer = (* sourcePtr).data.Pointer, Size = src_elem_size, Alignment = src_elem_align,
        }
        ;
        NativeAlloc.Copy(dst_handle, src_handle, byte_len);
        return SpanStatus(SpanError.Success);
    }
    @extern("C") @export("chic_rt_span_fill") public unsafe static int chic_rt_span_fill(* const @expose_address ChicSpan dest,
    * const @readonly @expose_address byte value) {
        if (IsNullSpanConst (dest))
        {
            return SpanStatus(SpanError.NullPointer);
        }
        let dest_len = SpanLen(dest);
        let dest_elem_size = SpanElemSize(dest);
        let dest_elem_align = SpanElemAlign(dest);
        if (dest_len == 0)
        {
            return SpanStatus(SpanError.Success);
        }
        if (NativePtr.IsNull ( (* dest).data.Pointer) || NativePtr.IsNullConst (value))
        {
            return SpanStatus(SpanError.NullPointer);
        }
        if (dest_elem_size == 0)
        {
            return SpanStatus(SpanError.Success);
        }
        var byte_len = 0usize;
        if (!TryMultiply (dest_len, dest_elem_size, out byte_len)) {
            return SpanStatus(SpanError.InvalidStride);
        }
        var src_handle = new ValueConstPtr {
            Pointer = NativePtr.AsByteConst(value), Size = dest_elem_size, Alignment = dest_elem_align,
        }
        ;
        for (var offset = 0usize; offset <byte_len; offset += dest_elem_size) {
            var target = NativePtr.OffsetMut((* dest).data.Pointer, (isize) offset);
            var dst_handle = new ValueMutPtr {
                Pointer = target, Size = dest_elem_size, Alignment = dest_elem_align,
            }
            ;
            NativeAlloc.Copy(dst_handle, src_handle, dest_elem_size);
        }
        return SpanStatus(SpanError.Success);
    }
    @extern("C") @export("chic_rt_span_ptr_at_mut") public unsafe static * mut byte chic_rt_span_ptr_at_mut(* const @expose_address ChicSpan span,
    usize index) {
        if (IsNullSpanConst (span))
        {
            return NativePtr.NullMut();
        }
        let span_len = SpanLen(span);
        let elem_size = SpanElemSize(span);
        let elem_align = SpanElemAlign(span);
        if (index >= span_len)
        {
            return NativePtr.NullMut();
        }
        if (elem_size == 0)
        {
            return DanglingMut(elem_size, elem_align).Pointer;
        }
        if (NativePtr.IsNull ( (* span).data.Pointer))
        {
            return NativePtr.NullMut();
        }
        var offset = 0usize;
        if (!TryMultiply (index, elem_size, out offset)) {
            return NativePtr.NullMut();
        }
        return NativePtr.OffsetMut((* span).data.Pointer, (isize) offset);
    }
    @extern("C") @export("chic_rt_span_ptr_at_readonly") public unsafe static * const byte chic_rt_span_ptr_at_readonly(* const @expose_address ChicReadOnlySpan span,
    usize index) {
        if (IsNullReadonlySpanConst (span))
        {
            return NativePtr.NullConst();
        }
        let span_len = ReadonlySpanLen(span);
        let elem_size = ReadonlySpanElemSize(span);
        let elem_align = ReadonlySpanElemAlign(span);
        if (index >= span_len)
        {
            return NativePtr.NullConst();
        }
        if (elem_size == 0)
        {
            return DanglingConst(elem_size, elem_align).Pointer;
        }
        if (NativePtr.IsNullConst ( (* span).data.Pointer))
        {
            return NativePtr.NullConst();
        }
        var offset = 0usize;
        if (!TryMultiply (index, elem_size, out offset)) {
            return NativePtr.NullConst();
        }
        return NativePtr.OffsetConst((* span).data.Pointer, (isize) offset);
    }
}
