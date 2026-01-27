namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

public static class GlueTestSupport
{
    @extern("C") public unsafe static void DropMarker(* mut @expose_address byte ptr) {
        if (ptr != null)
        {
            * ptr = 9u8;
        }
    }
    @extern("C") public unsafe static u64 HashMarker(* const @readonly @expose_address byte _ptr) {
        return 123u64;
    }
    @extern("C") public unsafe static u64 HashAlt(* const @readonly @expose_address byte _ptr) {
        return 7u64;
    }
    @extern("C") public unsafe static int EqPointer(* const @readonly @expose_address byte left,
    * const @readonly @expose_address byte right) {
        let leftAddr = NativePtr.ToIsizeConst(left);
        let rightAddr = NativePtr.ToIsizeConst(right);
        return leftAddr == rightAddr ?1 : 0;
    }
}

private unsafe static bool BytesEqualValue(ValueConstPtr left, ValueConstPtr right, usize len) {
    var idx = 0usize;
    while (idx < len)
    {
        let leftPtr = NativePtr.OffsetConst(left.Pointer, (isize) idx);
        let rightPtr = NativePtr.OffsetConst(right.Pointer, (isize) idx);
        let leftValue = NativePtr.ReadByteConst(leftPtr);
        let rightValue = NativePtr.ReadByteConst(rightPtr);
        if (leftValue != rightValue)
        {
            return false;
        }
        idx += 1usize;
    }
    return true;
}

