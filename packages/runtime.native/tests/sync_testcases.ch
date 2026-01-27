namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_sync_mutex_and_lock_helpers_When_executed_Then_sync_mutex_and_lock_helpers()
{
    unsafe {
        let handle = chic_rt_mutex_create();
        chic_rt_mutex_lock(handle);
        let held = chic_rt_lock_is_held(handle);
        let tryLocked = chic_rt_mutex_try_lock(handle);
        chic_rt_mutex_unlock(handle);
        let heldAfter = chic_rt_lock_is_held(handle);
        let lockHandle = chic_rt_lock_create();
        chic_rt_lock_enter(lockHandle);
        let heldByThread = chic_rt_lock_is_held_by_current_thread(lockHandle);
        chic_rt_lock_exit(lockHandle);
        chic_rt_lock_destroy(lockHandle);
        chic_rt_mutex_destroy(handle);
        let ok = handle != 0usize
            && held
            && !tryLocked
            && !heldAfter
            && heldByThread;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_sync_rwlock_condvar_once_When_executed_Then_sync_rwlock_condvar_once()
{
    unsafe {
        let rw = chic_rt_rwlock_create();
        chic_rt_rwlock_read_lock(rw);
        chic_rt_rwlock_read_unlock(rw);
        let readLocked = chic_rt_rwlock_try_read_lock(rw);
        chic_rt_rwlock_read_unlock(rw);
        chic_rt_rwlock_write_lock(rw);
        chic_rt_rwlock_write_unlock(rw);
        let writeLocked = chic_rt_rwlock_try_write_lock(rw);
        chic_rt_rwlock_write_unlock(rw);
        chic_rt_rwlock_destroy(rw);
        let cv = chic_rt_condvar_create();
        chic_rt_condvar_notify_one(cv);
        chic_rt_condvar_notify_all(cv);
        chic_rt_condvar_wait(cv, 0usize);
        chic_rt_condvar_destroy(cv);
        let once = chic_rt_once_create();
        let started = chic_rt_once_try_begin(once);
        chic_rt_once_complete(once);
        chic_rt_once_wait(once);
        let completed = chic_rt_once_is_completed(once);
        chic_rt_once_destroy(once);
        let ok = rw != 0usize && readLocked && writeLocked && started && completed;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_sync_atomic_operations_When_executed_Then_sync_atomic_operations()
{
    unsafe {
        var boolValue = (byte)0;
        let boolLoaded = chic_rt_atomic_bool_load(& boolValue, MemoryOrder.SeqCst);
        chic_rt_atomic_bool_store(& boolValue, 1u8, MemoryOrder.SeqCst);
        let boolLoaded2 = chic_rt_atomic_bool_load(& boolValue, MemoryOrder.SeqCst);
        let swapped = chic_rt_atomic_bool_compare_exchange(& boolValue, 1u8, 0u8, MemoryOrder.SeqCst);
        var usizeValue = 1usize;
        let usizeLoaded = chic_rt_atomic_usize_load(& usizeValue, MemoryOrder.SeqCst);
        chic_rt_atomic_usize_store(& usizeValue, 2usize, MemoryOrder.SeqCst);
        let usizeAdd = chic_rt_atomic_usize_fetch_add(& usizeValue, 3usize, MemoryOrder.SeqCst);
        let usizeLoaded2 = chic_rt_atomic_usize_load(& usizeValue, MemoryOrder.SeqCst);
        let usizeSub = chic_rt_atomic_usize_fetch_sub(& usizeValue, 2usize, MemoryOrder.SeqCst);
        var i32Value = 1;
        let i32Loaded = chic_rt_atomic_i32_load(& i32Value, MemoryOrder.SeqCst);
        chic_rt_atomic_i32_store(& i32Value, 2, MemoryOrder.SeqCst);
        let i32Add = chic_rt_atomic_i32_fetch_add(& i32Value, 3, MemoryOrder.SeqCst);
        let i32Sub = chic_rt_atomic_i32_fetch_sub(& i32Value, 1, MemoryOrder.SeqCst);
        let i32Swap = chic_rt_atomic_i32_compare_exchange(& i32Value, 4, 7, MemoryOrder.SeqCst);
        var u32Value = 1u;
        let u32Loaded = chic_rt_atomic_u32_load(& u32Value, MemoryOrder.SeqCst);
        chic_rt_atomic_u32_store(& u32Value, 2u, MemoryOrder.SeqCst);
        let u32Add = chic_rt_atomic_u32_fetch_add(& u32Value, 3u, MemoryOrder.SeqCst);
        let u32Sub = chic_rt_atomic_u32_fetch_sub(& u32Value, 1u, MemoryOrder.SeqCst);
        let u32Swap = chic_rt_atomic_u32_compare_exchange(& u32Value, 4u, 8u, MemoryOrder.SeqCst);
        var i64Value = 1L;
        let i64Loaded = chic_rt_atomic_i64_load(& i64Value, MemoryOrder.SeqCst);
        chic_rt_atomic_i64_store(& i64Value, 2L, MemoryOrder.SeqCst);
        let i64Add = chic_rt_atomic_i64_fetch_add(& i64Value, 3L, MemoryOrder.SeqCst);
        let i64Sub = chic_rt_atomic_i64_fetch_sub(& i64Value, 1L, MemoryOrder.SeqCst);
        let i64Swap = chic_rt_atomic_i64_compare_exchange(& i64Value, 4L, 9L, MemoryOrder.SeqCst);
        var u64Value = 1ul;
        let u64Loaded = chic_rt_atomic_u64_load(& u64Value, MemoryOrder.SeqCst);
        chic_rt_atomic_u64_store(& u64Value, 2ul, MemoryOrder.SeqCst);
        let u64Add = chic_rt_atomic_u64_fetch_add(& u64Value, 3ul, MemoryOrder.SeqCst);
        let u64Sub = chic_rt_atomic_u64_fetch_sub(& u64Value, 1ul, MemoryOrder.SeqCst);
        let u64Swap = chic_rt_atomic_u64_compare_exchange(& u64Value, 4ul, 10ul, MemoryOrder.SeqCst);
        let ok = boolLoaded == 0u8
            && boolLoaded2 == 1u8
            && swapped == 1u8
            && usizeLoaded == 1usize
            && usizeAdd == 2usize
            && usizeLoaded2 == 5usize
            && usizeSub == 5usize
            && i32Loaded == 1
            && i32Add == 2
            && i32Sub == 5
            && i32Swap == 1u8
            && u32Loaded == 1u
            && u32Add == 2u
            && u32Sub == 5u
            && u32Swap == 1u8
            && i64Loaded == 1L
            && i64Add == 2L
            && i64Sub == 5L
            && i64Swap == 1u8
            && u64Loaded == 1ul
            && u64Add == 2ul
            && u64Sub == 5ul
            && u64Swap == 1u8;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_sync_invalid_handles_noop_When_executed_Then_sync_invalid_handles_noop()
{
    unsafe {
        chic_rt_mutex_destroy(0usize);
        chic_rt_mutex_lock(0usize);
        let mutexTry = chic_rt_mutex_try_lock(0usize);
        chic_rt_mutex_unlock(0usize);
        let lockHeld = chic_rt_lock_is_held(0usize);
        let lockHeldByThread = chic_rt_lock_is_held_by_current_thread(0usize);

        chic_rt_rwlock_destroy(0usize);
        chic_rt_rwlock_read_lock(0usize);
        let rwRead = chic_rt_rwlock_try_read_lock(0usize);
        chic_rt_rwlock_read_unlock(0usize);
        chic_rt_rwlock_write_lock(0usize);
        let rwWrite = chic_rt_rwlock_try_write_lock(0usize);
        chic_rt_rwlock_write_unlock(0usize);

        chic_rt_condvar_destroy(0usize);
        chic_rt_condvar_notify_one(0usize);
        chic_rt_condvar_notify_all(0usize);
        chic_rt_condvar_wait(0usize, 0usize);

        chic_rt_once_destroy(0usize);
        chic_rt_once_wait(0usize);
        let onceDone = chic_rt_once_is_completed(0usize);
        let ok = !mutexTry && !lockHeld && !lockHeldByThread && !rwRead && !rwWrite && !onceDone;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_sync_lock_helpers_and_compare_exchange_failures_When_executed_Then_sync_lock_helpers_and_compare_exchange_failures()
{
    unsafe {
        let mutex = chic_rt_mutex_create();
        let tryLock = chic_rt_mutex_try_lock(mutex);
        let held = chic_rt_lock_is_held(mutex);
        let heldByThread = chic_rt_lock_is_held_by_current_thread(mutex);
        chic_rt_mutex_unlock(mutex);
        chic_rt_mutex_destroy(mutex);

        var flag = (byte)0;
        let noSwap = chic_rt_atomic_bool_compare_exchange(& flag, 1u8, 1u8, MemoryOrder.SeqCst);
        let swapped = chic_rt_atomic_bool_compare_exchange(& flag, 0u8, 1u8, MemoryOrder.SeqCst);
        let ok = tryLock && held && heldByThread && noSwap == 0u8 && swapped == 1u8;
        Assert.That(ok).IsTrue();
    }
}
