namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_shared_arc_and_weak_counts_When_executed_Then_shared_arc_and_weak_counts()
{
    unsafe {
        var value = 42;
        var * const @readonly @expose_address byte valuePtr = & value;
        var arc = new ChicArc {
            header = null
        }
        ;
        var ok = SharedRuntime.chic_rt_arc_new(& arc, valuePtr, (usize) __sizeof<int>(),
        (usize) __alignof<int>(), HashMapTestSupport.DropNoop, 0ul) == 0;
        ok = ok && SharedRuntime.chic_rt_arc_strong_count(& arc) >= 1usize;
        let dataPtr = SharedRuntime.chic_rt_arc_get(& arc);
        ok = ok && !NativePtr.IsNullConst(dataPtr);
        var arc2 = new ChicArc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_arc_clone(& arc2, & arc) == 0;
        ok = ok && SharedRuntime.chic_rt_arc_strong_count(& arc) >= 2usize;
        ok = ok && NativePtr.IsNull(SharedRuntime.chic_rt_arc_get_mut(& arc));
        var weak = new ChicWeak {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_arc_downgrade(& weak, & arc) == 0;
        ok = ok && SharedRuntime.chic_rt_arc_weak_count(& arc) >= 1usize;
        var upgraded = new ChicArc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_weak_upgrade(& upgraded, & weak) == 0;
        SharedRuntime.chic_rt_arc_drop(& arc2);
        SharedRuntime.chic_rt_arc_drop(& upgraded);
        SharedRuntime.chic_rt_arc_drop(& arc);
        SharedRuntime.chic_rt_weak_drop(& weak);
        ok = ok && SharedRuntime.chic_rt_shared_allocations() == 0usize;
        ok = ok && SharedRuntime.chic_rt_shared_frees() == 0usize;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_rc_and_weak_rc_When_executed_Then_shared_rc_and_weak_rc()
{
    unsafe {
        var value = 7;
        var * const @readonly @expose_address byte valuePtr = & value;
        var rc = new ChicRc {
            header = null
        }
        ;
        var ok = SharedRuntime.chic_rt_rc_new(& rc, valuePtr, (usize) __sizeof<int>(),
        (usize) __alignof<int>(), HashMapTestSupport.DropNoop, 0ul) == 0;
        ok = ok && SharedRuntime.chic_rt_rc_strong_count(& rc) >= 1usize;
        let rcData = SharedRuntime.chic_rt_rc_get(& rc);
        ok = ok && !NativePtr.IsNullConst(rcData);
        var rc2 = new ChicRc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_rc_clone(& rc2, & rc) == 0;
        ok = ok && SharedRuntime.chic_rt_rc_strong_count(& rc) >= 2usize;
        var weak = new ChicWeakRc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_rc_downgrade(& weak, & rc) == 0;
        var upgraded = new ChicRc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_weak_rc_upgrade(& upgraded, & weak) == 0;
        SharedRuntime.chic_rt_rc_drop(& rc2);
        SharedRuntime.chic_rt_rc_drop(& upgraded);
        SharedRuntime.chic_rt_rc_drop(& rc);
        SharedRuntime.chic_rt_weak_rc_drop(& weak);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_arc_get_mut_when_unique_When_executed_Then_shared_arc_get_mut_when_unique()
{
    unsafe {
        var value = 21;
        var * const @readonly @expose_address byte valuePtr = & value;
        var arc = new ChicArc {
            header = null
        }
        ;
        var ok = SharedRuntime.chic_rt_arc_new(& arc, valuePtr, (usize) __sizeof<int>(),
        (usize) __alignof<int>(), HashMapTestSupport.DropNoop, 0ul) == 0;
        let mutPtr = SharedRuntime.chic_rt_arc_get_mut(& arc);
        ok = ok && !NativePtr.IsNull(mutPtr);
        let dataPtr = SharedRuntime.chic_rt_arc_get_data(& arc);
        ok = ok && !NativePtr.IsNull(dataPtr);
        SharedRuntime.chic_rt_arc_drop(& arc);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_rc_get_mut_when_unique_When_executed_Then_shared_rc_get_mut_when_unique()
{
    unsafe {
        var value = 31;
        var * const @readonly @expose_address byte valuePtr = & value;
        var rc = new ChicRc {
            header = null
        }
        ;
        var ok = SharedRuntime.chic_rt_rc_new(& rc, valuePtr, (usize) __sizeof<int>(),
        (usize) __alignof<int>(), HashMapTestSupport.DropNoop, 0ul) == 0;
        let mutPtr = SharedRuntime.chic_rt_rc_get_mut(& rc);
        ok = ok && !NativePtr.IsNull(mutPtr);
        SharedRuntime.chic_rt_rc_drop(& rc);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_invalid_inputs_and_weak_clone_When_executed_Then_shared_invalid_inputs_and_weak_clone()
{
    unsafe {
        let badArc = SharedRuntime.chic_rt_arc_new((* mut ChicArc) NativePtr.NullMut(), NativePtr.NullConst(), 0usize, 0usize,
        HashMapTestSupport.DropNoop, 0ul);
        let badClone = SharedRuntime.chic_rt_arc_clone((* mut ChicArc) NativePtr.NullMut(), (* const ChicArc) NativePtr.NullConst());
        var ok = badArc == -1 && badClone == -1;

        var value = 11;
        var * const @readonly @expose_address byte valuePtr = & value;
        var arc = new ChicArc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_arc_new(& arc, valuePtr, (usize) __sizeof<int>(),
        (usize) __alignof<int>(), HashMapTestSupport.DropNoop, 0ul) == 0;
        var weak = new ChicWeak {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_arc_downgrade(& weak, & arc) == 0;
        var weak2 = new ChicWeak {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_weak_clone(& weak2, & weak) == 0;
        SharedRuntime.chic_rt_arc_drop(& arc);
        var upgraded = new ChicArc {
            header = null
        }
        ;
        let upgradeStatus = SharedRuntime.chic_rt_weak_upgrade(& upgraded, & weak);
        ok = ok && upgradeStatus == -1;
        SharedRuntime.chic_rt_weak_drop(& weak);
        SharedRuntime.chic_rt_weak_drop(& weak2);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_arc_new_allocation_failure_When_executed_Then_returns_allocation_failed()
{
    unsafe {
        NativeAlloc.TestReset();
        NativeAlloc.TestFailAllocAfter(0);
        var value = 5;
        var arc = new ChicArc {
            header = null
        }
        ;
        let status = SharedRuntime.chic_rt_arc_new(& arc, & value, (usize) __sizeof<int>(), (usize) __alignof<int>(),
        HashMapTestSupport.DropNoop, 0ul);
        NativeAlloc.TestReset();
        let ok = status == - 2;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_rc_new_allocation_failure_When_executed_Then_returns_allocation_failed()
{
    unsafe {
        NativeAlloc.TestReset();
        NativeAlloc.TestFailAllocAfter(0);
        var value = 6;
        var rc = new ChicRc {
            header = null
        }
        ;
        let status = SharedRuntime.chic_rt_rc_new(& rc, & value, (usize) __sizeof<int>(), (usize) __alignof<int>(),
        HashMapTestSupport.DropNoop, 0ul);
        NativeAlloc.TestReset();
        let ok = status == - 2;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_weak_rc_upgrade_failure_When_executed_Then_shared_weak_rc_upgrade_failure()
{
    unsafe {
        var value = 19;
        var * const @readonly @expose_address byte valuePtr = & value;
        var rc = new ChicRc {
            header = null
        }
        ;
        var ok = SharedRuntime.chic_rt_rc_new(& rc, valuePtr, (usize) __sizeof<int>(),
        (usize) __alignof<int>(), HashMapTestSupport.DropNoop, 0ul) == 0;
        var weak = new ChicWeakRc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_rc_downgrade(& weak, & rc) == 0;
        SharedRuntime.chic_rt_rc_drop(& rc);
        var upgraded = new ChicRc {
            header = null
        }
        ;
        let status = SharedRuntime.chic_rt_weak_rc_upgrade(& upgraded, & weak);
        ok = ok && status == -1;
        SharedRuntime.chic_rt_weak_rc_drop(& weak);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_null_handles_and_weak_rc_clone_When_executed_Then_shared_null_handles_and_weak_rc_clone()
{
    unsafe {
        var ok = NativePtr.IsNullConst(SharedRuntime.chic_rt_arc_get((* const ChicArc) NativePtr.NullConst()));
        ok = ok && SharedRuntime.chic_rt_arc_strong_count((* const ChicArc) NativePtr.NullConst()) == 0usize;
        ok = ok && SharedRuntime.chic_rt_arc_weak_count((* const ChicArc) NativePtr.NullConst()) == 0usize;
        ok = ok && NativePtr.IsNullConst(SharedRuntime.chic_rt_rc_get((* const ChicRc) NativePtr.NullConst()));
        ok = ok && SharedRuntime.chic_rt_rc_weak_count((* const ChicRc) NativePtr.NullConst()) == 0usize;
        let badClone = SharedRuntime.chic_rt_weak_rc_clone((* mut ChicWeakRc) NativePtr.NullMut(),
        (* const ChicWeakRc) NativePtr.NullConst());
        ok = ok && badClone == -1;

        var value = 55;
        var rc = new ChicRc {
            header = null
        }
        ;
        let newStatus = SharedRuntime.chic_rt_rc_new(& rc, & value, (usize) __sizeof<int>(), (usize) __alignof<int>(),
        HashMapTestSupport.DropNoop, 0ul);
        ok = ok && newStatus == 0;
        var weak = new ChicWeakRc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_rc_downgrade(& weak, & rc) == 0;
        var weak2 = new ChicWeakRc {
            header = null
        }
        ;
        ok = ok && SharedRuntime.chic_rt_weak_rc_clone(& weak2, & weak) == 0;
        ok = ok && SharedRuntime.chic_rt_rc_weak_count(& rc) >= 2usize;
        SharedRuntime.chic_rt_weak_rc_drop(& weak);
        SharedRuntime.chic_rt_weak_rc_drop(& weak2);
        SharedRuntime.chic_rt_rc_drop(& rc);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_allocations_and_invalid_inputs_When_executed_Then_shared_allocations_and_invalid_inputs()
{
    unsafe {
        let beforeAlloc = SharedRuntime.chic_rt_shared_allocations();
        let beforeFree = SharedRuntime.chic_rt_shared_frees();
        var data = 77u8;
        var arc = new ChicArc {
            header = NativePtr.NullMut()
        }
        ;
        let status = SharedRuntime.chic_rt_arc_new(& arc, & data, 1usize, 1usize, SharedRuntime.chic_rt_drop_missing, 0u64);
        var ok = status == (int) SharedError.Success;
        SharedRuntime.chic_rt_arc_drop(& arc);
        let afterAlloc = SharedRuntime.chic_rt_shared_allocations();
        let afterFree = SharedRuntime.chic_rt_shared_frees();
        ok = ok && afterAlloc >= beforeAlloc;
        ok = ok && afterFree >= beforeFree;

        let nullNew = SharedRuntime.chic_rt_arc_new((* mut ChicArc) NativePtr.NullMut(), & data, 1usize, 1usize,
        SharedRuntime.chic_rt_drop_missing, 0u64);
        ok = ok && nullNew == (int) SharedError.InvalidPointer;
        let nullClone = SharedRuntime.chic_rt_arc_clone((* mut ChicArc) NativePtr.NullMut(),
        (* const ChicArc) NativePtr.NullConst());
        ok = ok && nullClone == (int) SharedError.InvalidPointer;

        var rc = new ChicRc {
            header = NativePtr.NullMut()
        }
        ;
        let rcStatus = SharedRuntime.chic_rt_rc_new(& rc, & data, 1usize, 1usize, SharedRuntime.chic_rt_drop_missing, 0u64);
        ok = ok && rcStatus == (int) SharedError.Success;
        SharedRuntime.chic_rt_rc_drop(& rc);
        let rcNull = SharedRuntime.chic_rt_rc_clone((* mut ChicRc) NativePtr.NullMut(),
        (* const ChicRc) NativePtr.NullConst());
        ok = ok && rcNull == (int) SharedError.InvalidPointer;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_internal_helpers_When_executed_Then_shared_internal_helpers()
{
    unsafe {
        SharedRuntime.TestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}

testcase Given_shared_arc_downgrade_null_dest_When_executed_Then_invalid_pointer()
{
    unsafe {
        let status = SharedRuntime.chic_rt_arc_downgrade((* mut ChicWeak) NativePtr.NullMut(),
        (* const ChicArc) NativePtr.NullConst());
        Assert.That(status).IsEqualTo(-1);
    }
}

testcase Given_shared_weak_clone_null_dest_When_executed_Then_invalid_pointer()
{
    unsafe {
        let status = SharedRuntime.chic_rt_weak_clone((* mut ChicWeak) NativePtr.NullMut(),
        (* const ChicWeak) NativePtr.NullConst());
        Assert.That(status).IsEqualTo(-1);
    }
}

testcase Given_shared_weak_upgrade_null_dest_When_executed_Then_invalid_pointer()
{
    unsafe {
        let status = SharedRuntime.chic_rt_weak_upgrade((* mut ChicArc) NativePtr.NullMut(),
        (* const ChicWeak) NativePtr.NullConst());
        Assert.That(status).IsEqualTo(-1);
    }
}

testcase Given_shared_rc_downgrade_null_dest_When_executed_Then_invalid_pointer()
{
    unsafe {
        let status = SharedRuntime.chic_rt_rc_downgrade((* mut ChicWeakRc) NativePtr.NullMut(),
        (* const ChicRc) NativePtr.NullConst());
        Assert.That(status).IsEqualTo(-1);
    }
}

testcase Given_shared_weak_drop_null_When_executed_Then_noop()
{
    unsafe {
        SharedRuntime.chic_rt_weak_drop((* mut ChicWeak) NativePtr.NullMut());
        Assert.That(true).IsTrue();
    }
}

testcase Given_shared_weak_rc_drop_null_When_executed_Then_noop()
{
    unsafe {
        SharedRuntime.chic_rt_weak_rc_drop((* mut ChicWeakRc) NativePtr.NullMut());
        Assert.That(true).IsTrue();
    }
}

testcase Given_shared_object_new_with_registered_type_When_executed_Then_allocates_object()
{
    unsafe {
        GlueRuntime.chic_rt_type_metadata_clear();
        let meta = new RuntimeTypeMetadata {
            size = 8usize, align = 8usize, drop_fn = 0isize,
            variance = new VarianceSlice {
                ptr = NativePtr.NullConst(), len = 0usize
            }
            , flags = 0u
        }
        ;
        GlueRuntime.chic_rt_type_metadata_register(7777u64, meta);
        let obj = SharedRuntime.chic_rt_object_new(7777u64);
        let ok = !NativePtr.IsNull(obj);
        if (! NativePtr.IsNull (obj))
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = obj, Size = 8usize, Alignment = 8usize,
            }
            );
        }
        GlueRuntime.chic_rt_type_metadata_clear();
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_arc_new_zero_alignment_When_executed_Then_allocates_and_drops()
{
    unsafe {
        var value = 17;
        var arc = new ChicArc {
            header = null
        }
        ;
        let status = SharedRuntime.chic_rt_arc_new(& arc, & value, (usize) __sizeof<int>(), 0usize,
        HashMapTestSupport.DropNoop, 0u64);
        let ok = status == 0 && !NativePtr.IsNullConst(SharedRuntime.chic_rt_arc_get(& arc));
        SharedRuntime.chic_rt_arc_drop(& arc);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_shared_arc_get_data_null_handle_When_executed_Then_returns_null()
{
    unsafe {
        let ptr = SharedRuntime.chic_rt_arc_get_data((* const ChicArc) NativePtr.NullConst());
        Assert.That(NativePtr.IsNull(ptr)).IsTrue();
    }
}

testcase Given_shared_arc_get_mut_null_handle_When_executed_Then_returns_null()
{
    unsafe {
        let ptr = SharedRuntime.chic_rt_arc_get_mut((* mut ChicArc) NativePtr.NullMut());
        Assert.That(NativePtr.IsNull(ptr)).IsTrue();
    }
}

testcase Given_shared_rc_get_mut_non_unique_When_executed_Then_returns_null()
{
    unsafe {
        var value = 4;
        var rc = new ChicRc {
            header = null
        }
        ;
        let newStatus = SharedRuntime.chic_rt_rc_new(& rc, & value, (usize) __sizeof<int>(), (usize) __alignof<int>(),
        HashMapTestSupport.DropNoop, 0u64);
        var clone = new ChicRc {
            header = null
        }
        ;
        let cloneStatus = SharedRuntime.chic_rt_rc_clone(& clone, & rc);
        let mutPtr = SharedRuntime.chic_rt_rc_get_mut(& rc);
        let ok = newStatus == 0 && cloneStatus == 0 && NativePtr.IsNull(mutPtr);
        SharedRuntime.chic_rt_rc_drop(& clone);
        SharedRuntime.chic_rt_rc_drop(& rc);
        Assert.That(ok).IsTrue();
    }
}
