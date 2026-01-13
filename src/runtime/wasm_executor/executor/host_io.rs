#![allow(unsafe_code)]

use std::sync::Arc;

/// Host-provided IO/time hooks for WASM execution. When set on
/// `WasmExecutionOptions`, the executor will route env.write/read/isatty to
/// these callbacks instead of the built-in byte buffers.
#[derive(Clone)]
pub struct IoHooks {
    pub write: Option<Arc<dyn Fn(i32, *const u8, usize, *mut usize) -> i32 + Send + Sync>>,
    pub flush: Option<Arc<dyn Fn(i32) -> i32 + Send + Sync>>,
    pub read: Option<Arc<dyn Fn(i32, *mut u8, usize, *mut usize) -> i32 + Send + Sync>>,
    pub monotonic_nanos: Option<Arc<dyn Fn() -> i64 + Send + Sync>>,
    pub sleep_millis: Option<Arc<dyn Fn(u64) -> i32 + Send + Sync>>,
    pub fopen: Option<Arc<dyn Fn(&str, &str) -> i32 + Send + Sync>>,
    pub fread: Option<Arc<dyn Fn(i32, &mut [u8]) -> Result<usize, i32> + Send + Sync>>,
    pub fwrite: Option<Arc<dyn Fn(i32, &[u8]) -> Result<usize, i32> + Send + Sync>>,
    pub fflush: Option<Arc<dyn Fn(i32) -> i32 + Send + Sync>>,
    pub fclose: Option<Arc<dyn Fn(i32) -> i32 + Send + Sync>>,
    pub socket: Option<Arc<dyn Fn(i32, i32, i32) -> Result<i32, i32> + Send + Sync>>,
    pub connect: Option<Arc<dyn Fn(i32, u32, u16) -> i32 + Send + Sync>>,
    pub recv: Option<Arc<dyn Fn(i32, &mut [u8]) -> Result<usize, i32> + Send + Sync>>,
    pub send: Option<Arc<dyn Fn(i32, &[u8]) -> Result<usize, i32> + Send + Sync>>,
    pub shutdown: Option<Arc<dyn Fn(i32, i32) -> i32 + Send + Sync>>,
    pub close_socket: Option<Arc<dyn Fn(i32) -> i32 + Send + Sync>>,
    pub inet_pton: Option<Arc<dyn Fn(i32, &str) -> Result<[u8; 4], i32> + Send + Sync>>,
    pub htons: Option<Arc<dyn Fn(u16) -> u16 + Send + Sync>>,
}

impl IoHooks {
    pub fn empty() -> Self {
        Self {
            write: None,
            flush: None,
            read: None,
            monotonic_nanos: None,
            sleep_millis: None,
            fopen: None,
            fread: None,
            fwrite: None,
            fflush: None,
            fclose: None,
            socket: None,
            connect: None,
            recv: None,
            send: None,
            shutdown: None,
            close_socket: None,
            inet_pton: None,
            htons: None,
        }
    }
}
