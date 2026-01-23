namespace Std.Compiler.Object;
import Std.Memory;
import Std.Runtime;
import Std.Runtime.Collections;
@repr(c) public struct GlueIndexEntry
{
    public u64 type_id;
    public uint function_index;
}
@repr(c) public struct TypeMetadataEntry
{
    public u64 type_id;
    public usize size;
    public usize align;
    public isize drop_fn;
    public Std.Runtime.VarianceSlice variance;
    public uint flags;
}
// Chic-native object metadata/glue surface intended to replace the Rust “object crate”.
public static class ObjectRuntime
{
    private static * const @readonly @expose_address TypeMetadataEntry _typeMetadataTable;
    private static usize _typeMetadataTableLen;
    private static * const @readonly @expose_address GlueIndexEntry _hashGlueTable;
    private static usize _hashGlueTableLen;
    private static * const @readonly @expose_address GlueIndexEntry _eqGlueTable;
    private static usize _eqGlueTableLen;
    private static bool _pendingExceptionSet;
    private static i64 _pendingExceptionPayload;
    private static i64 _pendingExceptionTypeId;
    private static unsafe * const @readonly @expose_address byte AsConstByte <T >(* const @readonly @expose_address T ptr) {
        return(* const @readonly @expose_address byte) ptr;
    }
    private static unsafe * const @readonly @expose_address byte OffsetConstLocal(* const @readonly @expose_address byte pointer,
    isize offset) {
        if (offset == 0 || pointer == null)
        {
            return pointer;
        }
        let base = (isize) pointer;
        return(* const @readonly @expose_address byte)(base + offset);
    }
    private static Std.Runtime.TypeMetadataRecord EmptyMetadata() {
        return new Std.Runtime.TypeMetadataRecord(0usize, 0usize, 0isize);
    }
    private static unsafe int TypeMetadataFill(u64 type_id, * mut @expose_address Std.Runtime.TypeMetadataRecord out_metadata) {
        if (out_metadata == null)
        {
            return 2;
            // InvalidPointer
        }
        * out_metadata = EmptyMetadata();
        if (_typeMetadataTable != null)
        {
            var i = 0usize;
            while (i <_typeMetadataTableLen)
            {
                let entryPtr = (* const @readonly @expose_address TypeMetadataEntry) OffsetConstLocal(AsConstByte(_typeMetadataTable),
                (isize)(i * sizeof(TypeMetadataEntry)));
                if ( (* entryPtr).type_id == type_id)
                {
                    (* out_metadata).Size = (* entryPtr).size;
                    (* out_metadata).Align = (* entryPtr).align;
                    (* out_metadata).DropFn = (* entryPtr).drop_fn;
                    (* out_metadata).Variance = (* entryPtr).variance;
                    (* out_metadata).Flags = (* entryPtr).flags;
                    return 0;
                }
                i += 1;
            }
        }
        return 1;
        // NotFound
    }
    private static unsafe isize LookupGlueHandle(* const @readonly @expose_address GlueIndexEntry table, usize len, u64 type_id) {
        if (table == null || len == 0)
        {
            return 0isize;
        }
        var i = 0usize;
        while (i <len)
        {
            let entryPtr = (* const @readonly @expose_address GlueIndexEntry) OffsetConstLocal(AsConstByte(table), (isize)(i * sizeof(GlueIndexEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                return(isize)(* entryPtr).function_index;
            }
            i += 1;
        }
        return 0isize;
    }
    public static u64 TypeIdOf <T >() {
        return __type_id_of <T >();
    }
    public static Std.Runtime.TypeMetadataRecord MetadataOf <T >() {
        let typeId = __type_id_of <T >();
        var meta = EmptyMetadata();
        unsafe {
            let _ = TypeMetadataFill(typeId, & meta);
        }
        return meta;
    }
    public static bool TryGetMetadata(u64 typeId, out Std.Runtime.TypeMetadataRecord metadata) {
        var meta = EmptyMetadata();
        var ok = false;
        unsafe {
            ok = TypeMetadataFill(typeId, & meta) == 0;
        }
        metadata = meta;
        return ok;
    }
    public static usize SizeOf(u64 typeId) {
        var meta = EmptyMetadata();
        var found = false;
        unsafe {
            found = TypeMetadataFill(typeId, & meta) == 0;
        }
        if (found)
        {
            return meta.Size;
        }
        return 0usize;
    }
    public static usize AlignOf(u64 typeId) {
        var meta = EmptyMetadata();
        var found = false;
        unsafe {
            found = TypeMetadataFill(typeId, & meta) == 0;
        }
        if (found)
        {
            return meta.Align;
        }
        return 0usize;
    }
    public static isize DropGlueOf(u64 typeId) {
        var meta = EmptyMetadata();
        var found = false;
        unsafe {
            found = TypeMetadataFill(typeId, & meta) == 0;
        }
        if (found)
        {
            return meta.DropFn;
        }
        return 0isize;
    }
    public static isize CloneGlueOf(u64 typeId) {
        (void) typeId;
        return 0isize;
    }
    public static isize HashGlueOf(u64 typeId) {
        var handle = 0isize;
        unsafe {
            handle = LookupGlueHandle(_hashGlueTable, _hashGlueTableLen, typeId);
        }
        return handle;
    }
    public static isize EqGlueOf(u64 typeId) {
        var handle = 0isize;
        unsafe {
            handle = LookupGlueHandle(_eqGlueTable, _eqGlueTableLen, typeId);
        }
        return handle;
    }
    // --- ABI exports (weak on native; used by WASM executor for delegation) ---
    @extern("C") @weak @export("chic_rt_install_type_metadata") public unsafe static void chic_rt_install_type_metadata(* const @readonly @expose_address TypeMetadataEntry entries,
    usize len) {
        _typeMetadataTable = entries;
        _typeMetadataTableLen = len;
    }
    @extern("C") @weak @export("chic_rt_type_metadata") public unsafe static int chic_rt_type_metadata(u64 type_id, * mut @expose_address Std.Runtime.TypeMetadataRecord out_metadata) {
        return TypeMetadataFill(type_id, out_metadata);
    }
    @extern("C") @weak @export("chic_rt_type_size") public unsafe static usize chic_rt_type_size(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) == 0)
        {
            return meta.Size;
        }
        return 0usize;
    }
    @extern("C") @weak @export("chic_rt_type_align") public unsafe static usize chic_rt_type_align(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) == 0)
        {
            return meta.Align;
        }
        return 0usize;
    }
    @extern("C") @weak @export("chic_rt_type_drop_glue") public unsafe static isize chic_rt_type_drop_glue(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) == 0)
        {
            return meta.DropFn;
        }
        return 0isize;
    }
    @extern("C") @weak @export("chic_rt_type_clone_glue") public unsafe static isize chic_rt_type_clone_glue(u64 type_id) {
        (void) type_id;
        return 0isize;
    }
    @extern("C") @weak @export("chic_rt_install_hash_table") public unsafe static void chic_rt_install_hash_table(* const @readonly @expose_address GlueIndexEntry entries,
    usize len) {
        _hashGlueTable = entries;
        _hashGlueTableLen = len;
    }
    @extern("C") @weak @export("chic_rt_install_eq_table") public unsafe static void chic_rt_install_eq_table(* const @readonly @expose_address GlueIndexEntry entries,
    usize len) {
        _eqGlueTable = entries;
        _eqGlueTableLen = len;
    }
    @extern("C") @weak @export("chic_rt_type_hash_glue") public unsafe static isize chic_rt_type_hash_glue(u64 type_id) {
        return LookupGlueHandle(_hashGlueTable, _hashGlueTableLen, type_id);
    }
    @extern("C") @weak @export("chic_rt_type_eq_glue") public unsafe static isize chic_rt_type_eq_glue(u64 type_id) {
        return LookupGlueHandle(_eqGlueTable, _eqGlueTableLen, type_id);
    }
    @extern("C") @weak @export("chic_rt_type_metadata_clear") public unsafe static void chic_rt_type_metadata_clear() {
        _typeMetadataTable = (* const @readonly @expose_address TypeMetadataEntry) null;
        _typeMetadataTableLen = 0;
        _hashGlueTable = (* const @readonly @expose_address GlueIndexEntry) null;
        _hashGlueTableLen = 0;
        _eqGlueTable = (* const @readonly @expose_address GlueIndexEntry) null;
        _eqGlueTableLen = 0;
    }
    @extern("C") @weak @export("chic_rt_object_new") public unsafe static * mut @expose_address byte chic_rt_object_new(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) != 0)
        {
            return(* mut @expose_address byte) null;
        }
        if (meta.Size == 0usize || meta.Align == 0usize)
        {
            return(* mut @expose_address byte) null;
        }
        var handle = new ValueMutPtr();
        if (GlobalAllocator.AllocZeroed (meta.Size, meta.Align, out handle) != AllocationError.Success) {
            return(* mut @expose_address byte) null;
        }
        return handle.Pointer;
    }
    @extern("C") @weak @export("chic_rt_closure_env_alloc") public unsafe static * mut @expose_address byte chic_rt_closure_env_alloc(usize size,
    usize align) {
        if (size == 0usize)
        {
            return(* mut @expose_address byte) null;
        }
        var handle = new ValueMutPtr();
        if (GlobalAllocator.Alloc (size, align == 0usize ?1usize : align, out handle) != AllocationError.Success) {
            return(* mut @expose_address byte) null;
        }
        return handle.Pointer;
    }
    @extern("C") @weak @export("chic_rt_closure_env_free") public unsafe static void chic_rt_closure_env_free(* mut @expose_address byte ptr,
    usize size, usize align) {
        if (ptr == null || size == 0usize)
        {
            return;
        }
        var handle = new ValueMutPtr {
            Pointer = ptr, Size = size, Alignment = align == 0usize ?1usize : align
        }
        ;
        GlobalAllocator.Free(handle);
    }
    @extern("C") @weak @export("chic_rt_closure_env_clone") public unsafe static * mut @expose_address byte chic_rt_closure_env_clone(* const @readonly @expose_address byte src,
    usize size, usize align) {
        if (src == null || size == 0usize)
        {
            return(* mut @expose_address byte) null;
        }
        let dest = chic_rt_closure_env_alloc(size, align);
        if (dest == null)
        {
            return(* mut @expose_address byte) null;
        }
        let dstPtr = new ValueMutPtr {
            Pointer = dest, Size = size, Alignment = align == 0usize ?1usize : align
        }
        ;
        let srcPtr = new ValueConstPtr {
            Pointer = src, Size = size, Alignment = align == 0usize ?1usize : align
        }
        ;
        GlobalAllocator.Copy(dstPtr, srcPtr, size);
        return dest;
    }
    @extern("C") @weak @export("chic_rt_throw") public unsafe static void chic_rt_throw(i64 payload, i64 type_id) {
        _pendingExceptionSet = true;
        _pendingExceptionPayload = payload;
        _pendingExceptionTypeId = type_id;
    }
    @extern("C") @weak @export("chic_rt_has_pending_exception") public static int chic_rt_has_pending_exception() {
        return _pendingExceptionSet ?1 : 0;
    }
    @extern("C") @weak @export("chic_rt_clear_pending_exception") public static void chic_rt_clear_pending_exception() {
        _pendingExceptionSet = false;
        _pendingExceptionPayload = 0i64;
        _pendingExceptionTypeId = 0i64;
    }
    @extern("C") @weak @export("chic_rt_peek_pending_exception") public unsafe static int chic_rt_peek_pending_exception(* mut i64 payload,
    * mut i64 type_id) {
        if (!_pendingExceptionSet)
        {
            return 0;
        }
        if (payload != null)
        {
            * payload = _pendingExceptionPayload;
        }
        if (type_id != null)
        {
            * type_id = _pendingExceptionTypeId;
        }
        return 1;
    }
    @extern("C") @weak @export("chic_rt_take_pending_exception") public unsafe static int chic_rt_take_pending_exception(* mut i64 payload,
    * mut i64 type_id) {
        let ok = chic_rt_peek_pending_exception(payload, type_id);
        if (ok != 0)
        {
            chic_rt_clear_pending_exception();
        }
        return ok;
    }
    @extern("C") private static extern void chic_rt_abort(int code);
    @extern("C") @weak @export("chic_rt_abort_unhandled_exception") public static void chic_rt_abort_unhandled_exception() {
        chic_rt_abort(123);
    }
}
