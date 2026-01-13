use criterion::{Criterion, criterion_group, criterion_main};

use chic::async_flags::{AwaitStatus, FUTURE_FLAG_COMPLETED, FUTURE_FLAG_READY};
use chic::runtime::async_runtime::{
    FutureHeader, FutureVTable, RuntimeContext, chic_rt_async_scope, chic_rt_async_spawn_local,
};

unsafe extern "C" fn ready_poll(header: *mut FutureHeader, _ctx: *mut RuntimeContext) -> u32 {
    unsafe {
        (*header).flags |= FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED;
    }
    AwaitStatus::Ready as u32
}

unsafe extern "C" fn drop_noop(_header: *mut FutureHeader) {}

unsafe extern "C" fn pending_then_ready_poll(
    header: *mut FutureHeader,
    _ctx: *mut RuntimeContext,
) -> u32 {
    let counter_ptr = unsafe { (*header).state_pointer as *mut u32 };
    if counter_ptr.is_null() {
        return AwaitStatus::Pending as u32;
    }
    let polls = unsafe { counter_ptr.read() };
    if polls >= 1 {
        unsafe {
            (*header).flags |= FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED;
        }
        AwaitStatus::Ready as u32
    } else {
        unsafe { counter_ptr.write(polls + 1) };
        AwaitStatus::Pending as u32
    }
}

fn make_header(vtable: &FutureVTable) -> FutureHeader {
    FutureHeader {
        state_pointer: 0,
        vtable_pointer: vtable as *const _ as isize,
        executor_context: 0,
        flags: 0,
    }
}

fn bench_scope_ready(c: &mut Criterion) {
    let vtable = FutureVTable {
        poll: ready_poll,
        drop: drop_noop,
    };
    c.bench_function("async_scope_ready", |b| {
        b.iter(|| {
            let mut header = make_header(&vtable);
            let status = unsafe { chic_rt_async_scope(&mut header) };
            assert_eq!(status, AwaitStatus::Ready as u32);
        });
    });
}

fn bench_scope_pending_once(c: &mut Criterion) {
    let vtable = FutureVTable {
        poll: pending_then_ready_poll,
        drop: drop_noop,
    };
    c.bench_function("async_scope_pending_once", |b| {
        b.iter(|| {
            let mut poll_counter: u32 = 0;
            let mut header = make_header(&vtable);
            header.state_pointer = &mut poll_counter as *mut _ as isize;
            let status = unsafe { chic_rt_async_scope(&mut header) };
            assert_eq!(status, AwaitStatus::Ready as u32);
        });
    });
}

fn bench_spawn_local_ready(c: &mut Criterion) {
    let vtable = FutureVTable {
        poll: ready_poll,
        drop: drop_noop,
    };
    c.bench_function("async_spawn_local_ready", |b| {
        b.iter(|| {
            let mut header = make_header(&vtable);
            let status = unsafe { chic_rt_async_spawn_local(&mut header) };
            assert_eq!(status, AwaitStatus::Ready as u32);
        });
    });
}

fn bench_spawn_local_pending_once(c: &mut Criterion) {
    let vtable = FutureVTable {
        poll: pending_then_ready_poll,
        drop: drop_noop,
    };
    c.bench_function("async_spawn_local_pending_once", |b| {
        b.iter(|| {
            let mut poll_counter: u32 = 0;
            let mut header = make_header(&vtable);
            header.state_pointer = &mut poll_counter as *mut _ as isize;
            let status = unsafe { chic_rt_async_spawn_local(&mut header) };
            assert_eq!(status, AwaitStatus::Ready as u32);
        });
    });
}

fn runtime_async_benches(c: &mut Criterion) {
    bench_scope_ready(c);
    bench_scope_pending_once(c);
    bench_spawn_local_ready(c);
    bench_spawn_local_pending_once(c);
}

criterion_group!(runtime_async_scopes, runtime_async_benches);
criterion_main!(runtime_async_scopes);
