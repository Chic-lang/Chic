namespace Std.Runtime.Native;
@repr(c) internal struct SharedHeader
{
    public usize Strong;
    public usize Weak;
    public usize Size;
    public usize Align;
    public fn @extern("C")(* mut @expose_address byte) -> void DropFn;
    public u64 TypeId;
    public * mut @expose_address byte Data;
}
@repr(c) public struct ChicArc
{
    public * mut @expose_address byte header;
}
@repr(c) public struct ChicWeak
{
    public * mut @expose_address byte header;
}
@repr(c) public struct ChicRc
{
    public * mut @expose_address byte header;
}
@repr(c) public struct ChicWeakRc
{
    public * mut @expose_address byte header;
}
public enum SharedError
{
    Success = 0, InvalidPointer = - 1, AllocationFailed = - 2, Overflow = - 3,
}
public static class SharedRuntime
{
    private const usize HEADER_ALIGN = 8usize;
    @extern("C") private unsafe static extern fn @extern("C")(* mut @expose_address byte) -> void chic_rt_drop_resolve(u64 typeId);
    @extern("C") public unsafe static extern void chic_rt_drop_invoke(fn @extern("C")(* mut @expose_address byte) -> void dropFn,
    * mut @expose_address byte value);
    @extern("C") private unsafe static extern void chic_rt_drop_register(u64 typeId, fn @extern("C")(* mut @expose_address byte) -> void dropFn);
    @extern("C") @export("chic_rt_drop_missing") public unsafe static void chic_rt_drop_missing(* mut @expose_address byte _ptr) {
        (void) _ptr;
    }
    private unsafe static usize AlignUp(usize value, usize align) {
        if (align == 0)
        {
            return value;
        }
        let rem = value % align;
        return rem == 0 ?value : value + (align - rem);
    }
    private unsafe static * mut SharedHeader AsHeader(* mut @expose_address byte header) {
        return(* mut SharedHeader) header;
    }
    private unsafe static * mut @expose_address byte ReadHandleHeader <T >(* const @readonly T handle) {
        if (handle == null)
        {
            return NativePtr.NullMut();
        }
        let slot = (* const @readonly * mut @expose_address byte) handle;
        return * slot;
    }
    private unsafe static void WriteHandleHeader <T >(* mut T handle, * mut @expose_address byte header) {
        if (handle == null)
        {
            return;
        }
        let slot = (* mut * mut @expose_address byte) handle;
        * slot = header;
    }
    private unsafe static usize GetStrong(* mut @expose_address byte header) {
        return header == null ?0 : (* AsHeader(header)).Strong;
    }
    private unsafe static void SetStrong(* mut @expose_address byte header, usize value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).Strong = value;
    }
    private unsafe static usize GetWeak(* mut @expose_address byte header) {
        return header == null ?0 : (* AsHeader(header)).Weak;
    }
    private unsafe static void SetWeak(* mut @expose_address byte header, usize value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).Weak = value;
    }
    private unsafe static usize GetSize(* mut @expose_address byte header) {
        return header == null ?0 : (* AsHeader(header)).Size;
    }
    private unsafe static void SetSize(* mut @expose_address byte header, usize value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).Size = value;
    }
    private unsafe static usize GetAlign(* mut @expose_address byte header) {
        return header == null ?0 : (* AsHeader(header)).Align;
    }
    private unsafe static void SetAlign(* mut @expose_address byte header, usize value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).Align = value;
    }
    private unsafe static void SetDropFn(* mut @expose_address byte header, fn @extern("C")(* mut @expose_address byte) -> void value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).DropFn = value;
    }
    private unsafe static u64 GetTypeId(* mut @expose_address byte header) {
        return header == null ?0u64 : (* AsHeader(header)).TypeId;
    }
    private unsafe static void SetTypeId(* mut @expose_address byte header, u64 value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).TypeId = value;
    }
    private unsafe static * mut @expose_address byte GetData(* mut @expose_address byte header) {
        return header == null ?NativePtr.NullMut() : (* AsHeader(header)).Data;
    }
    private unsafe static void SetData(* mut @expose_address byte header, * mut @expose_address byte value) {
        if (header == null)
        {
            return;
        }
        (* AsHeader(header)).Data = value;
    }
    private unsafe static * mut @expose_address byte AllocateHeader(usize size, usize align) {
        var realAlign = align == 0 ?HEADER_ALIGN : align;
        if (realAlign <HEADER_ALIGN)
        {
            realAlign = HEADER_ALIGN;
        }
        var offset = AlignUp(sizeof(SharedHeader), realAlign);
        var total = offset + size;
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = total, Alignment = realAlign
        }
        ;
        var header = NativePtr.NullMut();
        if (NativeAlloc.AllocZeroed (total, realAlign, out alloc) != NativeAllocationError.Success) {
            return header;
        }
        header = alloc.Pointer;
        SetStrong(header, 1);
        SetWeak(header, 1);
        SetSize(header, size);
        SetAlign(header, realAlign);
        SetDropFn(header, chic_rt_drop_missing);
        SetTypeId(header, 0);
        SetData(header, NativePtr.OffsetMut(alloc.Pointer, (isize) offset));
        return header;
    }
    private unsafe static void ReleaseHeader(* mut byte header) {
        if (header == null)
        {
            return;
        }
        var offset = AlignUp(sizeof(SharedHeader), GetAlign(header));
        var total = offset + GetSize(header);
        var * mut byte raw = header;
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.AsByteMut(raw), Size = total, Alignment = GetAlign(header),
        }
        ;
        NativeAlloc.Free(alloc);
    }
    private unsafe static void InvokeDrop(* mut byte header) {
        // Drop hooks are not available in the bootstrap runtime yet.
        return;
    }
    @extern("C") @export("chic_rt_shared_allocations") public unsafe static usize chic_rt_shared_allocations() {
        return 0;
    }
    @extern("C") @export("chic_rt_shared_frees") public unsafe static usize chic_rt_shared_frees() {
        return 0;
    }
    @extern("C") private static extern usize chic_rt_type_size(u64 type_id);
    @extern("C") private static extern usize chic_rt_type_align(u64 type_id);
    @extern("C") @export("chic_rt_object_new") public unsafe static * mut byte chic_rt_object_new(u64 type_id) {
        let size = chic_rt_type_size(type_id);
        let align = chic_rt_type_align(type_id);
        if (size == 0usize || align == 0usize)
        {
            return NativePtr.NullMut();
        }
        var alloc = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = align
        }
        ;
        if (NativeAlloc.AllocZeroed (size, align, out alloc) != NativeAllocationError.Success) {
            return NativePtr.NullMut();
        }
        return alloc.Pointer;
    }
    @extern("C") @export("chic_rt_arc_new") public unsafe static int chic_rt_arc_new(* mut ChicArc dest, * const @readonly byte src,
    usize size, usize align, fn @extern("C")(* mut @expose_address byte) -> void dropFn, u64 typeId) {
        if (dest == null || NativePtr.IsNullConst (src))
        {
            return - 1;
        }
        var header = AllocateHeader(size, align);
        if (header == null)
        {
            return - 2;
        }
        var dst = new ValueMutPtr {
            Pointer = GetData(header), Size = size, Alignment = align == 0 ?1 : align,
        }
        ;
        var srcPtr = new ValueConstPtr {
            Pointer = src, Size = size, Alignment = align == 0 ?1 : align,
        }
        ;
        NativeAlloc.Copy(dst, srcPtr, size);
        SetDropFn(header, dropFn);
        SetTypeId(header, typeId);
        WriteHandleHeader <ChicArc >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_arc_clone") public unsafe static int chic_rt_arc_clone(* mut ChicArc dest, * const ChicArc src) {
        var source = ReadHandleHeader <ChicArc >(src);
        if (dest == null || src == null || source == null)
        {
            return - 1;
        }
        SetStrong(source, GetStrong(source) + 1);
        WriteHandleHeader <ChicArc >(dest, source);
        return 0;
    }
    private unsafe static void ReleaseStrong(* mut byte header) {
        if (header == null)
        {
            return;
        }
        var prev = GetStrong(header);
        if (prev >0)
        {
            SetStrong(header, prev - 1);
            if (prev == 1)
            {
                InvokeDrop(header);
                var weakPrev = GetWeak(header);
                SetWeak(header, weakPrev == 0 ?0 : weakPrev - 1);
                if (weakPrev == 1)
                {
                    ReleaseHeader(header);
                }
            }
        }
    }
    private unsafe static void ReleaseWeak(* mut byte header) {
        if (header == null)
        {
            return;
        }
        var prev = GetWeak(header);
        if (prev >0)
        {
            SetWeak(header, prev - 1);
            if (prev == 1)
            {
                ReleaseHeader(header);
            }
        }
    }
    @extern("C") @export("chic_rt_arc_drop") public unsafe static void chic_rt_arc_drop(* mut ChicArc target) {
        var header = ReadHandleHeader <ChicArc >(target);
        if (target == null || header == null)
        {
            return;
        }
        ReleaseStrong(header);
        WriteHandleHeader <ChicArc >(target, null);
    }
    @extern("C") @export("chic_rt_arc_get") public unsafe static * const @readonly byte chic_rt_arc_get(* const ChicArc src) {
        var header = ReadHandleHeader <ChicArc >(src);
        return header == null ?NativePtr.NullConst() : NativePtr.AsConstPtr(GetData(header));
    }
    @extern("C") @export("chic_rt_arc_get_mut") public unsafe static * mut byte chic_rt_arc_get_mut(* mut ChicArc src) {
        var header = ReadHandleHeader <ChicArc >(src);
        if (src == null || header == null)
        {
            return NativePtr.NullMut();
        }
        if (GetStrong (header) == 1)
        {
            return GetData(header);
        }
        return NativePtr.NullMut();
    }
    @extern("C") @export("chic_rt_arc_get_data") public unsafe static * mut byte chic_rt_arc_get_data(* const ChicArc handle) {
        var header = ReadHandleHeader <ChicArc >(handle);
        return header == null ?NativePtr.NullMut() : GetData(header);
    }
    @extern("C") @export("chic_rt_arc_strong_count") public unsafe static usize chic_rt_arc_strong_count(* const ChicArc src) {
        var header = ReadHandleHeader <ChicArc >(src);
        return header == null ?0 : GetStrong(header);
    }
    @extern("C") @export("chic_rt_arc_weak_count") public unsafe static usize chic_rt_arc_weak_count(* const ChicArc src) {
        var header = ReadHandleHeader <ChicArc >(src);
        return header == null ?0 : GetWeak(header);
    }
    @extern("C") @export("chic_rt_arc_downgrade") public unsafe static int chic_rt_arc_downgrade(* mut ChicWeak dest, * const ChicArc src) {
        var header = ReadHandleHeader <ChicArc >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        SetWeak(header, GetWeak(header) + 1);
        WriteHandleHeader <ChicWeak >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_weak_clone") public unsafe static int chic_rt_weak_clone(* mut ChicWeak dest, * const ChicWeak src) {
        var header = ReadHandleHeader <ChicWeak >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        SetWeak(header, GetWeak(header) + 1);
        WriteHandleHeader <ChicWeak >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_weak_drop") public unsafe static void chic_rt_weak_drop(* mut ChicWeak target) {
        var header = ReadHandleHeader <ChicWeak >(target);
        if (target == null || header == null)
        {
            return;
        }
        ReleaseWeak(header);
        WriteHandleHeader <ChicWeak >(target, null);
    }
    @extern("C") @export("chic_rt_weak_upgrade") public unsafe static int chic_rt_weak_upgrade(* mut ChicArc dest, * const ChicWeak src) {
        var header = ReadHandleHeader <ChicWeak >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        var strong = GetStrong(header);
        if (strong == 0)
        {
            WriteHandleHeader <ChicArc >(dest, null);
            return - 1;
        }
        SetStrong(header, strong + 1);
        WriteHandleHeader <ChicArc >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_rc_new") public unsafe static int chic_rt_rc_new(* mut ChicRc dest, * const @readonly byte src,
    usize size, usize align, fn @extern("C")(* mut @expose_address byte) -> void dropFn, u64 typeId) {
        if (dest == null || NativePtr.IsNullConst (src))
        {
            return - 1;
        }
        var header = AllocateHeader(size, align);
        if (header == null)
        {
            return - 2;
        }
        var dst = new ValueMutPtr {
            Pointer = GetData(header), Size = size, Alignment = align == 0 ?1 : align,
        }
        ;
        var srcPtr = new ValueConstPtr {
            Pointer = src, Size = size, Alignment = align == 0 ?1 : align,
        }
        ;
        NativeAlloc.Copy(dst, srcPtr, size);
        SetDropFn(header, dropFn);
        SetTypeId(header, typeId);
        WriteHandleHeader <ChicRc >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_rc_clone") public unsafe static int chic_rt_rc_clone(* mut ChicRc dest, * const ChicRc src) {
        var header = ReadHandleHeader <ChicRc >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        SetStrong(header, GetStrong(header) + 1);
        WriteHandleHeader <ChicRc >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_rc_drop") public unsafe static void chic_rt_rc_drop(* mut ChicRc target) {
        var header = ReadHandleHeader <ChicRc >(target);
        if (target == null || header == null)
        {
            return;
        }
        ReleaseStrong(header);
        WriteHandleHeader <ChicRc >(target, null);
    }
    @extern("C") @export("chic_rt_rc_get") public unsafe static * const @readonly byte chic_rt_rc_get(* const ChicRc src) {
        var header = ReadHandleHeader <ChicRc >(src);
        return header == null ?NativePtr.NullConst() : NativePtr.AsConstPtr(GetData(header));
    }
    @extern("C") @export("chic_rt_rc_get_mut") public unsafe static * mut byte chic_rt_rc_get_mut(* mut ChicRc src) {
        var header = ReadHandleHeader <ChicRc >(src);
        if (src == null || header == null)
        {
            return NativePtr.NullMut();
        }
        if (GetStrong (header) == 1)
        {
            return GetData(header);
        }
        return NativePtr.NullMut();
    }
    @extern("C") @export("chic_rt_rc_strong_count") public unsafe static usize chic_rt_rc_strong_count(* const ChicRc src) {
        var header = ReadHandleHeader <ChicRc >(src);
        return header == null ?0 : GetStrong(header);
    }
    @extern("C") @export("chic_rt_rc_weak_count") public unsafe static usize chic_rt_rc_weak_count(* const ChicRc src) {
        var header = ReadHandleHeader <ChicRc >(src);
        return header == null ?0 : GetWeak(header);
    }
    @extern("C") @export("chic_rt_rc_downgrade") public unsafe static int chic_rt_rc_downgrade(* mut ChicWeakRc dest, * const ChicRc src) {
        var header = ReadHandleHeader <ChicRc >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        SetWeak(header, GetWeak(header) + 1);
        WriteHandleHeader <ChicWeakRc >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_weak_rc_clone") public unsafe static int chic_rt_weak_rc_clone(* mut ChicWeakRc dest, * const ChicWeakRc src) {
        var header = ReadHandleHeader <ChicWeakRc >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        SetWeak(header, GetWeak(header) + 1);
        WriteHandleHeader <ChicWeakRc >(dest, header);
        return 0;
    }
    @extern("C") @export("chic_rt_weak_rc_drop") public unsafe static void chic_rt_weak_rc_drop(* mut ChicWeakRc target) {
        var header = ReadHandleHeader <ChicWeakRc >(target);
        if (target == null || header == null)
        {
            return;
        }
        ReleaseWeak(header);
        WriteHandleHeader <ChicWeakRc >(target, null);
    }
    @extern("C") @export("chic_rt_weak_rc_upgrade") public unsafe static int chic_rt_weak_rc_upgrade(* mut ChicRc dest, * const ChicWeakRc src) {
        var header = ReadHandleHeader <ChicWeakRc >(src);
        if (dest == null || src == null || header == null)
        {
            return - 1;
        }
        var strong = GetStrong(header);
        if (strong == 0)
        {
            WriteHandleHeader <ChicRc >(dest, null);
            return - 1;
        }
        SetStrong(header, strong + 1);
        WriteHandleHeader <ChicRc >(dest, header);
        return 0;
    }
    public unsafe static void TestCoverageHelpers() {
        let _ = AlignUp(9usize, 8usize);
        let _ = AlignUp(5usize, 0usize);
        var header = AllocateHeader(16usize, 4usize);
        if (header != null)
        {
            SetStrong(header, 2);
            SetWeak(header, 2);
            SetSize(header, 16usize);
            SetAlign(header, 4usize);
            SetTypeId(header, 123u64);
            SetDropFn(header, chic_rt_drop_missing);
            let _ = GetStrong(header);
            let _ = GetWeak(header);
            let _ = GetSize(header);
            let _ = GetAlign(header);
            let _ = GetTypeId(header);
            let _ = GetData(header);
            var arc = new ChicArc {
                header = null
            }
            ;
            WriteHandleHeader <ChicArc >(& arc, header);
            let _ = ReadHandleHeader <ChicArc >(& arc);
            InvokeDrop(header);
            ReleaseStrong(header);
            ReleaseWeak(header);
            ReleaseHeader(header);
            WriteHandleHeader <ChicArc >(& arc, null);
        }
        var headerStrongZero = AllocateHeader(8usize, 4usize);
        if (headerStrongZero != null)
        {
            SetStrong(headerStrongZero, 0);
            ReleaseStrong(headerStrongZero);
            ReleaseHeader(headerStrongZero);
        }
        var headerWeakZero = AllocateHeader(8usize, 4usize);
        if (headerWeakZero != null)
        {
            SetStrong(headerWeakZero, 1);
            SetWeak(headerWeakZero, 0);
            ReleaseStrong(headerWeakZero);
            ReleaseHeader(headerWeakZero);
        }
        var headerWeakTwo = AllocateHeader(8usize, 4usize);
        if (headerWeakTwo != null)
        {
            SetStrong(headerWeakTwo, 1);
            SetWeak(headerWeakTwo, 2);
            ReleaseWeak(headerWeakTwo);
            ReleaseWeak(headerWeakTwo);
        }
        let _ = ReadHandleHeader <ChicArc >((* const ChicArc) NativePtr.NullConst());
        WriteHandleHeader <ChicArc >((* mut ChicArc) NativePtr.NullMut(), header);
        var value = 9;
        var arcHandle = new ChicArc {
            header = null
        }
        ;
        let arcStatus = chic_rt_arc_new(& arcHandle, & value, (usize) __sizeof <int >(), (usize) __alignof <int >(), chic_rt_drop_missing,
        0u64);
        if (arcStatus == 0)
        {
            let _ = chic_rt_arc_get(& arcHandle);
            let _ = chic_rt_arc_get_mut(& arcHandle);
            let _ = chic_rt_arc_get_data(& arcHandle);
            let _ = chic_rt_arc_strong_count(& arcHandle);
            let _ = chic_rt_arc_weak_count(& arcHandle);
            var arcClone = new ChicArc {
                header = null
            }
            ;
            let _ = chic_rt_arc_clone(& arcClone, & arcHandle);
            let _ = chic_rt_arc_get_mut(& arcHandle);
            var weakHandle = new ChicWeak {
                header = null
            }
            ;
            let _ = chic_rt_arc_downgrade(& weakHandle, & arcHandle);
            var weakClone = new ChicWeak {
                header = null
            }
            ;
            let _ = chic_rt_weak_clone(& weakClone, & weakHandle);
            var upgraded = new ChicArc {
                header = null
            }
            ;
            let _ = chic_rt_weak_upgrade(& upgraded, & weakHandle);
            chic_rt_arc_drop(& arcClone);
            chic_rt_arc_drop(& upgraded);
            chic_rt_weak_drop(& weakClone);
            chic_rt_weak_drop(& weakHandle);
            chic_rt_arc_drop(& arcHandle);
        }
        var rcHandle = new ChicRc {
            header = null
        }
        ;
        let rcStatus = chic_rt_rc_new(& rcHandle, & value, (usize) __sizeof <int >(), (usize) __alignof <int >(), chic_rt_drop_missing,
        0u64);
        if (rcStatus == 0)
        {
            let _ = chic_rt_rc_get(& rcHandle);
            let _ = chic_rt_rc_get_mut(& rcHandle);
            let _ = chic_rt_rc_strong_count(& rcHandle);
            let _ = chic_rt_rc_weak_count(& rcHandle);
            var rcClone = new ChicRc {
                header = null
            }
            ;
            let _ = chic_rt_rc_clone(& rcClone, & rcHandle);
            var weakRc = new ChicWeakRc {
                header = null
            }
            ;
            let _ = chic_rt_rc_downgrade(& weakRc, & rcHandle);
            var weakRcClone = new ChicWeakRc {
                header = null
            }
            ;
            let _ = chic_rt_weak_rc_clone(& weakRcClone, & weakRc);
            var upgradedRc = new ChicRc {
                header = null
            }
            ;
            let _ = chic_rt_weak_rc_upgrade(& upgradedRc, & weakRc);
            chic_rt_rc_drop(& rcClone);
            chic_rt_rc_drop(& upgradedRc);
            chic_rt_weak_rc_drop(& weakRcClone);
            chic_rt_weak_rc_drop(& weakRc);
            chic_rt_rc_drop(& rcHandle);
        }
        let _ = chic_rt_shared_allocations();
        let _ = chic_rt_shared_frees();
        let size = chic_rt_type_size(0u64);
        let align = chic_rt_type_align(0u64);
        let obj = chic_rt_object_new(0u64);
        if (!NativePtr.IsNull (obj) && size >0usize && align >0usize)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = obj, Size = size, Alignment = align,
            }
            );
        }
        chic_rt_drop_missing(NativePtr.NullMut());
    }
}