testcase Given_glue_drop_registry_register_When_executed_Then_glue_drop_registry_register()
{
    unsafe {
        GlueRuntime.chic_rt_drop_clear();
        GlueRuntime.chic_rt_drop_register(10u64, GlueRuntime.chic_rt_drop_noop_ptr());
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out buffer);
        var ok = status == NativeAllocationError.Success;
        let func = GlueRuntime.chic_rt_drop_resolve(10u64);
        GlueRuntime.chic_rt_drop_invoke(func, buffer.Pointer);
        let value = NativePtr.ReadByteMut(buffer.Pointer);
        ok = ok && value == 0u8;
        NativeAlloc.Free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_drop_registry_update_When_executed_Then_glue_drop_registry_update()
{
    unsafe {
        GlueRuntime.chic_rt_drop_clear();
        GlueRuntime.chic_rt_drop_register(10u64, GlueRuntime.chic_rt_drop_noop_ptr());
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out buffer);
        var ok = status == NativeAllocationError.Success;
        * buffer.Pointer = 5u8;
        GlueRuntime.chic_rt_drop_register(10u64, GlueRuntime.chic_rt_drop_noop_ptr());
        let updated = GlueRuntime.chic_rt_drop_resolve(10u64);
        GlueRuntime.chic_rt_drop_invoke(updated, buffer.Pointer);
        let value2 = NativePtr.ReadByteMut(buffer.Pointer);
        ok = ok && value2 == 5u8;
        NativeAlloc.Free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_drop_table_lookup_When_executed_Then_glue_drop_table_lookup()
{
    unsafe {
        GlueRuntime.chic_rt_drop_clear();
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out buffer);
        var ok = status == NativeAllocationError.Success;
        * buffer.Pointer = 0u8;
        var tableEntry = new DropGlueEntry {
            type_id = 20u64, func = GlueTestSupport.DropMarker
        }
        ;
        GlueRuntime.chic_rt_install_drop_table(& tableEntry, 1usize);
        let tableFunc = GlueRuntime.chic_rt_drop_resolve(20u64);
        GlueRuntime.chic_rt_drop_invoke(tableFunc, buffer.Pointer);
        let value3 = NativePtr.ReadByteMut(buffer.Pointer);
        ok = ok && value3 == 9u8;
        NativeAlloc.Free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_hash_and_eq_registry_paths_When_executed_Then_glue_hash_and_eq_registry_paths()
{
    unsafe {
        GlueRuntime.chic_rt_hash_clear();
        GlueRuntime.chic_rt_hash_register(33u64, GlueTestSupport.HashMarker);
        let hashFn = GlueRuntime.chic_rt_hash_resolve(33u64);
        let hash = GlueRuntime.chic_rt_hash_invoke(hashFn, NativePtr.NullConst());
        var ok = hash == 123u64;

        GlueRuntime.chic_rt_hash_register(33u64, GlueTestSupport.HashAlt);
        let hashFn2 = GlueRuntime.chic_rt_hash_resolve(33u64);
        let hash2 = GlueRuntime.chic_rt_hash_invoke(hashFn2, NativePtr.NullConst());
        ok = ok && hash2 == 7u64;

        var tableEntry = new HashGlueEntry {
            type_id = 44u64, func = GlueTestSupport.HashMarker
        }
        ;
        GlueRuntime.chic_rt_install_hash_table(& tableEntry, 1usize);
        let tableFn = GlueRuntime.chic_rt_hash_resolve(44u64);
        let tableHash = GlueRuntime.chic_rt_hash_invoke(tableFn, NativePtr.NullConst());
        ok = ok && tableHash == 123u64;
        let missingFn = GlueRuntime.chic_rt_hash_resolve(999u64);
        let missingHash = GlueRuntime.chic_rt_hash_invoke(missingFn, NativePtr.NullConst());
        ok = ok && missingHash == 0u64;

        GlueRuntime.chic_rt_eq_clear();
        GlueRuntime.chic_rt_eq_register(55u64, GlueTestSupport.EqPointer);
        let eqFn = GlueRuntime.chic_rt_eq_resolve(55u64);
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out buffer);
        ok = ok && status == NativeAllocationError.Success;
        let eq = GlueRuntime.chic_rt_eq_invoke(eqFn, buffer.Pointer, buffer.Pointer);
        ok = ok && eq == 1;
        let eq2 = GlueRuntime.chic_rt_eq_invoke(eqFn, buffer.Pointer, NativePtr.NullConst());
        ok = ok && eq2 == 0;

        var eqEntry = new EqGlueEntry {
            type_id = 66u64, func = GlueTestSupport.EqPointer
        }
        ;
        GlueRuntime.chic_rt_install_eq_table(& eqEntry, 1usize);
        let eqTableFn = GlueRuntime.chic_rt_eq_resolve(66u64);
        let eqTable = GlueRuntime.chic_rt_eq_invoke(eqTableFn, buffer.Pointer, buffer.Pointer);
        ok = ok && eqTable == 1;
        let missingEqFn = GlueRuntime.chic_rt_eq_resolve(777u64);
        let missingEq = GlueRuntime.chic_rt_eq_invoke(missingEqFn, buffer.Pointer, buffer.Pointer);
        ok = ok && missingEq == 0;

        NativeAlloc.Free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_type_metadata_registry_and_table_When_executed_Then_glue_type_metadata_registry_and_table()
{
    unsafe {
        GlueRuntime.chic_rt_type_metadata_clear();
        let meta = new RuntimeTypeMetadata {
            size = 16usize, align = 8usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 3u
        }
        ;
        GlueRuntime.chic_rt_type_metadata_register(77u64, meta);
        var ok = GlueRuntime.chic_rt_type_size(77u64) == 16usize;
        ok = ok && GlueRuntime.chic_rt_type_align(77u64) == 8usize;
        ok = ok && GlueRuntime.chic_rt_type_drop_glue(77u64) == 0isize;

        var outMeta = new RuntimeTypeMetadata {
            size = 0usize, align = 0usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 0u
        }
        ;
        let rc = GlueRuntime.chic_rt_type_metadata(77u64, & outMeta);
        ok = ok && rc == 0;
        ok = ok && outMeta.size == 16usize;

        GlueRuntime.chic_rt_type_metadata_clear();
        let missing = GlueRuntime.chic_rt_type_metadata(77u64, & outMeta);
        ok = ok && missing == 1;

        var tableEntry = new TypeMetadataEntry {
            type_id = 88u64, size = 4usize, align = 4usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 1u
        }
        ;
        GlueRuntime.chic_rt_install_type_metadata(& tableEntry, 1usize);
        ok = ok && GlueRuntime.chic_rt_type_size(88u64) == 4usize;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_interface_defaults_and_closure_env_When_executed_Then_glue_interface_defaults_and_closure_env()
{
    unsafe {
        GlueRuntime.chic_rt_install_interface_defaults((* const @readonly @expose_address InterfaceDefaultDescriptor) NativePtr.NullConst(), 0u64);
        var ok = GlueRuntime.chic_rt_interface_defaults_len() == 0u64;

        var iface = new InterfaceDefaultDescriptor {
            implementer = NativePtr.NullConst(), interface_type = NativePtr.NullConst(), method = NativePtr.NullConst(), symbol = NativePtr.NullConst()
        }
        ;
        GlueRuntime.chic_rt_install_interface_defaults(& iface, 1u64);
        ok = ok && GlueRuntime.chic_rt_interface_defaults_len() == 1u64;

        let empty = GlueRuntime.chic_rt_closure_env_alloc(0u64, 1u64);
        ok = ok && empty == null;
        let env = GlueRuntime.chic_rt_closure_env_alloc(4u64, 1u64);
        ok = ok && env != null;
        if (env != null)
        {
            * env = 1u8;
            * NativePtr.OffsetMut(env, 1isize) = 2u8;
            * NativePtr.OffsetMut(env, 2isize) = 3u8;
            * NativePtr.OffsetMut(env, 3isize) = 4u8;
        }
        let clone = GlueRuntime.chic_rt_closure_env_clone(NativePtr.AsConstPtr(env), 4u64, 1u64);
        ok = ok && clone != null;
        let left = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(env), Size = 4usize, Alignment = 1usize
        }
        ;
        let right = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(clone), Size = 4usize, Alignment = 1usize
        }
        ;
        if (env != null && clone != null)
        {
            ok = ok && BytesEqualValue(left, right, 4usize);
        }
        else
        {
            ok = false;
        }
        GlueRuntime.chic_rt_closure_env_free(env, 4u64, 1u64);
        GlueRuntime.chic_rt_closure_env_free(clone, 4u64, 1u64);

        let src = new ValueConstPtr {
            Pointer = NativePtr.NullConst(), Size = 0usize, Alignment = 1usize
        }
        ;
        let dest = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        GlueRuntime.chic_rt_clone_invoke(0isize, src, dest);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_type_metadata_accessors_and_ffi_stubs_When_executed_Then_glue_type_metadata_accessors_and_ffi_stubs()
{
    unsafe {
        GlueRuntime.chic_rt_type_metadata_clear();
        var entry = new TypeMetadataEntry {
            type_id = 101u64, size = 12usize, align = 4usize, drop_fn = 99isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 5u
        }
        ;
        GlueRuntime.chic_rt_install_type_metadata(& entry, 1usize);
        var ok = GlueRuntime.chic_rt_type_size(101u64) == 12usize;
        ok = ok && GlueRuntime.chic_rt_type_align(101u64) == 4usize;
        ok = ok && GlueRuntime.chic_rt_type_drop_glue(101u64) == 99isize;
        ok = ok && GlueRuntime.chic_rt_type_clone_glue(101u64) == 0isize;
        ok = ok && GlueRuntime.chic_rt_type_hash_glue(101u64) == 0isize;
        ok = ok && GlueRuntime.chic_rt_type_eq_glue(101u64) == 0isize;

        var meta = new RuntimeTypeMetadata {
            size = 8usize, align = 8usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 1u
        }
        ;
        GlueRuntime.chic_rt_type_metadata_register(202u64, meta);
        var outMeta = new RuntimeTypeMetadata {
            size = 0usize, align = 0usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 0u
        }
        ;
        let status = GlueRuntime.chic_rt_type_metadata(202u64, & outMeta);
        ok = ok && status == 0;
        ok = ok && outMeta.size == 8usize;

        let ifacePtr = GlueRuntime.chic_rt_interface_defaults_ptr();
        let ifaceLen = GlueRuntime.chic_rt_interface_defaults_len();
        if (ifaceLen == 0u64)
        {
            ok = ok && ifacePtr == null;
        }

        GlueRuntime.chic_rt_clone_invoke(1isize, new ValueConstPtr {
            Pointer = NativePtr.NullConst(), Size = 0usize, Alignment = 1usize
        }
        , new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        );
        let cloned = GlueRuntime.chic_rt_closure_env_clone(NativePtr.NullConst(), 0u64, 1u64);
        ok = ok && cloned == null;

        let resolve = GlueRuntime.chic_rt_ffi_resolve(NativePtr.NullConst());
        ok = ok && resolve == null;
        let eager = GlueRuntime.chic_rt_ffi_eager_resolve(NativePtr.NullConst());
        ok = ok && eager == null;
        GlueRuntime.chic_rt_ffi_add_search_path(NativePtr.NullConst());
        GlueRuntime.chic_rt_ffi_set_default_pattern(NativePtr.NullConst());
        Assert.That(ok).IsTrue();
    }
}

testcase Given_glue_closure_env_and_metadata_updates_When_executed_Then_glue_closure_env_and_metadata_updates()
{
    unsafe {
        var ok = true;
        GlueRuntime.chic_rt_type_metadata_clear();
        var meta = new RuntimeTypeMetadata {
            size = 4usize, align = 4usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 0u
        }
        ;
        GlueRuntime.chic_rt_type_metadata_register(303u64, meta);
        meta.size = 6usize;
        GlueRuntime.chic_rt_type_metadata_register(303u64, meta);
        var outMeta = new RuntimeTypeMetadata {
            size = 0usize, align = 0usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 0u
        }
        ;
        let status = GlueRuntime.chic_rt_type_metadata(303u64, & outMeta);
        ok = ok && status == 0;
        ok = ok && outMeta.size == 6usize;

        let envPtr = GlueRuntime.chic_rt_closure_env_alloc(4u64, 1u64);
        ok = ok && envPtr != null;
        if (envPtr != null)
        {
            * envPtr = 1u8;
            * NativePtr.OffsetMut(envPtr, 1isize) = 2u8;
            * NativePtr.OffsetMut(envPtr, 2isize) = 3u8;
            * NativePtr.OffsetMut(envPtr, 3isize) = 4u8;
        }
        let clone = GlueRuntime.chic_rt_closure_env_clone(envPtr, 4u64, 1u64);
        ok = ok && clone != null;
        if (envPtr != null && clone != null)
        {
            let left = new ValueConstPtr {
                Pointer = NativePtr.AsConstPtr(envPtr), Size = 4usize, Alignment = 1usize
            }
            ;
            let right = new ValueConstPtr {
                Pointer = NativePtr.AsConstPtr(clone), Size = 4usize, Alignment = 1usize
            }
            ;
            ok = ok && BytesEqualValue(left, right, 4usize);
        }
        GlueRuntime.chic_rt_closure_env_free(clone, 4u64, 1u64);
        GlueRuntime.chic_rt_closure_env_free(envPtr, 4u64, 1u64);
        Assert.That(ok).IsTrue();
    }
}
