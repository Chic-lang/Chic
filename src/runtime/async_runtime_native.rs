#![allow(unsafe_code)]

use crate::runtime::async_runtime::{FutureHeader, RuntimeContext};

// External bindings to the Chic-native async runtime. These are only used when the
// `chic_native_runtime` cfg is active; the Rust executor is not compiled in that mode.

unsafe extern "C" {
    pub fn chic_rt_async_register_future(header: *mut FutureHeader);
    pub fn chic_rt_async_spawn(header: *mut FutureHeader);
    pub fn chic_rt_async_block_on(header: *mut FutureHeader);
    pub fn chic_rt_async_spawn_local(header: *mut FutureHeader) -> u32;
    pub fn chic_rt_async_scope(header: *mut FutureHeader) -> u32;
    pub fn chic_rt_async_cancel(header: *mut FutureHeader) -> u32;
    pub fn chic_rt_async_task_result(src_ptr: *const u8, out_ptr: *mut u8, out_len: u32) -> u32;
    pub fn chic_rt_async_token_state(state_ptr: *mut bool) -> u32;
    pub fn chic_rt_async_token_new() -> *mut bool;
    pub fn chic_rt_async_token_cancel(state_ptr: *mut bool) -> u32;
    pub fn chic_rt_await(ctx: *mut RuntimeContext, awaited: *mut FutureHeader) -> u32;
    pub fn chic_rt_yield(ctx: *mut RuntimeContext) -> u32;
}
