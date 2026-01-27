namespace Std.Compiler.Object;
import Std.Memory;
import Std.Testing;
testcase Given_object_runtime_type_id_is_nonzero_When_executed_Then_object_runtime_type_id_is_nonzero()
{
    let typeId = ObjectRuntime.TypeIdOf <ObjectRuntime >();
    Assert.That(typeId).IsNotEqualTo(0ul);
}
@repr(c) private struct SampleA
{
    public int a;
    public byte b;
}
@repr(c) private struct SampleB
{
    public ulong x;
    public ulong y;
}
private unsafe void InstallTypeMetadata(* const @readonly @expose_address TypeMetadataEntry entry) {
    ObjectRuntime.InstallTypeMetadataTable(entry, 1usize);
}
testcase Given_object_runtime_size_matches_intrinsic_When_executed_Then_object_runtime_size_matches_intrinsic()
{
    let typeId = ObjectRuntime.TypeIdOf <ObjectRuntime >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeId, size = __sizeof <ObjectRuntime >(), align = __alignof <ObjectRuntime >(), drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
    }
    let size = ObjectRuntime.SizeOf(typeId);
    Assert.That(size == __sizeof <ObjectRuntime >()).IsTrue();
}
testcase Given_object_runtime_align_matches_intrinsic_When_executed_Then_object_runtime_align_matches_intrinsic()
{
    let typeId = ObjectRuntime.TypeIdOf <ObjectRuntime >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeId, size = __sizeof <ObjectRuntime >(), align = __alignof <ObjectRuntime >(), drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
    }
    let align = ObjectRuntime.AlignOf(typeId);
    Assert.That(align == __alignof <ObjectRuntime >()).IsTrue();
}
testcase Given_object_runtime_metadata_resolve_matches_intrinsics_When_executed_Then_object_runtime_metadata_resolve_matches_intrinsics()
{
    let typeId = ObjectRuntime.TypeIdOf <ObjectRuntime >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeId, size = __sizeof <ObjectRuntime >(), align = __alignof <ObjectRuntime >(), drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
    }
    let metadata = ObjectRuntime.MetadataOf <ObjectRuntime >();
    Assert.That(metadata.Size == __sizeof <ObjectRuntime >()).IsTrue();
    Assert.That(metadata.Align == __alignof <ObjectRuntime >()).IsTrue();
}
testcase Given_object_runtime_metadata_try_get_succeeds_When_executed_Then_object_runtime_metadata_try_get_succeeds()
{
    let typeId = ObjectRuntime.TypeIdOf <ObjectRuntime >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeId, size = __sizeof <ObjectRuntime >(), align = __alignof <ObjectRuntime >(), drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
    }
    var metadata = new Std.Runtime.TypeMetadataRecord(0, 0, 0);
    let ok = ObjectRuntime.TryGetMetadata(typeId, out metadata);
    Assert.That(ok).IsTrue();
    Assert.That(metadata.Size == __sizeof <ObjectRuntime >()).IsTrue();
}
testcase Given_object_runtime_try_get_fails_for_unknown_type_When_executed_Then_returns_false()
{
    let typeId = 0x1234_5678_9abc_def0ul;
    var metadata = new Std.Runtime.TypeMetadataRecord(77, 88, 99);
    let ok = ObjectRuntime.TryGetMetadata(typeId, out metadata);
    Assert.That(ok).IsFalse();
    Assert.That(metadata.Size == 0usize).IsTrue();
    Assert.That(metadata.Align == 0usize).IsTrue();
}
testcase Given_object_runtime_hash_eq_glue_tables_When_installed_Then_lookup_returns_indices()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    let typeB = ObjectRuntime.TypeIdOf <SampleB >();
    unsafe {
        var entryA = new GlueIndexEntry {
            type_id = typeA, function_index = 42u
        }
        ;
        ObjectRuntime.InstallHashGlueTable(& entryA, 1usize);
        ObjectRuntime.InstallEqGlueTable(& entryA, 1usize);
        Assert.That(ObjectRuntime.HashGlueOf(typeA) == 42isize).IsTrue();
        Assert.That(ObjectRuntime.EqGlueOf(typeA) == 42isize).IsTrue();
        Assert.That(ObjectRuntime.HashGlueOf(typeB) == 0isize).IsTrue();
        var entryB = new GlueIndexEntry {
            type_id = typeB, function_index = 7u
        }
        ;
        ObjectRuntime.InstallHashGlueTable(& entryB, 1usize);
        ObjectRuntime.InstallEqGlueTable(& entryB, 1usize);
        Assert.That(ObjectRuntime.HashGlueOf(typeB) == 7isize).IsTrue();
        Assert.That(ObjectRuntime.EqGlueOf(typeB) == 7isize).IsTrue();
        ObjectRuntime.ClearTables();
    }
}
testcase Given_object_runtime_drop_glue_When_installed_Then_queries_return_value()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeA, size = __sizeof <SampleA >(), align = __alignof <SampleA >(), drop_fn = 55isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
    }
    Assert.That(ObjectRuntime.DropGlueOf(typeA) == 55isize).IsTrue();
    unsafe {
        Assert.That(ObjectRuntime.DropGlueOf(typeA) == 55isize).IsTrue();
        Assert.That(ObjectRuntime.CloneGlueOf(typeA) == 0isize).IsTrue();
    }
}
testcase Given_type_metadata_install_overwrites_existing_When_executed_Then_latest_wins()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    unsafe {
        var entryA = new TypeMetadataEntry {
            type_id = typeA, size = 1usize, align = 1usize, drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entryA);
        var entryB = new TypeMetadataEntry {
            type_id = typeA, size = 9usize, align = 4usize, drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entryB);
        var meta = new Std.Runtime.TypeMetadataRecord(0, 0, 0);
        let ok = ObjectRuntime.TryGetMetadata(typeA, out meta);
        Assert.That(ok).IsTrue();
        Assert.That(meta.Size == 9usize).IsTrue();
        Assert.That(meta.Align == 4usize).IsTrue();
    }
}
testcase Given_type_metadata_clear_When_executed_Then_queries_see_empty_tables()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeA, size = __sizeof <SampleA >(), align = __alignof <SampleA >(), drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
        ObjectRuntime.ClearTables();
    }
    Assert.That(ObjectRuntime.SizeOf(typeA) == 0usize).IsTrue();
    Assert.That(ObjectRuntime.AlignOf(typeA) == 0usize).IsTrue();
    Assert.That(ObjectRuntime.DropGlueOf(typeA) == 0isize).IsTrue();
}
testcase Given_install_type_metadata_table_When_used_Then_fill_reads_table_entries()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    let typeB = ObjectRuntime.TypeIdOf <SampleB >();
    unsafe {
        var entryA = new TypeMetadataEntry {
            type_id = typeA, size = __sizeof <SampleA >(), align = __alignof <SampleA >(), drop_fn = 1isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 11u,
        }
        ;
        ObjectRuntime.InstallTypeMetadataTable(& entryA, 1usize);
        Assert.That(ObjectRuntime.SizeOf(typeA) == __sizeof <SampleA >()).IsTrue();
        Assert.That(ObjectRuntime.AlignOf(typeA) == __alignof <SampleA >()).IsTrue();
        Assert.That(ObjectRuntime.DropGlueOf(typeA) == 1isize).IsTrue();
        Assert.That(ObjectRuntime.SizeOf(typeB) == 0usize).IsTrue();
        var outMeta = new Std.Runtime.TypeMetadataRecord(0, 0, 0);
        let okA = ObjectRuntime.TryGetMetadata(typeA, out outMeta);
        Assert.That(okA).IsTrue();
        Assert.That(outMeta.Size == __sizeof <SampleA >()).IsTrue();
        ObjectRuntime.ClearTables();
    }
}
testcase Given_type_metadata_export_When_out_pointer_is_null_Then_returns_invalid_pointer_code()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    unsafe {
        let rc = ObjectRuntime.chic_rt_type_metadata(typeA, (* mut @expose_address Std.Runtime.TypeMetadataRecord) 0);
        Assert.That(rc == 2).IsTrue();
    }
}
testcase Given_object_new_When_metadata_missing_Then_returns_null()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    unsafe {
        let ptr = ObjectRuntime.ObjectNew(typeA);
        Assert.That(ptr == null).IsTrue();
    }
}
testcase Given_object_new_When_align_is_zero_Then_returns_null()
{
    let bogusId = 0x9999_8888_7777_6666ul;
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = bogusId, size = 16usize, align = 0usize, drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
        let ptr = ObjectRuntime.ObjectNew(bogusId);
        Assert.That(ptr == null).IsTrue();
    }
}
testcase Given_object_new_When_metadata_present_Then_allocates_zeroed_memory()
{
    let typeA = ObjectRuntime.TypeIdOf <SampleA >();
    unsafe {
        var entry = new TypeMetadataEntry {
            type_id = typeA, size = __sizeof <SampleA >(), align = __alignof <SampleA >(), drop_fn = 0isize, variance = new Std.Runtime.VarianceSlice {
                Ptr = null, Len = 0usize
            }
            , flags = 0u,
        }
        ;
        InstallTypeMetadata(& entry);
        let ptr = ObjectRuntime.ObjectNew(typeA);
        Assert.That(ptr != null).IsTrue();
        Assert.That((* ptr) == 0u8).IsTrue();
    }
}
testcase Given_closure_env_alloc_clone_free_When_executed_Then_memory_roundtrips()
{
    unsafe {
        let null0 = ObjectRuntime.chic_rt_closure_env_alloc(0usize, 8usize);
        Assert.That(null0 == null).IsTrue();
        let size = 16usize;
        let src = ObjectRuntime.chic_rt_closure_env_alloc(size, 0usize);
        Assert.That(src != null).IsTrue();
        var i = 0usize;
        while (i <size)
        {
            let p = (* mut @expose_address byte)((isize) src + (isize) i);
            * p = (byte)(i + 1usize);
            i += 1usize;
        }
        let cloneNull = ObjectRuntime.chic_rt_closure_env_clone(null, size, 8usize);
        Assert.That(cloneNull == null).IsTrue();
        let cloned = ObjectRuntime.chic_rt_closure_env_clone(src, size, 8usize);
        Assert.That(cloned != null).IsTrue();
        Assert.That((* cloned) == 1u8).IsTrue();
        let last = (* const @readonly @expose_address byte)((isize) cloned + 15isize);
        Assert.That((* last) == 16u8).IsTrue();
        ObjectRuntime.chic_rt_closure_env_free(null, size, 8usize);
        ObjectRuntime.chic_rt_closure_env_free(src, 0usize, 8usize);
        ObjectRuntime.chic_rt_closure_env_free(src, size, 8usize);
        ObjectRuntime.chic_rt_closure_env_free(cloned, size, 8usize);
    }
}
testcase Given_pending_exception_state_When_set_and_taken_Then_behaves_deterministically()
{
    unsafe {
        Assert.That(ObjectRuntime.chic_rt_has_pending_exception() == 0).IsTrue();
        ObjectRuntime.chic_rt_throw(99i64, 0x5555_6666_7777_8888i64);
        Assert.That(ObjectRuntime.chic_rt_has_pending_exception() == 1).IsTrue();
        var payload = 0i64;
        var typeId = 0i64;
        let peekOk = ObjectRuntime.chic_rt_peek_pending_exception(& payload, & typeId);
        Assert.That(peekOk == 1).IsTrue();
        Assert.That(payload == 99i64).IsTrue();
        Assert.That(typeId == 0x5555_6666_7777_8888i64).IsTrue();
        let takeOk = ObjectRuntime.chic_rt_take_pending_exception(& payload, & typeId);
        Assert.That(takeOk == 1).IsTrue();
        Assert.That(ObjectRuntime.chic_rt_has_pending_exception() == 0).IsTrue();
        ObjectRuntime.chic_rt_clear_pending_exception();
        Assert.That(ObjectRuntime.chic_rt_has_pending_exception() == 0).IsTrue();
    }
}
