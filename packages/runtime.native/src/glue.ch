namespace Std.Runtime.Native;
// Chic-native replacements for the legacy C shim registries and closure helpers.
// These implement the frozen `chic_rt_*` ABI directly in Chic, avoiding
// the shim C file.
@repr(c) internal struct DropGlueEntry
{
    public u64 type_id;
    public fn @extern("C")(* mut @expose_address byte) -> void func;
}
@repr(c) internal struct HashGlueEntry
{
    public u64 type_id;
    public fn @extern("C")(* const @readonly @expose_address byte) -> u64 func;
}
@repr(c) internal struct EqGlueEntry
{
    public u64 type_id;
    public fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int func;
}
@repr(c) public struct VarianceSlice
{
    public * const @readonly @expose_address byte ptr;
    public usize len;
}
@repr(c) public struct TypeMetadataEntry
{
    public u64 type_id;
    public usize size;
    public usize align;
    public isize drop_fn;
    public VarianceSlice variance;
    public uint flags;
}
@repr(c) public struct RuntimeTypeMetadata
{
    public usize size;
    public usize align;
    public isize drop_fn;
    public VarianceSlice variance;
    public uint flags;
}
@repr(c) internal struct NativeTypeMetadataRecord
{
    public u64 type_id;
    public RuntimeTypeMetadata meta;
}
@repr(c) public struct InterfaceDefaultDescriptor
{
    public * const @readonly @expose_address byte implementer;
    public * const @readonly @expose_address byte interface_type;
    public * const @readonly @expose_address byte method;
    public * const @readonly @expose_address byte symbol;
}
public static class GlueRuntime
{
    @extern("C") private unsafe static u64 __hash_missing(* const @readonly @expose_address byte value) {
        (void) value;
        return 0u64;
    }
    @extern("C") private unsafe static int __eq_missing(* const @readonly @expose_address byte left, * const @readonly @expose_address byte right) {
        (void) left;
        (void) right;
        return 0;
    }
    // Drop glue ----------------------------------------------------------------
    private static * const @readonly @expose_address DropGlueEntry _dropTable;
    private static usize _dropTableLen;
    private static * mut @expose_address DropGlueEntry _dropRegistry;
    private static usize _dropRegistryLen;
    private static usize _dropRegistryCap;
    private static * const @readonly @expose_address HashGlueEntry _hashTable;
    private static usize _hashTableLen;
    private static * mut @expose_address HashGlueEntry _hashRegistry;
    private static usize _hashRegistryLen;
    private static usize _hashRegistryCap;
    private static * const @readonly @expose_address EqGlueEntry _eqTable;
    private static usize _eqTableLen;
    private static * mut @expose_address EqGlueEntry _eqRegistry;
    private static usize _eqRegistryLen;
    private static usize _eqRegistryCap;
    private static * const @readonly @expose_address TypeMetadataEntry _typeMetadataTable;
    private static usize _typeMetadataTableLen;
    private static * mut @expose_address NativeTypeMetadataRecord _typeMetadataRegistry;
    private static usize _typeMetadataRegistryLen;
    private static usize _typeMetadataRegistryCap;
    private static usize _typeMetadataRegistryBaselineLen;
    private static bool _typeMetadataRegistryBaselineInstalled = false;
    private static * const @readonly @expose_address InterfaceDefaultDescriptor _interfaceDefaults;
    private static u64 _interfaceDefaultsLen;
    private static unsafe * mut @expose_address byte AsByte <T >(* mut @expose_address T ptr) {
        return(* mut @expose_address byte) ptr;
    }
    private static unsafe * const @readonly @expose_address byte AsConstByte <T >(* const @readonly @expose_address T ptr) {
        return(* const @readonly @expose_address byte) ptr;
    }
    // Local pointer offset helpers to avoid relying on the legacy shim arithmetic.
    private static unsafe * mut @expose_address byte OffsetMutLocal(* mut @expose_address byte pointer, isize offset) {
        if (offset == 0 || pointer == null)
        {
            return pointer;
        }
        let base = (isize) pointer;
        return(* mut @expose_address byte)(base + offset);
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
    private static unsafe bool ReserveDropRegistry(usize needed) {
        if (needed <= _dropRegistryCap)
        {
            return true;
        }
        var cap = _dropRegistryCap == 0 ?8usize : _dropRegistryCap * 2usize;
        while (cap <needed)
        {
            cap *= 2;
        }
        let newSize = cap * sizeof(DropGlueEntry);
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newSize, Alignment = sizeof(usize)
        }
        ;
        if (_dropRegistry == null)
        {
            if (NativeAlloc.Alloc (newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        else
        {
            var current = new ValueMutPtr {
                Pointer = AsByte(_dropRegistry), Size = _dropRegistryCap * sizeof(DropGlueEntry), Alignment = sizeof(usize),
            }
            ;
            if (NativeAlloc.Realloc (current, current.Size, newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        _dropRegistry = (* mut @expose_address DropGlueEntry) result.Pointer;
        _dropRegistryCap = cap;
        return true;
    }
    private static unsafe bool ReserveHashRegistry(usize needed) {
        if (needed <= _hashRegistryCap)
        {
            return true;
        }
        var cap = _hashRegistryCap == 0 ?8usize : _hashRegistryCap * 2usize;
        while (cap <needed)
        {
            cap *= 2;
        }
        let newSize = cap * sizeof(HashGlueEntry);
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newSize, Alignment = sizeof(usize)
        }
        ;
        if (_hashRegistry == null)
        {
            if (NativeAlloc.Alloc (newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        else
        {
            var current = new ValueMutPtr {
                Pointer = AsByte(_hashRegistry), Size = _hashRegistryCap * sizeof(HashGlueEntry), Alignment = sizeof(usize),
            }
            ;
            if (NativeAlloc.Realloc (current, current.Size, newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        _hashRegistry = (* mut @expose_address HashGlueEntry) result.Pointer;
        _hashRegistryCap = cap;
        return true;
    }
    private static unsafe bool ReserveEqRegistry(usize needed) {
        if (needed <= _eqRegistryCap)
        {
            return true;
        }
        var cap = _eqRegistryCap == 0 ?8usize : _eqRegistryCap * 2usize;
        while (cap <needed)
        {
            cap *= 2;
        }
        let newSize = cap * sizeof(EqGlueEntry);
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newSize, Alignment = sizeof(usize)
        }
        ;
        if (_eqRegistry == null)
        {
            if (NativeAlloc.Alloc (newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        else
        {
            var current = new ValueMutPtr {
                Pointer = AsByte(_eqRegistry), Size = _eqRegistryCap * sizeof(EqGlueEntry), Alignment = sizeof(usize),
            }
            ;
            if (NativeAlloc.Realloc (current, current.Size, newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        _eqRegistry = (* mut @expose_address EqGlueEntry) result.Pointer;
        _eqRegistryCap = cap;
        return true;
    }
    private static unsafe bool ReserveTypeMetadataRegistry(usize needed) {
        if (needed <= _typeMetadataRegistryCap)
        {
            return true;
        }
        var cap = _typeMetadataRegistryCap == 0 ?8usize : _typeMetadataRegistryCap * 2usize;
        while (cap <needed)
        {
            cap *= 2;
        }
        let newSize = cap * sizeof(NativeTypeMetadataRecord);
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newSize, Alignment = sizeof(usize)
        }
        ;
        if (_typeMetadataRegistry == null)
        {
            if (NativeAlloc.Alloc (newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        else
        {
            var current = new ValueMutPtr {
                Pointer = AsByte(_typeMetadataRegistry), Size = _typeMetadataRegistryCap * sizeof(NativeTypeMetadataRecord), Alignment = sizeof(usize),
            }
            ;
            if (NativeAlloc.Realloc (current, current.Size, newSize, sizeof(usize), out result) != NativeAllocationError.Success) {
                return false;
            }
        }
        _typeMetadataRegistry = (* mut @expose_address NativeTypeMetadataRecord) result.Pointer;
        _typeMetadataRegistryCap = cap;
        return true;
    }
    @extern("C") private unsafe static void __drop_noop(* mut @expose_address byte _ptr) {
    }
    @extern("C") @export("__drop_noop") public unsafe static void __drop_noop_export(* mut @expose_address byte ptr) {
        __drop_noop(ptr);
    }
    @extern("C") @export("chic_rt_drop_noop_ptr") public unsafe static fn @extern("C")(* mut @expose_address byte) -> void chic_rt_drop_noop_ptr() {
        return __drop_noop;
    }
    @extern("C") @export("chic_rt_install_drop_table") public unsafe static void chic_rt_install_drop_table(* const @readonly @expose_address DropGlueEntry entries,
    usize len) {
        if (entries == null || len == 0usize)
        {
            return;
        }
        var i = 0usize;
        while (i <len)
        {
            let entryPtr = (* const @readonly @expose_address DropGlueEntry) OffsetConstLocal(AsConstByte(entries), (isize)(i * sizeof(DropGlueEntry)));
            chic_rt_drop_register((* entryPtr).type_id, (* entryPtr).func);
            i += 1usize;
        }
    }
    @extern("C") @export("chic_rt_drop_register") public unsafe static void chic_rt_drop_register(u64 type_id, fn @extern("C")(* mut @expose_address byte) -> void func) {
        var i = 0usize;
        while (i <_dropRegistryLen)
        {
            var entryPtr = (* mut @expose_address DropGlueEntry) OffsetMutLocal(AsByte(_dropRegistry), (isize)(i * sizeof(DropGlueEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                (* entryPtr).func = func;
                return;
            }
            i += 1;
        }
        let needed = _dropRegistryLen + 1;
        if (!ReserveDropRegistry (needed))
        {
            return;
        }
        var slot = (* mut @expose_address DropGlueEntry) OffsetMutLocal(AsByte(_dropRegistry), (isize)(_dropRegistryLen * sizeof(DropGlueEntry)));
        (* slot).type_id = type_id;
        (* slot).func = func;
        _dropRegistryLen = needed;
    }
    @extern("C") @export("chic_rt_drop_clear") public unsafe static void chic_rt_drop_clear() {
        _dropTable = (* const @readonly @expose_address DropGlueEntry) NativePtr.NullConst();
        _dropTableLen = 0;
        _dropRegistryLen = 0;
    }
    @extern("C") @export("chic_rt_drop_resolve") public unsafe static fn @extern("C")(* mut @expose_address byte) -> void chic_rt_drop_resolve(u64 type_id) {
        var i = 0usize;
        while (i <_dropRegistryLen)
        {
            let entryPtr = (* mut @expose_address DropGlueEntry) OffsetMutLocal(AsByte(_dropRegistry), (isize)(i * sizeof(DropGlueEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                return(* entryPtr).func;
            }
            i += 1;
        }
        if (_dropTable != null && _dropTableLen >0)
        {
            var baseIndex = 0usize;
            while (baseIndex <_dropTableLen)
            {
                let entryPtr = (* const @readonly @expose_address DropGlueEntry) OffsetConstLocal(AsConstByte(_dropTable),
                (isize)(baseIndex * sizeof(DropGlueEntry)));
                if ( (* entryPtr).type_id == type_id)
                {
                    return(* entryPtr).func;
                }
                baseIndex += 1;
            }
        }
        return SharedRuntime.chic_rt_drop_missing;
    }
    @extern("C") @export("chic_rt_drop_invoke") public unsafe static void chic_rt_drop_invoke(fn @extern("C")(* mut @expose_address byte) -> void func,
    * mut @expose_address byte value) {
        if (func == null || value == null)
        {
            return;
        }
        unsafe {
            func(value);
        }
    }
    // Hash glue ---------------------------------------------------------------
    @extern("C") @export("chic_rt_install_hash_table") public unsafe static void chic_rt_install_hash_table(* const @readonly @expose_address HashGlueEntry entries,
    usize len) {
        if (entries == null || len == 0usize)
        {
            return;
        }
        var i = 0usize;
        while (i <len)
        {
            let entryPtr = (* const @readonly @expose_address HashGlueEntry) OffsetConstLocal(AsConstByte(entries), (isize)(i * sizeof(HashGlueEntry)));
            chic_rt_hash_register((* entryPtr).type_id, (* entryPtr).func);
            i += 1usize;
        }
    }
    @extern("C") @export("chic_rt_hash_register") public unsafe static void chic_rt_hash_register(u64 type_id, fn @extern("C")(* const @readonly @expose_address byte) -> u64 func) {
        var i = 0usize;
        while (i <_hashRegistryLen)
        {
            var entryPtr = (* mut @expose_address HashGlueEntry) OffsetMutLocal(AsByte(_hashRegistry), (isize)(i * sizeof(HashGlueEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                (* entryPtr).func = func;
                return;
            }
            i += 1;
        }
        let needed = _hashRegistryLen + 1;
        if (!ReserveHashRegistry (needed))
        {
            return;
        }
        var slot = (* mut @expose_address HashGlueEntry) OffsetMutLocal(AsByte(_hashRegistry), (isize)(_hashRegistryLen * sizeof(HashGlueEntry)));
        (* slot).type_id = type_id;
        (* slot).func = func;
        _hashRegistryLen = needed;
    }
    @extern("C") @export("chic_rt_hash_clear") public unsafe static void chic_rt_hash_clear() {
        _hashTable = (* const @readonly @expose_address HashGlueEntry) NativePtr.NullConst();
        _hashTableLen = 0;
        _hashRegistryLen = 0;
    }
    @extern("C") @export("chic_rt_hash_resolve") public unsafe static fn @extern("C")(* const @readonly @expose_address byte) -> u64 chic_rt_hash_resolve(u64 type_id) {
        var i = 0usize;
        while (i <_hashRegistryLen)
        {
            let entryPtr = (* mut @expose_address HashGlueEntry) OffsetMutLocal(AsByte(_hashRegistry), (isize)(i * sizeof(HashGlueEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                return(* entryPtr).func;
            }
            i += 1;
        }
        if (_hashTable != null && _hashTableLen >0)
        {
            var baseIndex = 0usize;
            while (baseIndex <_hashTableLen)
            {
                let entryPtr = (* const @readonly @expose_address HashGlueEntry) OffsetConstLocal(AsConstByte(_hashTable),
                (isize)(baseIndex * sizeof(HashGlueEntry)));
                if ( (* entryPtr).type_id == type_id)
                {
                    return(* entryPtr).func;
                }
                baseIndex += 1;
            }
        }
        return __hash_missing;
    }
    @extern("C") @export("chic_rt_hash_invoke") public unsafe static u64 chic_rt_hash_invoke(fn @extern("C")(* const @readonly @expose_address byte) -> u64 func,
    * const @readonly @expose_address byte value) {
        if (func == null)
        {
            return 0;
        }
        return func(value);
    }
    // Eq glue -----------------------------------------------------------------
    @extern("C") @export("chic_rt_install_eq_table") public unsafe static void chic_rt_install_eq_table(* const @readonly @expose_address EqGlueEntry entries,
    usize len) {
        if (entries == null || len == 0usize)
        {
            return;
        }
        var i = 0usize;
        while (i <len)
        {
            let entryPtr = (* const @readonly @expose_address EqGlueEntry) OffsetConstLocal(AsConstByte(entries), (isize)(i * sizeof(EqGlueEntry)));
            chic_rt_eq_register((* entryPtr).type_id, (* entryPtr).func);
            i += 1usize;
        }
    }
    @extern("C") @export("chic_rt_eq_register") public unsafe static void chic_rt_eq_register(u64 type_id, fn @extern("C")(* const @readonly @expose_address byte,
    * const @readonly @expose_address byte) -> int func) {
        var i = 0usize;
        while (i <_eqRegistryLen)
        {
            var entryPtr = (* mut @expose_address EqGlueEntry) OffsetMutLocal(AsByte(_eqRegistry), (isize)(i * sizeof(EqGlueEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                (* entryPtr).func = func;
                return;
            }
            i += 1;
        }
        let needed = _eqRegistryLen + 1;
        if (!ReserveEqRegistry (needed))
        {
            return;
        }
        var slot = (* mut @expose_address EqGlueEntry) OffsetMutLocal(AsByte(_eqRegistry), (isize)(_eqRegistryLen * sizeof(EqGlueEntry)));
        (* slot).type_id = type_id;
        (* slot).func = func;
        _eqRegistryLen = needed;
    }
    @extern("C") @export("chic_rt_eq_clear") public unsafe static void chic_rt_eq_clear() {
        _eqTable = (* const @readonly @expose_address EqGlueEntry) NativePtr.NullConst();
        _eqTableLen = 0;
        _eqRegistryLen = 0;
    }
    @extern("C") @export("chic_rt_eq_resolve") public unsafe static fn @extern("C")(* const @readonly @expose_address byte,
    * const @readonly @expose_address byte) -> int chic_rt_eq_resolve(u64 type_id) {
        var i = 0usize;
        while (i <_eqRegistryLen)
        {
            let entryPtr = (* mut @expose_address EqGlueEntry) OffsetMutLocal(AsByte(_eqRegistry), (isize)(i * sizeof(EqGlueEntry)));
            if ( (* entryPtr).type_id == type_id)
            {
                return(* entryPtr).func;
            }
            i += 1;
        }
        if (_eqTable != null && _eqTableLen >0)
        {
            var baseIndex = 0usize;
            while (baseIndex <_eqTableLen)
            {
                let entryPtr = (* const @readonly @expose_address EqGlueEntry) OffsetConstLocal(AsConstByte(_eqTable), (isize)(baseIndex * sizeof(EqGlueEntry)));
                if ( (* entryPtr).type_id == type_id)
                {
                    return(* entryPtr).func;
                }
                baseIndex += 1;
            }
        }
        return __eq_missing;
    }
    @extern("C") @export("chic_rt_eq_invoke") public unsafe static int chic_rt_eq_invoke(fn @extern("C")(* const @readonly @expose_address byte,
    * const @readonly @expose_address byte) -> int func, * const @readonly @expose_address byte left, * const @readonly @expose_address byte right) {
        if (func == null || left == null || right == null)
        {
            return 0;
        }
        return func(left, right);
    }
    // Type metadata -----------------------------------------------------------
    private static unsafe RuntimeTypeMetadata RuntimeTypeMetadataFromEntry(* const @readonly @expose_address TypeMetadataEntry entry) {
        if (entry == null)
        {
            return EmptyMetadata();
        }
        return new RuntimeTypeMetadata {
            size = (* entry).size, align = (* entry).align, drop_fn = (* entry).drop_fn, variance = (* entry).variance, flags = (* entry).flags,
        }
        ;
    }
    private static RuntimeTypeMetadata EmptyMetadata() {
        return new RuntimeTypeMetadata {
            size = 0usize, align = 0usize, drop_fn = 0isize, variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 0u,
        }
        ;
    }
    private static unsafe * mut NativeTypeMetadataRecord LookupTypeMetadata(u64 type_id) {
        var i = 0usize;
        while (i <_typeMetadataRegistryLen)
        {
            var entryPtr = (* mut @expose_address NativeTypeMetadataRecord) OffsetMutLocal(AsByte(_typeMetadataRegistry),
            (isize)(i * sizeof(NativeTypeMetadataRecord)));
            if ( (* entryPtr).type_id == type_id)
            {
                return entryPtr;
            }
            i += 1;
        }
        return(* mut @expose_address NativeTypeMetadataRecord) NativePtr.NullMut();
    }
    private static unsafe int TypeMetadataFill(u64 type_id, * mut RuntimeTypeMetadata out_metadata) {
        var outPtr = out_metadata;
        if (outPtr == null)
        {
            return 2;
            // InvalidPointer
        }
        * outPtr = EmptyMetadata();
        let registered = LookupTypeMetadata(type_id);
        if (registered != null)
        {
            * outPtr = (* registered).meta;
            return 0;
        }
        if (_typeMetadataTable != null)
        {
            var i = 0usize;
            while (i <_typeMetadataTableLen)
            {
                let entryPtr = (* const @readonly @expose_address TypeMetadataEntry) OffsetConstLocal(AsConstByte(_typeMetadataTable),
                (isize)(i * sizeof(TypeMetadataEntry)));
                if ( (* entryPtr).type_id == type_id)
                {
                    * outPtr = RuntimeTypeMetadataFromEntry(entryPtr);
                    return 0;
                }
                i += 1;
            }
        }
        return 1;
        // NotFound
    }
    @extern("C") @export("chic_rt_install_type_metadata") public unsafe static void chic_rt_install_type_metadata(* const @readonly @expose_address TypeMetadataEntry entries,
    usize len) {
        if (entries == null || len == 0usize)
        {
            return;
        }
        var i = 0usize;
        while (i <len)
        {
            let entryPtr = (* const @readonly @expose_address TypeMetadataEntry) OffsetConstLocal(AsConstByte(entries), (isize)(i * sizeof(TypeMetadataEntry)));
            chic_rt_type_metadata_register((* entryPtr).type_id, RuntimeTypeMetadataFromEntry(entryPtr));
            i += 1usize;
        }
        if (!_typeMetadataRegistryBaselineInstalled)
        {
            _typeMetadataRegistryBaselineInstalled = true;
            _typeMetadataRegistryBaselineLen = _typeMetadataRegistryLen;
        }
    }
    @extern("C") @export("chic_rt_type_size") public unsafe static usize chic_rt_type_size(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) == 0)
        {
            return meta.size;
        }
        return 0;
    }
    @extern("C") @export("chic_rt_type_align") public unsafe static usize chic_rt_type_align(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) == 0)
        {
            return meta.align;
        }
        return 0;
    }
    @extern("C") @export("chic_rt_type_drop_glue") public unsafe static isize chic_rt_type_drop_glue(u64 type_id) {
        var meta = EmptyMetadata();
        if (TypeMetadataFill (type_id, & meta) == 0 && meta.drop_fn != 0isize)
        {
            return meta.drop_fn;
        }
        return 0;
    }
    @extern("C") @export("chic_rt_type_clone_glue") public unsafe static isize chic_rt_type_clone_glue(u64 type_id) {
        (void) type_id;
        return 0;
    }
    @extern("C") @export("chic_rt_type_hash_glue") public unsafe static isize chic_rt_type_hash_glue(u64 type_id) {
        let func = chic_rt_hash_resolve(type_id);
        if (func == null || func == __hash_missing)
        {
            return 0isize;
        }
        let ptr = (* const @readonly @expose_address byte) func;
        return(isize) ptr;
    }
    @extern("C") @export("chic_rt_type_eq_glue") public unsafe static isize chic_rt_type_eq_glue(u64 type_id) {
        let func = chic_rt_eq_resolve(type_id);
        if (func == null || func == __eq_missing)
        {
            return 0isize;
        }
        let ptr = (* const @readonly @expose_address byte) func;
        return(isize) ptr;
    }
    @extern("C") @export("chic_rt_type_metadata") public unsafe static int chic_rt_type_metadata(u64 type_id, * mut RuntimeTypeMetadata out_metadata) {
        return TypeMetadataFill(type_id, out_metadata);
    }
    @extern("C") @export("chic_rt_type_metadata_register") public unsafe static void chic_rt_type_metadata_register(u64 type_id,
    RuntimeTypeMetadata metadata) {
        var existing = LookupTypeMetadata(type_id);
        if (existing != null)
        {
            (* existing).meta = metadata;
            return;
        }
        let nextLen = _typeMetadataRegistryLen + 1;
        if (!ReserveTypeMetadataRegistry (nextLen))
        {
            return;
        }
        var slot = (* mut @expose_address NativeTypeMetadataRecord) OffsetMutLocal(AsByte(_typeMetadataRegistry), (isize)(_typeMetadataRegistryLen * sizeof(NativeTypeMetadataRecord)));
        (* slot).type_id = type_id;
        (* slot).meta = metadata;
        _typeMetadataRegistryLen = nextLen;
    }
    @extern("C") @export("chic_rt_type_metadata_clear") public unsafe static void chic_rt_type_metadata_clear() {
        _typeMetadataRegistryLen = _typeMetadataRegistryBaselineLen;
    }
    // Interface defaults ------------------------------------------------------
    @extern("C") @export("chic_rt_install_interface_defaults") public unsafe static void chic_rt_install_interface_defaults(* const @readonly @expose_address InterfaceDefaultDescriptor entries,
    u64 len) {
        if (entries == null || len == 0)
        {
            _interfaceDefaults = (* const @readonly @expose_address InterfaceDefaultDescriptor) NativePtr.NullConst();
            _interfaceDefaultsLen = 0;
            return;
        }
        _interfaceDefaults = entries;
        _interfaceDefaultsLen = len;
    }
    @extern("C") @export("chic_rt_interface_defaults_ptr") public unsafe static * const @readonly @expose_address InterfaceDefaultDescriptor chic_rt_interface_defaults_ptr() {
        return _interfaceDefaults;
    }
    @extern("C") @export("chic_rt_interface_defaults_len") public unsafe static u64 chic_rt_interface_defaults_len() {
        return _interfaceDefaultsLen;
    }
    // Closure env helpers -----------------------------------------------------
    @extern("C") @export("chic_rt_closure_env_free") public unsafe static void chic_rt_closure_env_free(* mut @expose_address byte ptr,
    u64 size, u64 align) {
        (void) align;
        if (ptr == null || size == 0)
        {
            return;
        }
        let handle = new ValueMutPtr {
            Pointer = ptr, Size = (usize) size, Alignment = (usize) align
        }
        ;
        NativeAlloc.Free(handle);
    }
    @extern("C") @export("chic_rt_closure_env_alloc") public unsafe static * mut @expose_address byte chic_rt_closure_env_alloc(u64 size,
    u64 align) {
        if (size == 0)
        {
            return NativePtr.NullMut();
        }
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = (usize) size, Alignment = (usize) align
        }
        ;
        if (NativeAlloc.Alloc ( (usize) size, (usize) align, out result) != NativeAllocationError.Success) {
            return NativePtr.NullMut();
        }
        return result.Pointer;
    }
    @extern("C") @export("chic_rt_closure_env_clone") public unsafe static * mut @expose_address byte chic_rt_closure_env_clone(* const @readonly @expose_address byte src,
    u64 size, u64 align) {
        if (src == null || size == 0)
        {
            return NativePtr.NullMut();
        }
        let dest = chic_rt_closure_env_alloc(size, align);
        if (dest == null)
        {
            return NativePtr.NullMut();
        }
        let dstPtr = new ValueMutPtr {
            Pointer = dest, Size = (usize) size, Alignment = (usize) align
        }
        ;
        let srcPtr = new ValueConstPtr {
            Pointer = src, Size = (usize) size, Alignment = (usize) align
        }
        ;
        NativeAlloc.Copy(dstPtr, srcPtr, (usize) size);
        return dest;
    }
    @extern("C") @export("chic_rt_clone_invoke") public unsafe static void chic_rt_clone_invoke(isize glue, ValueConstPtr src,
    ValueMutPtr dest) {
        if (glue == 0)
        {
            return;
        }
        let srcPtr = src.Size == 0 ?NativePtr.NullConst() : src.Pointer;
        let destPtr = dest.Size == 0 ?NativePtr.NullMut() : dest.Pointer;
    }
    // FFI resolver placeholders ----------------------------------------------
    @extern("C") @export("chic_rt_ffi_resolve") public unsafe static * mut @expose_address byte chic_rt_ffi_resolve(* const @readonly @expose_address byte _descriptor) {
        (void) _descriptor;
        return NativePtr.NullMut();
    }
    @extern("C") @export("chic_rt_ffi_eager_resolve") public unsafe static * mut @expose_address byte chic_rt_ffi_eager_resolve(* const @readonly @expose_address byte descriptor) {
        return chic_rt_ffi_resolve(descriptor);
    }
    @extern("C") @export("chic_rt_ffi_add_search_path") public unsafe static void chic_rt_ffi_add_search_path(* const @readonly @expose_address byte path) {
        (void) path;
    }
    @extern("C") @export("chic_rt_ffi_set_default_pattern") public unsafe static void chic_rt_ffi_set_default_pattern(* const @readonly @expose_address byte pattern) {
        (void) pattern;
    }
}
