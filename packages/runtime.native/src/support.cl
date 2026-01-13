namespace Std.Runtime.Native;
// Minimal pointer/atomic/alloc helpers for the native runtime. No shim exports
// remain; everything here is internal plumbing used by other native runtime
// modules.
public enum MemoryOrder
{
    Relaxed, Acquire, Release, AcqRel, SeqCst,
}
public struct AtomicUsize
{
    private usize _value;
    public init(usize value = 0) {
        _value = value;
    }
    public usize Load(MemoryOrder order = MemoryOrder.SeqCst) {
        return _value;
    }
    public void Store(usize value, MemoryOrder order = MemoryOrder.SeqCst) {
        _value = value;
    }
    public usize FetchAdd(usize value, MemoryOrder order = MemoryOrder.SeqCst) {
        let previous = _value;
        _value = previous + value;
        return previous;
    }
    public usize FetchSub(usize value, MemoryOrder order = MemoryOrder.SeqCst) {
        let previous = _value;
        _value = previous - value;
        return previous;
    }
}
public enum NativeAllocationError
{
    Success = 0, AllocationFailed = 2,
}
public static class NativePtr
{
    public unsafe static * mut @expose_address byte NullMut() {
        return(* mut @expose_address byte) 0;
    }
    public unsafe static * const @readonly @expose_address byte NullConst() {
        return(* const @readonly @expose_address byte) 0;
    }
    public unsafe static bool IsNull(* mut @expose_address byte ptr) {
        return(nuint) ptr == 0;
    }
    public unsafe static bool IsNullConst(* const @readonly @expose_address byte ptr) {
        return(nuint) ptr == 0;
    }
    public unsafe static * mut @expose_address byte FromIsize(isize value) {
        return(* mut @expose_address byte) value;
    }
    public unsafe static * const @readonly @expose_address byte FromIsizeConst(isize value) {
        return(* const @readonly @expose_address byte) value;
    }
    public unsafe static isize ToIsize(* mut @expose_address byte ptr) {
        return(isize) ptr;
    }
    public unsafe static isize ToIsizeConst(* const @readonly @expose_address byte ptr) {
        return(isize) ptr;
    }
    public unsafe static * mut @expose_address byte OffsetMut(* mut @expose_address byte ptr, isize offset) {
        if (ptr == null || offset == 0)
        {
            return ptr;
        }
        let base = (isize) ptr;
        return(* mut @expose_address byte)(base + offset);
    }
    public unsafe static * const @readonly @expose_address byte OffsetConst(* const @readonly @expose_address byte ptr, isize offset) {
        if (ptr == null || offset == 0)
        {
            return ptr;
        }
        let base = (isize) ptr;
        return(* const @readonly @expose_address byte)(base + offset);
    }
    public unsafe static * mut @expose_address byte AsByteMut(* mut @expose_address byte ptr) {
        return ptr;
    }
    public unsafe static * const @readonly @expose_address byte AsByteConst(* const @readonly @expose_address byte ptr) {
        return ptr;
    }
    public unsafe static * const @readonly @expose_address byte AsConstPtr(* const @readonly @expose_address byte ptr) {
        return ptr;
    }
    public unsafe static * mut @expose_address byte AsMutPtr(* const @readonly @expose_address byte ptr) {
        return(* mut @expose_address byte) ptr;
    }
    public unsafe static byte ReadByteConst(* const @readonly @expose_address byte ptr) {
        return * ptr;
    }
    public unsafe static byte ReadByteMut(* mut @expose_address byte ptr) {
        return * ptr;
    }
}
public static class NativeAlloc
{
    private static int _test_fail_alloc_after = - 1;
    private static int _test_fail_realloc_after = - 1;
    private static int _test_fail_sys_alloc_count = 0;
    @extern("C") private unsafe static extern int posix_memalign(* mut * mut @expose_address byte out_ptr, usize align, usize size);
    @extern("C") private unsafe static extern * mut @expose_address byte malloc(usize size);
    @extern("C") private unsafe static extern * mut @expose_address byte calloc(usize count, usize size);
    @extern("C") private unsafe static extern * mut @expose_address byte realloc(* mut @expose_address byte ptr, usize size);
    @extern("C") private unsafe static extern void free(* mut @expose_address byte ptr);
    @extern("C") private unsafe static extern void memcpy(* mut @expose_address byte dest, * const @readonly @expose_address byte src,
    usize len);
    @extern("C") private unsafe static extern void memmove(* mut @expose_address byte dest, * const @readonly @expose_address byte src,
    usize len);
    @extern("C") private unsafe static extern void memset(* mut @expose_address byte dest, byte value, usize len);
    public static void TestReset() {
        _test_fail_alloc_after = - 1;
        _test_fail_realloc_after = - 1;
        _test_fail_sys_alloc_count = 0;
    }
    public static void TestFailAllocAfter(int remaining) {
        _test_fail_alloc_after = remaining;
    }
    public static void TestFailReallocAfter(int remaining) {
        _test_fail_realloc_after = remaining;
    }
    public static void TestFailSysAllocCount(int count) {
        _test_fail_sys_alloc_count = count <0 ?0 : count;
    }
    private static bool ShouldFailAlloc() {
        if (_test_fail_alloc_after < 0)
        {
            return false;
        }
        if (_test_fail_alloc_after == 0)
        {
            _test_fail_alloc_after = - 1;
            return true;
        }
        _test_fail_alloc_after = _test_fail_alloc_after - 1;
        return false;
    }
    private static bool ShouldFailRealloc() {
        if (_test_fail_realloc_after < 0)
        {
            return false;
        }
        if (_test_fail_realloc_after == 0)
        {
            _test_fail_realloc_after = - 1;
            return true;
        }
        _test_fail_realloc_after = _test_fail_realloc_after - 1;
        return false;
    }
    private static bool ShouldFailSysAlloc() {
        if (_test_fail_sys_alloc_count <= 0)
        {
            return false;
        }
        _test_fail_sys_alloc_count = _test_fail_sys_alloc_count - 1;
        return true;
    }
    public unsafe static NativeAllocationError Alloc(usize size, usize align, out ValueMutPtr result) {
        var handle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = align,
        }
        ;
        var * mut @expose_address byte ptr = NativePtr.NullMut();
        if (size == 0)
        {
            result = handle;
            return NativeAllocationError.Success;
        }
        if (ShouldFailAlloc())
        {
            result = handle;
            return NativeAllocationError.AllocationFailed;
        }
        if (align <= sizeof(void * )) {
            ptr = ShouldFailSysAlloc() ?NativePtr.NullMut() : malloc(size);
            if (NativePtr.IsNull (ptr))
            {
                ptr = ShouldFailSysAlloc() ?NativePtr.NullMut() : calloc(1usize, size);
            }
        }
        else
        {
            let status = ShouldFailSysAlloc() ?1 : posix_memalign(& ptr, align, size);
            if (status != 0 || NativePtr.IsNull (ptr))
            {
                ptr = ShouldFailSysAlloc() ?NativePtr.NullMut() : malloc(size);
                if (NativePtr.IsNull (ptr))
                {
                    ptr = ShouldFailSysAlloc() ?NativePtr.NullMut() : calloc(1usize, size);
                }
            }
        }
        if (NativePtr.IsNull (ptr))
        {
            let fallback_align = align <= sizeof(void * ) ?sizeof(void * ) : align;
            let status = ShouldFailSysAlloc() ?1 : posix_memalign(& ptr, fallback_align, size);
            if (status == 0 && ! NativePtr.IsNull (ptr))
            {
                memset(ptr, 0u8, size);
            }
        }
        handle.Pointer = ptr;
        result = handle;
        return NativePtr.IsNull(ptr) ?NativeAllocationError.AllocationFailed : NativeAllocationError.Success;
    }
    public unsafe static NativeAllocationError AllocZeroed(usize size, usize align, out ValueMutPtr result) {
        if (size == 0)
        {
            var empty = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0, Alignment = align,
            }
            ;
            result = empty;
            return NativeAllocationError.Success;
        }
        if (ShouldFailAlloc())
        {
            var failed = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = size, Alignment = align,
            }
            ;
            result = failed;
            return NativeAllocationError.AllocationFailed;
        }
        var handle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = align,
        }
        ;
        if (align <= sizeof(void * )) {
            var * mut @expose_address byte ptr = ShouldFailSysAlloc() ?NativePtr.NullMut() : calloc(1, size);
            if (NativePtr.IsNull (ptr))
            {
                let fallback_align = align <= sizeof(void * ) ?sizeof(void * ) : align;
                let status = ShouldFailSysAlloc() ?1 : posix_memalign(& ptr, fallback_align, size);
                if (status == 0 && ! NativePtr.IsNull (ptr))
                {
                    memset(ptr, 0u8, size);
                }
            }
            handle.Pointer = ptr;
            handle.Size = size;
            handle.Alignment = align;
            result = handle;
            return NativePtr.IsNull(ptr) ?NativeAllocationError.AllocationFailed : NativeAllocationError.Success;
        }
        let status = Alloc(size, align, out handle);
        if (status != NativeAllocationError.Success)
        {
            result = handle;
            return status;
        }
        Set(handle, 0, size);
        result = handle;
        return NativeAllocationError.Success;
    }
    public unsafe static NativeAllocationError Realloc(ValueMutPtr pointer, usize oldSize, usize newSize, usize align, out ValueMutPtr result) {
        if (NativePtr.IsNull (pointer.Pointer))
        {
            var allocResult = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0, Alignment = align,
            }
            ;
            let status = Alloc(newSize, align, out allocResult);
            result = allocResult;
            return status;
        }
        if (ShouldFailRealloc())
        {
            result = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0, Alignment = align
            }
            ;
            return NativeAllocationError.AllocationFailed;
        }
        var * mut @expose_address byte updated = ShouldFailSysAlloc() ?NativePtr.NullMut() : realloc(pointer.Pointer, newSize);
        if (NativePtr.IsNull (updated))
        {
            updated = ShouldFailSysAlloc() ?NativePtr.NullMut() : malloc(newSize);
            if (NativePtr.IsNull (updated))
            {
                updated = ShouldFailSysAlloc() ?NativePtr.NullMut() : calloc(1usize, newSize);
            }
            if (NativePtr.IsNull (updated))
            {
                result = new ValueMutPtr {
                    Pointer = NativePtr.NullMut(), Size = 0, Alignment = align
                }
                ;
                return NativeAllocationError.AllocationFailed;
            }
            let bytes_to_copy = oldSize <newSize ?oldSize : newSize;
            if (bytes_to_copy >0)
            {
                memcpy(updated, pointer.Pointer, bytes_to_copy);
            }
        }
        var handle = new ValueMutPtr {
            Pointer = updated, Size = newSize, Alignment = align
        }
        ;
        result = handle;
        return NativeAllocationError.Success;
    }
    public unsafe static void Free(ValueMutPtr pointer) {
        if (! NativePtr.IsNull (pointer.Pointer))
        {
            free(pointer.Pointer);
        }
    }
    public unsafe static void Copy(ValueMutPtr dest, ValueConstPtr src, usize len) {
        if (len == 0)
        {
            return;
        }
        if (NativePtr.IsNull(dest.Pointer) || NativePtr.IsNullConst(src.Pointer))
        {
            return;
        }
        // Keep runtime-native memory helpers portable across backends (WASM has no libc).
        var i = 0usize;
        while (i <len)
        {
            let b = * NativePtr.OffsetConst(src.Pointer, (isize) i);
            * NativePtr.OffsetMut(dest.Pointer, (isize) i) = b;
            i += 1usize;
        }
    }
    public unsafe static void Move(ValueMutPtr dest, ValueConstPtr src, usize len) {
        if (len == 0)
        {
            return;
        }
        if (NativePtr.IsNull(dest.Pointer) || NativePtr.IsNullConst(src.Pointer))
        {
            return;
        }
        let destAddr = (nuint) dest.Pointer;
        let srcAddr = (nuint) src.Pointer;
        if (destAddr == srcAddr)
        {
            return;
        }
        if (destAddr <srcAddr || destAddr >= srcAddr + (nuint) len)
        {
            // Forward copy when no overlap or dest before src.
            var i = 0usize;
            while (i <len)
            {
                let b = * NativePtr.OffsetConst(src.Pointer, (isize) i);
                * NativePtr.OffsetMut(dest.Pointer, (isize) i) = b;
                i += 1usize;
            }
            return;
        }
        // Backward copy when regions overlap.
        var j = len;
        while (j >0usize)
        {
            j -= 1usize;
            let b = * NativePtr.OffsetConst(src.Pointer, (isize) j);
            * NativePtr.OffsetMut(dest.Pointer, (isize) j) = b;
        }
    }
    public unsafe static void Set(ValueMutPtr dest, byte value, usize len) {
        if (len == 0)
        {
            return;
        }
        if (NativePtr.IsNull(dest.Pointer))
        {
            return;
        }
        var i = 0usize;
        while (i <len)
        {
            * NativePtr.OffsetMut(dest.Pointer, (isize) i) = value;
            i += 1usize;
        }
    }
    public unsafe static void ZeroInitRaw(* mut @expose_address byte ptr, usize len) {
        if (len == 0 || NativePtr.IsNull (ptr))
        {
            return;
        }
        var i = 0usize;
        while (i <len)
        {
            * NativePtr.OffsetMut(ptr, (isize) i) = 0u8;
            i += 1usize;
        }
    }
}
