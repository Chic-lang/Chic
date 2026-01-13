#![allow(clippy::missing_panics_doc)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; Rust sync primitives have been removed."
);

unsafe extern "C" {
    pub fn chic_rt_mutex_create() -> usize;
    pub fn chic_rt_mutex_destroy(handle: usize);
    pub fn chic_rt_mutex_lock(handle: usize);
    pub fn chic_rt_mutex_try_lock(handle: usize) -> bool;
    pub fn chic_rt_mutex_unlock(handle: usize);

    pub fn chic_rt_lock_create() -> usize;
    pub fn chic_rt_lock_destroy(handle: usize);
    pub fn chic_rt_lock_enter(handle: usize);
    pub fn chic_rt_lock_try_enter(handle: usize) -> bool;
    pub fn chic_rt_lock_exit(handle: usize);
    pub fn chic_rt_lock_is_held(handle: usize) -> bool;
    pub fn chic_rt_lock_is_held_by_current_thread(handle: usize) -> bool;

    pub fn chic_rt_rwlock_create() -> usize;
    pub fn chic_rt_rwlock_destroy(handle: usize);
    pub fn chic_rt_rwlock_read_lock(handle: usize);
    pub fn chic_rt_rwlock_try_read_lock(handle: usize) -> bool;
    pub fn chic_rt_rwlock_read_unlock(handle: usize);
    pub fn chic_rt_rwlock_write_lock(handle: usize);
    pub fn chic_rt_rwlock_try_write_lock(handle: usize) -> bool;
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

    pub fn chic_rt_atomic_bool_load(target: *const std::sync::atomic::AtomicBool, order: u8) -> u8;
    pub fn chic_rt_atomic_bool_store(
        target: *mut std::sync::atomic::AtomicBool,
        value: u8,
        order: u8,
    );
    pub fn chic_rt_atomic_bool_compare_exchange(
        target: *mut std::sync::atomic::AtomicBool,
        expected: u8,
        desired: u8,
        order: u8,
    ) -> u8;
    pub fn chic_rt_atomic_usize_load(
        target: *const std::sync::atomic::AtomicUsize,
        order: u8,
    ) -> usize;
    pub fn chic_rt_atomic_usize_store(
        target: *mut std::sync::atomic::AtomicUsize,
        value: usize,
        order: u8,
    );
    pub fn chic_rt_atomic_usize_fetch_add(
        target: *mut std::sync::atomic::AtomicUsize,
        value: usize,
        order: u8,
    ) -> usize;
    pub fn chic_rt_atomic_usize_fetch_sub(
        target: *mut std::sync::atomic::AtomicUsize,
        value: usize,
        order: u8,
    ) -> usize;
    pub fn chic_rt_atomic_i32_load(target: *const std::sync::atomic::AtomicI32, order: u8) -> i32;
    pub fn chic_rt_atomic_i32_store(
        target: *mut std::sync::atomic::AtomicI32,
        value: i32,
        order: u8,
    );
    pub fn chic_rt_atomic_i32_fetch_add(
        target: *mut std::sync::atomic::AtomicI32,
        value: i32,
        order: u8,
    ) -> i32;
    pub fn chic_rt_atomic_i32_fetch_sub(
        target: *mut std::sync::atomic::AtomicI32,
        value: i32,
        order: u8,
    ) -> i32;
    pub fn chic_rt_atomic_i32_compare_exchange(
        target: *mut std::sync::atomic::AtomicI32,
        expected: i32,
        desired: i32,
        order: u8,
    ) -> u8;
    pub fn chic_rt_atomic_u32_load(target: *const std::sync::atomic::AtomicU32, order: u8) -> u32;
    pub fn chic_rt_atomic_u32_store(
        target: *mut std::sync::atomic::AtomicU32,
        value: u32,
        order: u8,
    );
    pub fn chic_rt_atomic_u32_fetch_add(
        target: *mut std::sync::atomic::AtomicU32,
        value: u32,
        order: u8,
    ) -> u32;
    pub fn chic_rt_atomic_u32_fetch_sub(
        target: *mut std::sync::atomic::AtomicU32,
        value: u32,
        order: u8,
    ) -> u32;
    pub fn chic_rt_atomic_u32_compare_exchange(
        target: *mut std::sync::atomic::AtomicU32,
        expected: u32,
        desired: u32,
        order: u8,
    ) -> u8;
    pub fn chic_rt_atomic_i64_load(target: *const std::sync::atomic::AtomicI64, order: u8) -> i64;
    pub fn chic_rt_atomic_i64_store(
        target: *mut std::sync::atomic::AtomicI64,
        value: i64,
        order: u8,
    );
    pub fn chic_rt_atomic_i64_fetch_add(
        target: *mut std::sync::atomic::AtomicI64,
        value: i64,
        order: u8,
    ) -> i64;
    pub fn chic_rt_atomic_i64_fetch_sub(
        target: *mut std::sync::atomic::AtomicI64,
        value: i64,
        order: u8,
    ) -> i64;
    pub fn chic_rt_atomic_i64_compare_exchange(
        target: *mut std::sync::atomic::AtomicI64,
        expected: i64,
        desired: i64,
        order: u8,
    ) -> u8;
    pub fn chic_rt_atomic_u64_load(target: *const std::sync::atomic::AtomicU64, order: u8) -> u64;
    pub fn chic_rt_atomic_u64_store(
        target: *mut std::sync::atomic::AtomicU64,
        value: u64,
        order: u8,
    );
    pub fn chic_rt_atomic_u64_fetch_add(
        target: *mut std::sync::atomic::AtomicU64,
        value: u64,
        order: u8,
    ) -> u64;
    pub fn chic_rt_atomic_u64_fetch_sub(
        target: *mut std::sync::atomic::AtomicU64,
        value: u64,
        order: u8,
    ) -> u64;
    pub fn chic_rt_atomic_u64_compare_exchange(
        target: *mut std::sync::atomic::AtomicU64,
        expected: u64,
        desired: u64,
        order: u8,
    ) -> u8;
}
