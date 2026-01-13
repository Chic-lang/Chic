#![allow(unsafe_code)]

use crate::runtime::thread::ThreadStatus;

// External bindings to the Chic-native sync runtime. When the native archive
// is linked, the Rust implementations are skipped and these shims link against
// the Chic runtime.
unsafe extern "C" {
    pub fn chic_rt_mutex_create() -> usize;
    pub fn chic_rt_mutex_destroy(handle: usize);
    pub fn chic_rt_mutex_lock(handle: usize);
    pub fn chic_rt_mutex_try_lock(handle: usize) -> bool;
    pub fn chic_rt_mutex_unlock(handle: usize);

    pub fn chic_rt_rwlock_create() -> usize;
    pub fn chic_rt_rwlock_destroy(handle: usize);
    pub fn chic_rt_rwlock_read_lock(handle: usize);
    pub fn chic_rt_rwlock_read_unlock(handle: usize);
    pub fn chic_rt_rwlock_try_read_lock(handle: usize) -> bool;
    pub fn chic_rt_rwlock_try_write_lock(handle: usize) -> bool;
    pub fn chic_rt_rwlock_write_lock(handle: usize);
    pub fn chic_rt_rwlock_write_unlock(handle: usize);

    pub fn chic_rt_condvar_create() -> usize;
    pub fn chic_rt_condvar_destroy(handle: usize);
    pub fn chic_rt_condvar_notify_one(handle: usize);
    pub fn chic_rt_condvar_notify_all(handle: usize);
    pub fn chic_rt_condvar_wait(condvar_handle: usize, mutex_handle: usize);

    pub fn chic_rt_once_create() -> usize;
    pub fn chic_rt_once_destroy(handle: usize);
    pub fn chic_rt_once_try_begin(handle: usize) -> bool;
    pub fn chic_rt_once_complete(handle: usize);
    pub fn chic_rt_once_wait(handle: usize);
    pub fn chic_rt_once_is_completed(handle: usize) -> bool;

    pub fn chic_rt_atomic_bool_load(ptr: *mut u8, order: u8) -> u8;
    pub fn chic_rt_atomic_bool_store(ptr: *mut u8, value: u8, order: u8);
    pub fn chic_rt_atomic_bool_compare_exchange(
        ptr: *mut u8,
        expected: u8,
        desired: u8,
        success: u8,
        failure: u8,
    ) -> u8;
    pub fn chic_rt_atomic_usize_load(ptr: *mut usize, order: u8) -> usize;
    pub fn chic_rt_atomic_usize_store(ptr: *mut usize, value: usize, order: u8);
    pub fn chic_rt_atomic_usize_fetch_add(ptr: *mut usize, value: usize, order: u8) -> usize;
    pub fn chic_rt_atomic_usize_fetch_sub(ptr: *mut usize, value: usize, order: u8) -> usize;
    pub fn chic_rt_atomic_usize_compare_exchange(
        ptr: *mut usize,
        expected: usize,
        desired: usize,
        success: u8,
        failure: u8,
    ) -> u8;
    pub fn chic_rt_atomic_i32_load(ptr: *mut i32, order: u8) -> i32;
    pub fn chic_rt_atomic_i32_store(ptr: *mut i32, value: i32, order: u8);
    pub fn chic_rt_atomic_i32_fetch_add(ptr: *mut i32, value: i32, order: u8) -> i32;
    pub fn chic_rt_atomic_i32_fetch_sub(ptr: *mut i32, value: i32, order: u8) -> i32;
    pub fn chic_rt_atomic_i32_compare_exchange(
        ptr: *mut i32,
        expected: i32,
        desired: i32,
        success: u8,
        failure: u8,
    ) -> u8;
    pub fn chic_rt_atomic_u32_load(ptr: *mut u32, order: u8) -> u32;
    pub fn chic_rt_atomic_u32_store(ptr: *mut u32, value: u32, order: u8);
    pub fn chic_rt_atomic_u32_fetch_add(ptr: *mut u32, value: u32, order: u8) -> u32;
    pub fn chic_rt_atomic_u32_fetch_sub(ptr: *mut u32, value: u32, order: u8) -> u32;
    pub fn chic_rt_atomic_u32_compare_exchange(
        ptr: *mut u32,
        expected: u32,
        desired: u32,
        success: u8,
        failure: u8,
    ) -> u8;
    pub fn chic_rt_atomic_i64_load(ptr: *mut i64, order: u8) -> i64;
    pub fn chic_rt_atomic_i64_store(ptr: *mut i64, value: i64, order: u8);
    pub fn chic_rt_atomic_i64_fetch_add(ptr: *mut i64, value: i64, order: u8) -> i64;
    pub fn chic_rt_atomic_i64_fetch_sub(ptr: *mut i64, value: i64, order: u8) -> i64;
    pub fn chic_rt_atomic_i64_compare_exchange(
        ptr: *mut i64,
        expected: i64,
        desired: i64,
        success: u8,
        failure: u8,
    ) -> u8;
    pub fn chic_rt_atomic_u64_load(ptr: *mut u64, order: u8) -> u64;
    pub fn chic_rt_atomic_u64_store(ptr: *mut u64, value: u64, order: u8);
    pub fn chic_rt_atomic_u64_fetch_add(ptr: *mut u64, value: u64, order: u8) -> u64;
    pub fn chic_rt_atomic_u64_fetch_sub(ptr: *mut u64, value: u64, order: u8) -> u64;
    pub fn chic_rt_atomic_u64_compare_exchange(
        ptr: *mut u64,
        expected: u64,
        desired: u64,
        success: u8,
        failure: u8,
    ) -> u8;

    pub fn chic_rt_thread_spawn(
        start: *const crate::runtime::thread::ThreadStart,
        handle: *mut crate::runtime::thread::ThreadHandle,
    ) -> ThreadStatus;
    pub fn chic_rt_thread_join(
        handle: *mut crate::runtime::thread::ThreadHandle,
    ) -> ThreadStatus;
    pub fn chic_rt_thread_detach(
        handle: *mut crate::runtime::thread::ThreadHandle,
    ) -> ThreadStatus;
    pub fn chic_rt_thread_sleep_ms(ms: u64);
    pub fn chic_rt_thread_yield();
    pub fn chic_rt_thread_spin_wait(iterations: u32);
}
