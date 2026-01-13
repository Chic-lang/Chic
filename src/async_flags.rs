//! Canonical async flag values shared between MIR lowering and codegen.
//!
//! These constants mirror the Chic runtime surface defined in
//! `Std.Async.FutureFlags` and the await status codes surfaced by
//! `Std.Async.RuntimeIntrinsics.chic_rt_await`.

/// Await result used by the runtime entry points.
pub const AWAIT_STATUS_PENDING: u32 = 0;
pub const AWAIT_STATUS_READY: u32 = 1;

/// Canonical await status for async waits.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwaitStatus {
    Pending = AWAIT_STATUS_PENDING,
    Ready = AWAIT_STATUS_READY,
}

/// Runtime flag bits that must stay in sync with `Std.Async.FutureFlags`.
pub const FUTURE_FLAG_READY: u32 = 0x0000_0001;
pub const FUTURE_FLAG_COMPLETED: u32 = 0x0000_0002;
pub const FUTURE_FLAG_CANCELLED: u32 = 0x0000_0004;
pub const FUTURE_FLAG_FAULTED: u32 = 0x8000_0000;
