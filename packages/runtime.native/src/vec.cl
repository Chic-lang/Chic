namespace Std.Runtime.Native;
// Chic-native vector runtime with inline storage and heap growth.
@repr(c) public struct InlinePadding7
{
    public byte b0;
    public byte b1;
    public byte b2;
    public byte b3;
    public byte b4;
    public byte b5;
    public byte b6;
}
@repr(c) public struct ChicVec
{
    public * mut @expose_address byte ptr;
    public usize len;
    public usize cap;
    public usize elem_size;
    public usize elem_align;
    public fn @extern("C")(* mut @expose_address byte) -> void drop_fn;
    public * mut @expose_address byte region_ptr;
    public byte uses_inline;
    public InlinePadding7 inline_pad;
    public InlineBytes64 inline_storage;
}
@repr(c) public struct ChicVecView
{
    public * const @readonly @expose_address byte data;
    public usize len;
    public usize elem_size;
    public usize elem_align;
}
@repr(c) public struct ChicVecIter
{
    public * const @readonly @expose_address byte data;
    public usize index;
    public usize len;
    public usize elem_size;
    public usize elem_align;
}
@repr(c) public struct VecLayoutInfo
{
    public usize size;
    public usize offset_ptr;
    public usize offset_len;
    public usize offset_cap;
    public usize offset_elem_size;
    public usize offset_elem_align;
    public usize offset_drop_fn;
    public usize offset_region_ptr;
    public usize offset_uses_inline;
    public usize offset_inline_storage;
}
public enum VecError
{
    Success = 0, AllocationFailed = 1, InvalidPointer = 2, CapacityOverflow = 3, OutOfBounds = 4, LengthOverflow = 5, IterationComplete = 6,
}
public static class VecRuntime
{
    private const usize INLINE_BYTES = 64usize;
    private const byte INLINE_TRUE = 1;
    private const byte INLINE_FALSE = 0;
    private unsafe static extern void chic_rt_drop_invoke(fn @extern("C")(* mut @expose_address byte) -> void dropFn,
    * mut @expose_address byte value);
    @extern("C") private unsafe static extern fn @extern("C")(* mut @expose_address byte) -> void chic_rt_drop_noop_ptr();
    private static InlineBytes64 ZeroInline() {
        return new InlineBytes64 {
            b00 = 0, b01 = 0, b02 = 0, b03 = 0, b04 = 0, b05 = 0, b06 = 0, b07 = 0, b08 = 0, b09 = 0, b10 = 0, b11 = 0, b12 = 0, b13 = 0, b14 = 0, b15 = 0, b16 = 0, b17 = 0, b18 = 0, b19 = 0, b20 = 0, b21 = 0, b22 = 0, b23 = 0, b24 = 0, b25 = 0, b26 = 0, b27 = 0, b28 = 0, b29 = 0, b30 = 0, b31 = 0, b32 = 0, b33 = 0, b34 = 0, b35 = 0, b36 = 0, b37 = 0, b38 = 0, b39 = 0, b40 = 0, b41 = 0, b42 = 0, b43 = 0, b44 = 0, b45 = 0, b46 = 0, b47 = 0, b48 = 0, b49 = 0, b50 = 0, b51 = 0, b52 = 0, b53 = 0, b54 = 0, b55 = 0, b56 = 0, b57 = 0, b58 = 0, b59 = 0, b60 = 0, b61 = 0, b62 = 0, b63 = 0,
        }
        ;
    }
    private static InlinePadding7 ZeroPad7() {
        return new InlinePadding7 {
            b0 = 0, b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0, b6 = 0,
        }
        ;
    }
    @allow(all) private unsafe static isize AsIsize(usize value) {
        unchecked {
            return(isize) value;
        }
    }
    private unsafe static RegionHandle NullRegion() {
        return new RegionHandle {
            Pointer = 0ul,
            Profile = 0ul,
            Generation = 0ul
        }
        ;
    }
    private unsafe static * mut @expose_address byte InlinePtr(* mut ChicVec ptr) {
        if (IsNullVec (ptr))
        {
            return NativePtr.NullMut();
        }
        return & mut(* ptr).inline_storage.b00;
    }
    private unsafe static * const @readonly @expose_address byte InlinePtrConst(* const ChicVec ptr) {
        if (IsNullVecConst (ptr))
        {
            return NativePtr.NullConst();
        }
        return & (* ptr).inline_storage.b00;
    }
    private unsafe static usize InlineCapacity(ChicVec vec) {
        if (vec.elem_size == 0 || vec.elem_size >INLINE_BYTES)
        {
            return 0;
        }
        return INLINE_BYTES / vec.elem_size;
    }
    private unsafe static ValueMutPtr MakeMut(* mut @expose_address byte ptr, usize size, usize align) {
        return new ValueMutPtr {
            Pointer = ptr, Size = size, Alignment = align
        }
        ;
    }
    private unsafe static ValueConstPtr MakeConst(* const @readonly @expose_address byte ptr, usize size, usize align) {
        return new ValueConstPtr {
            Pointer = ptr, Size = size, Alignment = align
        }
        ;
    }
    private unsafe static ValueMutPtr MakeVecMut(* mut ChicVec ptr) {
        var * mut @expose_address byte raw = ptr;
        return new ValueMutPtr {
            Pointer = NativePtr.AsByteMut(raw), Size = sizeof(ChicVec), Alignment = 1,
        }
        ;
    }
    private unsafe static * mut @expose_address byte AsBytePtr(* mut ChicVec ptr) {
        var * mut @expose_address byte raw = ptr;
        return NativePtr.AsByteMut(raw);
    }
    private unsafe static ValueConstPtr MakeVecConst(* const ChicVec ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return new ValueConstPtr {
            Pointer = NativePtr.AsByteConst(raw), Size = sizeof(ChicVec), Alignment = 1,
        }
        ;
    }
    private unsafe static * const @readonly @expose_address byte AsBytePtrConst(* const ChicVec ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.AsByteConst(raw);
    }
    private unsafe static bool IsNullVec(* mut ChicVec vec) {
        return NativePtr.ToIsize(AsBytePtr(vec)) == 0;
    }
    private unsafe static bool IsNullVecConst(* const ChicVec vec) {
        return NativePtr.ToIsizeConst(AsBytePtrConst(vec)) == 0;
    }
    private unsafe static bool IsNullConstPtr(* const ValueConstPtr ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0;
    }
    private unsafe static bool IsNullMutPtr(* const ValueMutPtr ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0;
    }
    private unsafe static bool IsNullIter(ChicVecIter * iter) {
        var * const @readonly @expose_address byte raw = iter;
        return NativePtr.ToIsizeConst(raw) == 0;
    }
    private unsafe static bool IsNullView(* mut ChicVecView ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0;
    }
    private unsafe static ChicVec LoadVec(* const ChicVec ptr) {
        var tmp = new ChicVec {
            ptr = NativePtr.NullMut(), len = 0, cap = 0, elem_size = 0, elem_align = 0, drop_fn = chic_rt_drop_noop_ptr(), region_ptr = NativePtr.NullMut(), uses_inline = INLINE_FALSE, inline_pad = ZeroPad7(), inline_storage = ZeroInline(),
        }
        ;
        if (! IsNullVecConst (ptr))
        {
            NativeAlloc.Copy(MakeVecMut(& tmp), MakeVecConst(ptr), sizeof(ChicVec));
            if (tmp.uses_inline != 0)
            {
                tmp.ptr = NativePtr.AsMutPtr(InlinePtrConst(ptr));
            }
        }
        return tmp;
    }
    private unsafe static void StoreVec(* mut ChicVec dest, ChicVec value, bool copyInline) {
        if (IsNullVec (dest))
        {
            return;
        }
        // Keep inline data owned by the destination unless an explicit transfer is requested.
        var destPtr = dest;
        var valueCopy = value;
        var adjusted = valueCopy;
        if (valueCopy.uses_inline != 0)
        {
            adjusted.ptr = InlinePtr(destPtr);
            if (! copyInline)
            {
                // Preserve existing inline bytes from the destination.
                adjusted.inline_storage = (* destPtr).inline_storage;
                adjusted.inline_pad = (* destPtr).inline_pad;
            }
            else if (! NativePtr.IsNull (valueCopy.ptr))
            {
                // Preserve inline mutations by copying from the source inline buffer.
                NativeAlloc.Copy(MakeMut(& adjusted.inline_storage.b00, INLINE_BYTES, 1), MakeConst(valueCopy.ptr, INLINE_BYTES,
                1), INLINE_BYTES);
            }
        }
        NativeAlloc.Copy(MakeVecMut(destPtr), MakeVecConst(& adjusted), sizeof(ChicVec));
    }
    private unsafe static ValueMutPtr MakeIterMut(ChicVecIter * ptr) {
        var * mut @expose_address byte raw = ptr;
        return new ValueMutPtr {
            Pointer = NativePtr.AsByteMut(raw), Size = sizeof(ChicVecIter), Alignment = 1,
        }
        ;
    }
    private unsafe static ValueConstPtr MakeIterConst(* const ChicVecIter ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return new ValueConstPtr {
            Pointer = NativePtr.AsByteConst(raw), Size = sizeof(ChicVecIter), Alignment = 1,
        }
        ;
    }
    private unsafe static ChicVecIter LoadIter(ChicVecIter * ptr) {
        var tmp = new ChicVecIter {
            data = NativePtr.NullConst(), index = 0, len = 0, elem_size = 0, elem_align = 0,
        }
        ;
        if (IsNullIter (ptr))
        {
            return tmp;
        }
        tmp.data = (* ptr).data;
        tmp.index = (* ptr).index;
        tmp.len = (* ptr).len;
        tmp.elem_size = (* ptr).elem_size;
        tmp.elem_align = (* ptr).elem_align;
        return tmp;
    }
    private unsafe static void StoreIter(* mut ChicVecIter ptr, ChicVecIter value) {
        var target = ptr;
        if (IsNullIter (target))
        {
            return;
        }
        (* target).data = value.data;
        (* target).index = value.index;
        (* target).len = value.len;
        (* target).elem_size = value.elem_size;
        (* target).elem_align = value.elem_align;
    }
    private unsafe static void NormalizeInlinePtr(* mut ChicVec vec) {
        if (IsNullVec (vec))
        {
            return;
        }
        if ( (* vec).uses_inline != 0)
        {
            (* vec).ptr = InlinePtr(vec);
        }
    }
    private unsafe static void ActivateInline(* mut ChicVec vec) {
        var target = vec;
        if (IsNullVec (target))
        {
            return;
        }
        (* target).inline_storage = ZeroInline();
        (* target).inline_pad = ZeroPad7();
        (* target).uses_inline = INLINE_TRUE;
        (* target).ptr = InlinePtr(target);
        (* target).cap = InlineCapacity(* target);
        (* target).len = 0;
    }
    private unsafe static void ResetVec(ref ChicVec value) {
        value.ptr = NativePtr.NullMut();
        value.len = 0;
        value.cap = 0;
        value.uses_inline = INLINE_FALSE;
        value.inline_pad = ZeroPad7();
        value.inline_storage = ZeroInline();
    }
    private unsafe static bool AllocateExact(ref ChicVec vec, usize capacity) {
        if (capacity == 0)
        {
            vec.cap = 0;
            vec.ptr = NativePtr.NullMut();
            vec.uses_inline = INLINE_FALSE;
            return true;
        }
        if (vec.elem_size == 0)
        {
            vec.cap = capacity;
            vec.ptr = NativePtr.NullMut();
            vec.uses_inline = INLINE_FALSE;
            return true;
        }
        let align = vec.elem_align == 0 ?1 : vec.elem_align;
        let bytes = capacity * vec.elem_size;
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = bytes, Alignment = align
        }
        ;
        if (NativeAlloc.Alloc (bytes, align, out alloc) != NativeAllocationError.Success) {
            return false;
        }
        vec.ptr = alloc.Pointer;
        vec.cap = capacity;
        vec.uses_inline = INLINE_FALSE;
        return true;
    }
    private unsafe static VecError EnsureCapacity(* mut ChicVec vec, usize additional) {
        let vec_addr = NativePtr.ToIsize(AsBytePtr(vec));
        if (vec_addr == 0)
        {
            return VecError.InvalidPointer;
        }
        var target = vec;
        NormalizeInlinePtr(target);
        if ( (* target).uses_inline == INLINE_FALSE && (* target).cap == 0 && (* target).elem_size != 0 && InlineCapacity (* target) >0)
        {
            ActivateInline(target);
            NormalizeInlinePtr(target);
        }
        let ptrAddr = NativePtr.ToIsize((* target).ptr);
        let inlineAddr = NativePtr.ToIsize(InlinePtr(target));
        let inlineEnd = inlineAddr + AsIsize(INLINE_BYTES);
        let baseAddr = NativePtr.ToIsize(AsBytePtr(target));
        let vecEnd = baseAddr + AsIsize(sizeof(ChicVec));
        if ( (* target).uses_inline == INLINE_FALSE && ptrAddr >= baseAddr && ptrAddr <vecEnd)
        {
            // Any pointer inside the struct should be treated as inline storage.
            (* target).uses_inline = INLINE_TRUE;
            (* target).ptr = InlinePtr(target);
            (* target).cap = InlineCapacity(* target);
        }
        else if ( (* target).uses_inline == INLINE_FALSE && ptrAddr >= inlineAddr && ptrAddr <inlineEnd)
        {
            (* target).uses_inline = INLINE_TRUE;
            (* target).ptr = InlinePtr(target);
            (* target).cap = InlineCapacity(* target);
        }
        if (additional == 0)
        {
            return VecError.Success;
        }
        let needed = (* target).len + additional;
        if (needed < (* target).len)
        {
            return VecError.CapacityOverflow;
        }
        let align = (* target).elem_align == 0 ?1 : (* target).elem_align;
        // Honor callers that set the pointer to the inline buffer even if the bookkeeping
        // flag is wrong.
        let currentAddr = NativePtr.ToIsize((* target).ptr);
        if ( (* target).uses_inline == INLINE_FALSE && inlineAddr == currentAddr)
        {
            (* target).uses_inline = INLINE_TRUE;
            (* target).cap = InlineCapacity(* target);
        }
        if ( (* target).elem_size == 0)
        {
            if (needed > (* target).cap)
            {
                (* target).cap = needed;
            }
            return VecError.Success;
        }
        if ( (* target).uses_inline != 0)
        {
            let inlineCap = InlineCapacity(* target);
            if (needed <= inlineCap)
            {
                return VecError.Success;
            }
            var newCap = inlineCap * 2;
            if (newCap <needed)
            {
                newCap = needed;
            }
            let newSize = newCap * (* target).elem_size;
            if (newCap != 0 && newSize / (* target).elem_size != newCap)
            {
                return VecError.CapacityOverflow;
            }
            var alloc = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = newSize, Alignment = align
            }
            ;
            if (NativeAlloc.Alloc (newSize, align, out alloc) != NativeAllocationError.Success) {
                return VecError.AllocationFailed;
            }
            NativeAlloc.Copy(alloc, MakeConst((* target).ptr, (* target).len * (* target).elem_size, align), (* target).len * (* target).elem_size);
            (* target).ptr = alloc.Pointer;
            (* target).cap = newCap;
            (* target).uses_inline = INLINE_FALSE;
            return VecError.Success;
        }
        if (needed <= (* target).cap)
        {
            return VecError.Success;
        }
        var grow = (* target).cap == 0 ?needed : (* target).cap * 2;
        if (grow <needed)
        {
            grow = needed;
        }
        let newBytes = grow * (* target).elem_size;
        if (grow != 0 && newBytes / (* target).elem_size != grow)
        {
            return VecError.CapacityOverflow;
        }
        var updated = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newBytes, Alignment = align
        }
        ;
        if (NativeAlloc.Realloc (MakeMut ( (* target).ptr, (* target).cap * (* target).elem_size, align), (* target).cap * (* target).elem_size,
        newBytes, align, out updated) != NativeAllocationError.Success) {
            return VecError.AllocationFailed;
        }
        (* target).ptr = updated.Pointer;
        (* target).cap = grow;
        return VecError.Success;
    }
    private unsafe static void InvokeDrop(fn @extern("C")(* mut @expose_address byte) -> void dropFn, * mut @expose_address byte ptr) {
        let noop = chic_rt_drop_noop_ptr();
        if (dropFn == null || dropFn == noop)
        {
            return;
        }
        var scratch = (byte) 0;
        var * mut @expose_address byte scratchPtr = & scratch;
        var target = ptr;
        if (NativePtr.IsNull (target))
        {
            target = scratchPtr;
        }
        chic_rt_drop_invoke(dropFn, target);
    }
    private unsafe static ChicVec DropAll(ChicVec vec) {
        var local = vec;
        let dropFn = local.drop_fn;
        let elemSize = local.elem_size;
        let elemAlign = local.elem_align == 0 ?1 : local.elem_align;
        let dataPtr = local.ptr;
        let cap = local.cap;
        if (dropFn != null)
        {
            var index = 0usize;
            while (index <local.len)
            {
                var ptr = NativePtr.OffsetMut(dataPtr, AsIsize(index * elemSize));
                InvokeDrop(dropFn, ptr);
                index += 1;
            }
        }
        if (local.uses_inline == 0 && ! NativePtr.IsNull (dataPtr) && cap >0)
        {
            var handle = MakeMut(dataPtr, cap * elemSize, elemAlign);
            NativeAlloc.Free(handle);
        }
        ResetVec(ref local);
        return local;
    }
    private unsafe static ChicVec MakeVec(usize elemSize, usize elemAlign, fn @extern("C")(* mut @expose_address byte) -> void dropFn,
    RegionHandle region_handle) {
        var vec = new ChicVec {
            ptr = NativePtr.NullMut(), len = 0, cap = 0, elem_size = elemSize, elem_align = elemAlign == 0 ?1 : elemAlign, drop_fn = dropFn, region_ptr = NativePtr.FromIsize((isize) region_handle.Pointer), uses_inline = INLINE_FALSE, inline_pad = ZeroPad7(), inline_storage = ZeroInline(),
        }
        ;
        return vec;
    }
    @extern("C") @export("chic_rt_vec_new") public unsafe static ChicVec chic_rt_vec_new(usize elem_size,
    usize elem_align, fn @extern("C")(* mut @expose_address byte) -> void drop_fn) {
        return MakeVec(elem_size, elem_align, drop_fn, NullRegion());
    }
    @extern("C") @export("chic_rt_vec_new_in_region") public unsafe static ChicVec chic_rt_vec_new_in_region(usize elem_size,
    usize elem_align, fn @extern("C")(* mut @expose_address byte) -> void drop_fn, RegionHandle region_handle) {
        return MakeVec(elem_size, elem_align, drop_fn, region_handle);
    }
    @extern("C") @export("chic_rt_vec_with_capacity") public unsafe static ChicVec chic_rt_vec_with_capacity(usize elem_size,
    usize elem_align, usize capacity, fn @extern("C")(* mut @expose_address byte) -> void drop_fn) {
        var vec = MakeVec(elem_size, elem_align, drop_fn, NullRegion());
        if (capacity >0)
        {
            let _ = AllocateExact(ref vec, capacity);
        }
        return vec;
    }
    @extern("C") @export("chic_rt_vec_with_capacity_in_region") public unsafe static ChicVec chic_rt_vec_with_capacity_in_region(usize elem_size,
    usize elem_align, usize capacity, fn @extern("C")(* mut @expose_address byte) -> void drop_fn, RegionHandle region_handle) {
        var vec = MakeVec(elem_size, elem_align, drop_fn, region_handle);
        if (capacity >0)
        {
            let _ = AllocateExact(ref vec, capacity);
        }
        return vec;
    }
    @extern("C") @export("chic_rt_vec_drop") public unsafe static void chic_rt_vec_drop(* mut ChicVec vec) {
        if (IsNullVec (vec))
        {
            return;
        }
        var local = LoadVec(vec);
        local = DropAll(local);
        StoreVec(vec, local, true);
    }
    @extern("C") @export("chic_rt_vec_clone") public unsafe static int chic_rt_vec_clone(* mut ChicVec dest,
    * const ChicVec src) {
        if (IsNullVec (dest) || IsNullVecConst (src))
        {
            return 2;
        }
        var source = LoadVec(src);
        var target = MakeVec(source.elem_size, source.elem_align, source.drop_fn, new RegionHandle {
            Pointer = (ulong) (nuint) source.region_ptr,
            Profile = 0ul,
            Generation = 0ul
        }
        );
        if (source.len == 0)
        {
            StoreVec(dest, target, true);
            return 0;
        }
        if (! AllocateExact (ref target, source.len)) {
            StoreVec(dest, target, true);
            return 1;
        }
        let align = source.elem_align == 0 ?1 : source.elem_align;
        if (source.elem_size >0 && ! NativePtr.IsNull (source.ptr))
        {
            NativeAlloc.Copy(MakeMut(target.ptr, source.len * source.elem_size, align), MakeConst(source.ptr, source.len * source.elem_size,
            align), source.len * source.elem_size);
        }
        target.len = source.len;
        StoreVec(dest, target, true);
        return 0;
    }
    @extern("C") @export("chic_rt_vec_into_array") public unsafe static int chic_rt_vec_into_array(* mut ChicVec dest,
    * mut ChicVec src) {
        if (IsNullVec (dest) || IsNullVec (src))
        {
            return 2;
        }
        var source = LoadVec(src);
        var result = MakeVec(source.elem_size, source.elem_align, source.drop_fn, new RegionHandle {
            Pointer = (ulong) (nuint) source.region_ptr,
            Profile = 0ul,
            Generation = 0ul
        }
        );
        if (source.len == 0)
        {
            StoreVec(dest, result, true);
            ResetVec(ref source);
            StoreVec(src, source, true);
            return 0;
        }
        let align = source.elem_align == 0 ?1 : source.elem_align;
        let can_move = source.uses_inline == 0 && source.cap == source.len && ! NativePtr.IsNull(source.ptr);
        if (can_move)
        {
            result.ptr = source.ptr;
            result.len = source.len;
            result.cap = source.len;
            result.uses_inline = INLINE_FALSE;
        }
        else
        {
            if (! AllocateExact (ref result, source.len)) {
                StoreVec(dest, result, true);
                return 1;
            }
            if (source.elem_size >0 && ! NativePtr.IsNull (source.ptr))
            {
                NativeAlloc.Copy(MakeMut(result.ptr, source.len * source.elem_size, align), MakeConst(source.ptr, source.len * source.elem_size,
                align), source.len * source.elem_size);
            }
            result.len = source.len;
        }
        StoreVec(dest, result, true);
        ResetVec(ref source);
        StoreVec(src, source, true);
        return 0;
    }
    @export("chic_rt_array_into_vec") public unsafe static int chic_rt_array_into_vec(* mut ChicVec dest,
    * mut ChicVec src) {
        return chic_rt_vec_into_array(dest, src);
    }
    @extern("C") @export("chic_rt_vec_reserve") public unsafe static int chic_rt_vec_reserve(* mut ChicVec vec,
    usize additional) {
        if (IsNullVec (vec))
        {
            return 2;
        }
        return(int) EnsureCapacity(vec, additional);
    }
    @extern("C") @export("chic_rt_vec_shrink_to_fit") public unsafe static int chic_rt_vec_shrink_to_fit(* mut ChicVec vec) {
        if (IsNullVec (vec))
        {
            return 2;
        }
        var local = LoadVec(vec);
        if (local.uses_inline != 0 || local.len == local.cap)
        {
            StoreVec(vec, local, true);
            return 0;
        }
        let align = local.elem_align == 0 ?1 : local.elem_align;
        let bytes = local.len * local.elem_size;
        var updated = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = bytes, Alignment = align
        }
        ;
        if (NativeAlloc.Realloc (MakeMut (local.ptr, local.cap * local.elem_size, align), local.cap * local.elem_size, bytes,
        align, out updated) != NativeAllocationError.Success) {
            StoreVec(vec, local, true);
            return 1;
        }
        local.ptr = updated.Pointer;
        local.cap = local.len;
        StoreVec(vec, local, true);
        return 0;
    }
    @extern("C") @export("chic_rt_vec_push") public unsafe static int chic_rt_vec_push(* mut ChicVec vec,
    * const ValueConstPtr value) {
        let vec_addr = NativePtr.ToIsize(AsBytePtr(vec));
        if (vec_addr == 0)
        {
            return(int) VecError.InvalidPointer;
        }
        var * const @readonly @expose_address byte value_ptr = value;
        let value_addr = NativePtr.ToIsizeConst(value_ptr);
        if (value_addr == 0)
        {
            return(int) VecError.InvalidPointer;
        }
        let input = * value;
        let status = EnsureCapacity(vec, 1);
        if (status != VecError.Success)
        {
            return(int) status;
        }
        let align = (* vec).elem_align == 0 ?1 : (* vec).elem_align;
        var * mut @expose_address byte dst = NativePtr.OffsetMut((* vec).ptr, AsIsize((* vec).len * (* vec).elem_size));
        NativeAlloc.Copy(MakeMut(dst, (* vec).elem_size, align), input, (* vec).elem_size);
        (* vec).len += 1;
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_pop") public unsafe static int chic_rt_vec_pop(* mut ChicVec vec,
    * const ValueMutPtr outPtr) {
        if (IsNullVec (vec))
        {
            return(int) VecError.InvalidPointer;
        }
        var local = LoadVec(vec);
        if (local.len == 0)
        {
            return(int) VecError.OutOfBounds;
        }
        let out_handle = IsNullMutPtr(outPtr) ?MakeMut(NativePtr.NullMut(), 0, 0) : * outPtr;
        local.len -= 1;
        var * mut @expose_address byte src = NativePtr.OffsetMut(local.ptr, AsIsize(local.len * local.elem_size));
        if (! NativePtr.IsNull (out_handle.Pointer) && local.elem_size >0)
        {
            NativeAlloc.Copy(out_handle, MakeConst(src, local.elem_size, local.elem_align == 0 ?1 : local.elem_align), local.elem_size);
        }
        InvokeDrop(local.drop_fn, src);
        StoreVec(vec, local, true);
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_insert") public unsafe static int chic_rt_vec_insert(* mut ChicVec vec,
    usize index, * const ValueConstPtr value) {
        if (IsNullVec (vec))
        {
            return(int) VecError.InvalidPointer;
        }
        if (IsNullConstPtr (value))
        {
            return(int) VecError.InvalidPointer;
        }
        if (index > (* vec).len)
        {
            return(int) VecError.OutOfBounds;
        }
        let input = * value;
        let elem_bytes = (* vec).elem_size;
        if (elem_bytes == 0)
        {
            (* vec).len += 1;
            return(int) VecError.Success;
        }
        if (input.Size != elem_bytes)
        {
            return(int) VecError.InvalidPointer;
        }
        if (NativePtr.IsNull (input.Pointer))
        {
            return(int) VecError.InvalidPointer;
        }
        let status = EnsureCapacity(vec, 1);
        if (status != VecError.Success)
        {
            return(int) status;
        }
        let align = (* vec).elem_align == 0 ?1 : (* vec).elem_align;
        var * mut @expose_address byte base = (* vec).ptr;
        if ( (* vec).uses_inline != 0 || NativePtr.IsNull (base))
        {
            base = InlinePtr(vec);
        }
        var * mut @expose_address byte dest = NativePtr.OffsetMut(base, AsIsize(index * elem_bytes));
        let tail_bytes = ((* vec).len - index) * elem_bytes;
        if (tail_bytes >0)
        {
            NativeAlloc.Move(MakeMut(NativePtr.OffsetMut(dest, AsIsize(elem_bytes)), tail_bytes, align), MakeConst(NativePtr.AsConstPtr(dest),
            tail_bytes, align), tail_bytes);
        }
        NativeAlloc.Copy(MakeMut(dest, elem_bytes, align), input, elem_bytes);
        (* vec).len += 1;
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_remove") public unsafe static int chic_rt_vec_remove(* mut ChicVec vec,
    usize index, * const ValueMutPtr outPtr) {
        if (IsNullVec (vec))
        {
            return(int) VecError.InvalidPointer;
        }
        if (index >= (* vec).len)
        {
            return(int) VecError.OutOfBounds;
        }
        let out_handle = IsNullMutPtr(outPtr) ?MakeMut(NativePtr.NullMut(), 0, 0) : * outPtr;
        let align = (* vec).elem_align == 0 ?1 : (* vec).elem_align;
        let elem_bytes = (* vec).elem_size;
        var * mut @expose_address byte base = (* vec).ptr;
        if ( (* vec).uses_inline != 0 || NativePtr.IsNull (base))
        {
            base = InlinePtr(vec);
        }
        var * mut @expose_address byte src = NativePtr.OffsetMut(base, AsIsize(index * elem_bytes));
        if (! NativePtr.IsNull (out_handle.Pointer) && elem_bytes >0)
        {
            NativeAlloc.Copy(out_handle, MakeConst(src, elem_bytes, align), elem_bytes);
        }
        InvokeDrop((* vec).drop_fn, src);
        NativeAlloc.Move(MakeMut(src, ((* vec).len - index - 1) * elem_bytes, align), MakeConst(NativePtr.AsConstPtr(NativePtr.OffsetMut(src,
        AsIsize(elem_bytes))), ((* vec).len - index - 1) * elem_bytes, align), ((* vec).len - index - 1) * elem_bytes);
        (* vec).len -= 1;
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_swap_remove") public unsafe static int chic_rt_vec_swap_remove(* mut ChicVec vec,
    usize index, * const ValueMutPtr outPtr) {
        if (IsNullVec (vec))
        {
            return(int) VecError.InvalidPointer;
        }
        if (index >= (* vec).len)
        {
            return(int) VecError.OutOfBounds;
        }
        let out_handle = IsNullMutPtr(outPtr) ?MakeMut(NativePtr.NullMut(), 0, 0) : * outPtr;
        let align = (* vec).elem_align == 0 ?1 : (* vec).elem_align;
        let elem_bytes = (* vec).elem_size;
        var * mut @expose_address byte base = (* vec).ptr;
        if ( (* vec).uses_inline != 0 || NativePtr.IsNull (base))
        {
            base = InlinePtr(vec);
        }
        var * mut @expose_address byte target = NativePtr.OffsetMut(base, AsIsize(index * elem_bytes));
        var * mut @expose_address byte last = NativePtr.OffsetMut(base, AsIsize(((* vec).len - 1) * elem_bytes));
        if (! NativePtr.IsNull (out_handle.Pointer) && elem_bytes >0)
        {
            NativeAlloc.Copy(out_handle, MakeConst(target, elem_bytes, align), elem_bytes);
        }
        InvokeDrop((* vec).drop_fn, target);
        if (index != (* vec).len - 1)
        {
            NativeAlloc.Copy(MakeMut(target, elem_bytes, align), MakeConst(last, elem_bytes, align), elem_bytes);
        }
        (* vec).len -= 1;
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_truncate") public unsafe static int chic_rt_vec_truncate(* mut ChicVec vec,
    usize new_len) {
        if (IsNullVec (vec))
        {
            return 2;
        }
        var local = LoadVec(vec);
        if (new_len >= local.len)
        {
            return 0;
        }
        var index = new_len;
        while (index <local.len)
        {
            var * mut @expose_address byte ptr = NativePtr.OffsetMut(local.ptr, AsIsize(index * local.elem_size));
            InvokeDrop(local.drop_fn, ptr);
            index += 1;
        }
        local.len = new_len;
        StoreVec(vec, local, true);
        return 0;
    }
    @extern("C") @export("chic_rt_vec_clear") public unsafe static int chic_rt_vec_clear(* mut ChicVec vec) {
        if (IsNullVec (vec))
        {
            return 2;
        }
        return chic_rt_vec_truncate(vec, 0);
    }
    @extern("C") @export("chic_rt_vec_set_len") public unsafe static int chic_rt_vec_set_len(* mut ChicVec vec,
    usize new_len) {
        if (IsNullVec (vec))
        {
            return 2;
        }
        var local = LoadVec(vec);
        if (new_len >local.cap)
        {
            local.cap = new_len;
        }
        local.len = new_len;
        StoreVec(vec, local, true);
        return 0;
    }
    @extern("C") @export("chic_rt_vec_copy_to_array") public unsafe static int chic_rt_vec_copy_to_array(* mut ChicVec dest,
    * const ChicVec src) {
        return chic_rt_vec_clone(dest, src);
    }
    @export("chic_rt_array_copy_to_vec") public unsafe static int chic_rt_array_copy_to_vec(* mut ChicVec dest,
    * const ChicVec src) {
        return chic_rt_vec_clone(dest, src);
    }
    @extern("C") @export("chic_rt_vec_iter") public unsafe static ChicVecIter chic_rt_vec_iter(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return new ChicVecIter {
                data = NativePtr.NullConst(), index = 0, len = 0, elem_size = 0, elem_align = 0,
            }
            ;
        }
        var local = LoadVec(vec);
        var * const @readonly @expose_address byte dataPtr = local.ptr;
        if (local.uses_inline != 0 || NativePtr.IsNullConst (dataPtr))
        {
            dataPtr = InlinePtrConst(vec);
        }
        return new ChicVecIter {
            data = NativePtr.AsConstPtr(dataPtr), index = 0, len = local.len, elem_size = local.elem_size, elem_align = local.elem_align,
        }
        ;
    }
    @extern("C") @export("chic_rt_vec_iter_next") public unsafe static int chic_rt_vec_iter_next(ChicVecIter * iter,
    * const ValueMutPtr outPtr) {
        if (IsNullIter (iter))
        {
            return(int) VecError.InvalidPointer;
        }
        var local = LoadIter(iter);
        if (local.index >= local.len)
        {
            return(int) VecError.IterationComplete;
        }
        var out_handle = IsNullMutPtr(outPtr) ?MakeMut(NativePtr.NullMut(), 0, 0) : * outPtr;
        let src = NativePtr.OffsetConst(local.data, AsIsize(local.index * local.elem_size));
        if (! NativePtr.IsNull (out_handle.Pointer))
        {
            var srcHandle = MakeConst(src, local.elem_size, local.elem_align);
            NativeAlloc.Copy(out_handle, srcHandle, local.elem_size);
        }
        local.index += 1;
        StoreIter(iter, local);
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_iter_next_ptr") public unsafe static ValueConstPtr chic_rt_vec_iter_next_ptr(ChicVecIter * iter) {
        if (IsNullIter (iter))
        {
            return MakeConst(NativePtr.NullConst(), 0, 0);
        }
        var local = LoadIter(iter);
        if (local.index >= local.len)
        {
            return MakeConst(NativePtr.NullConst(), local.elem_size, local.elem_align);
        }
        var * const @readonly @expose_address byte ptr = NativePtr.OffsetConst(local.data, AsIsize(local.index * local.elem_size));
        local.index += 1;
        StoreIter(iter, local);
        return MakeConst(ptr, local.elem_size, local.elem_align);
    }
    @extern("C") @export("chic_rt_vec_len") public unsafe static usize chic_rt_vec_len(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 0;
        }
        var local = LoadVec(vec);
        return local.len;
    }
    @extern("C") @export("chic_rt_vec_capacity") public unsafe static usize chic_rt_vec_capacity(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 0;
        }
        var local = LoadVec(vec);
        return local.cap;
    }
    @extern("C") @export("chic_rt_vec_is_empty") public unsafe static int chic_rt_vec_is_empty(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 1;
        }
        var local = LoadVec(vec);
        return local.len == 0 ?1 : 0;
    }
    @extern("C") @export("chic_rt_vec_view") public unsafe static int chic_rt_vec_view(* const ChicVec vec,
    * mut ChicVecView dest) {
        if (IsNullView (dest))
        {
            return(int) VecError.InvalidPointer;
        }
        if (IsNullVecConst (vec))
        {
            * dest = new ChicVecView {
                data = NativePtr.NullConst(), len = 0, elem_size = 0, elem_align = 0,
            }
            ;
            return(int) VecError.Success;
        }
        var local = LoadVec(vec);
        var * const @readonly @expose_address byte dataPtr = local.ptr;
        if (local.uses_inline != 0 || NativePtr.IsNullConst (dataPtr))
        {
            dataPtr = InlinePtrConst(vec);
        }
        * dest = new ChicVecView {
            data = NativePtr.AsConstPtr(dataPtr), len = local.len, elem_size = local.elem_size, elem_align = local.elem_align,
        }
        ;
        return(int) VecError.Success;
    }
    @extern("C") @export("chic_rt_vec_data") public unsafe static ValueConstPtr chic_rt_vec_data(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return MakeConst(NativePtr.NullConst(), 0, 0);
        }
        var local = LoadVec(vec);
        let align = local.elem_align == 0 ?1 : local.elem_align;
        var * const @readonly @expose_address byte dataPtr = local.ptr;
        if (local.uses_inline != 0 || NativePtr.IsNullConst (dataPtr))
        {
            dataPtr = InlinePtrConst(vec);
        }
        return MakeConst(NativePtr.AsConstPtr(dataPtr), local.elem_size, align);
    }
    @extern("C") @export("chic_rt_vec_data_mut") public unsafe static ValueMutPtr chic_rt_vec_data_mut(* mut ChicVec vec) {
        if (IsNullVec (vec))
        {
            return MakeMut(NativePtr.NullMut(), 0, 0);
        }
        var local = LoadVec(vec);
        let align = local.elem_align == 0 ?1 : local.elem_align;
        var * mut @expose_address byte dataPtr = local.ptr;
        if (local.uses_inline != 0 || NativePtr.IsNull (dataPtr))
        {
            dataPtr = InlinePtr(vec);
        }
        return MakeMut(dataPtr, local.elem_size, align);
    }
    @extern("C") @export("chic_rt_vec_inline_capacity") public unsafe static usize chic_rt_vec_inline_capacity(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 0;
        }
        var local = LoadVec(vec);
        return InlineCapacity(local);
    }
    @extern("C") @export("chic_rt_vec_layout_debug") public unsafe static VecLayoutInfo chic_rt_vec_layout_debug() {
        var vec = MakeVec(1, 1, chic_rt_drop_noop_ptr(), NullRegion());
        var base = & vec;
        let baseAddr = NativePtr.ToIsize(AsBytePtr(base));
        var * mut @expose_address byte lenPtr = & mut(* base).len;
        var * mut @expose_address byte capPtr = & mut(* base).cap;
        var * mut @expose_address byte elemSizePtr = & mut(* base).elem_size;
        var * mut @expose_address byte elemAlignPtr = & mut(* base).elem_align;
        var dropFieldPtr = & mut(* base).drop_fn;
        var * mut @expose_address byte dropPtr = (* mut @expose_address byte) dropFieldPtr;
        var * mut @expose_address byte regionPtr = & mut(* base).region_ptr;
        var * mut @expose_address byte usesInlinePtr = & mut(* base).uses_inline;
        var inlinePtr = InlinePtr(base);
        return new VecLayoutInfo {
            size = sizeof(ChicVec), offset_ptr = 0, offset_len = NativePtr.ToIsize(lenPtr) - baseAddr, offset_cap = NativePtr.ToIsize(capPtr) - baseAddr, offset_elem_size = NativePtr.ToIsize(elemSizePtr) - baseAddr, offset_elem_align = NativePtr.ToIsize(elemAlignPtr) - baseAddr, offset_drop_fn = NativePtr.ToIsize(dropPtr) - baseAddr, offset_region_ptr = NativePtr.ToIsize(regionPtr) - baseAddr, offset_uses_inline = NativePtr.ToIsize(usesInlinePtr) - baseAddr, offset_inline_storage = NativePtr.ToIsize(inlinePtr) - baseAddr,
        }
        ;
    }
    @extern("C") @export("chic_rt_vec_inline_ptr") public unsafe static ValueMutPtr chic_rt_vec_inline_ptr(* mut ChicVec vec) {
        if (IsNullVec (vec))
        {
            return MakeMut(NativePtr.NullMut(), 0, 0);
        }
        var local = LoadVec(vec);
        var * mut @expose_address byte ptr = InlinePtr(vec);
        let align = local.elem_align == 0 ?1 : local.elem_align;
        return MakeMut(ptr, local.elem_size, align);
    }
    @extern("C") @export("chic_rt_vec_mark_inline") public unsafe static void chic_rt_vec_mark_inline(* mut ChicVec vec,
    int uses_inline) {
        if (IsNullVec (vec))
        {
            return;
        }
        var local = LoadVec(vec);
        local.uses_inline = uses_inline != 0 ?INLINE_TRUE : INLINE_FALSE;
        StoreVec(vec, local, true);
    }
    @extern("C") @export("chic_rt_vec_uses_inline") public unsafe static int chic_rt_vec_uses_inline(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 0;
        }
        var local = LoadVec(vec);
        return local.uses_inline != 0 ?1 : 0;
    }
    @extern("C") @export("chic_rt_vec_ptr_at") public unsafe static ValueMutPtr chic_rt_vec_ptr_at(* const ChicVec vec,
    usize index) {
        if (IsNullVecConst (vec))
        {
            return MakeMut(NativePtr.NullMut(), 0, 0);
        }
        var local = LoadVec(vec);
        if (index >= local.len)
        {
            return MakeMut(NativePtr.NullMut(), local.elem_size, local.elem_align);
        }
        var * mut @expose_address byte basePtr = local.ptr;
        if (local.uses_inline != 0 || NativePtr.IsNull (basePtr))
        {
            basePtr = NativePtr.AsMutPtr(InlinePtrConst(vec));
        }
        var * mut @expose_address byte ptr = NativePtr.OffsetMut(basePtr, AsIsize(index * local.elem_size));
        return MakeMut(ptr, local.elem_size, local.elem_align);
    }
    @extern("C") @export("chic_rt_vec_get_ptr") public unsafe static ValueMutPtr chic_rt_vec_get_ptr(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return MakeMut(NativePtr.NullMut(), 0, 0);
        }
        var local = LoadVec(vec);
        var * mut @expose_address byte ptr = local.ptr;
        if (local.uses_inline != 0 || NativePtr.IsNull (ptr))
        {
            ptr = NativePtr.AsMutPtr(InlinePtrConst(vec));
        }
        return MakeMut(ptr, local.elem_size, local.elem_align);
    }
    @extern("C") @export("chic_rt_vec_set_ptr") public unsafe static void chic_rt_vec_set_ptr(* mut ChicVec vec,
    * const ValueMutPtr ptr) {
        if (IsNullVec (vec))
        {
            return;
        }
        if (IsNullMutPtr (ptr))
        {
            return;
        }
        let handle = * ptr;
        var local = LoadVec(vec);
        local.ptr = handle.Pointer;
        local.elem_size = handle.Size;
        local.elem_align = handle.Alignment;
        StoreVec(vec, local, true);
    }
    @extern("C") @export("chic_rt_vec_set_cap") public unsafe static void chic_rt_vec_set_cap(* mut ChicVec vec,
    usize cap) {
        if (IsNullVec (vec))
        {
            return;
        }
        var local = LoadVec(vec);
        local.cap = cap;
        StoreVec(vec, local, true);
    }
    @extern("C") @export("chic_rt_vec_elem_size") public unsafe static usize chic_rt_vec_elem_size(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 0;
        }
        var local = LoadVec(vec);
        return local.elem_size;
    }
    @extern("C") @export("chic_rt_vec_elem_align") public unsafe static usize chic_rt_vec_elem_align(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return 0;
        }
        var local = LoadVec(vec);
        return local.elem_align;
    }
    @extern("C") @export("chic_rt_vec_set_elem_size") public unsafe static void chic_rt_vec_set_elem_size(* mut ChicVec vec,
    usize size) {
        if (IsNullVec (vec))
        {
            return;
        }
        var local = LoadVec(vec);
        local.elem_size = size;
        StoreVec(vec, local, true);
    }
    @extern("C") @export("chic_rt_vec_set_elem_align") public unsafe static void chic_rt_vec_set_elem_align(* mut ChicVec vec,
    usize align) {
        if (IsNullVec (vec))
        {
            return;
        }
        var local = LoadVec(vec);
        local.elem_align = align;
        StoreVec(vec, local, true);
    }
    @extern("C") @export("chic_rt_vec_get_drop") public unsafe static fn @extern("C")(* mut @expose_address byte) -> void chic_rt_vec_get_drop(* const ChicVec vec) {
        if (IsNullVecConst (vec))
        {
            return chic_rt_drop_noop_ptr();
        }
        var local = LoadVec(vec);
        return local.drop_fn;
    }
    @extern("C") @export("chic_rt_vec_set_drop") public unsafe static void chic_rt_vec_set_drop(* mut ChicVec vec,
    fn @extern("C")(* mut @expose_address byte) -> void drop_fn) {
        if (IsNullVec (vec))
        {
            return;
        }
        var local = LoadVec(vec);
        local.drop_fn = drop_fn;
        StoreVec(vec, local, true);
    }
}
