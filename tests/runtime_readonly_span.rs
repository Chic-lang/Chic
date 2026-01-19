#![allow(unsafe_code)]

#[path = "support/runtime_vec.rs"]
mod runtime_vec;

use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;
use std::sync::OnceLock;

use chic::runtime::span::{
    ChicReadOnlySpan, ChicSpan, SpanError, SpanLayoutInfo, chic_rt_span_copy_to,
    chic_rt_span_from_raw_const, chic_rt_span_from_raw_mut, chic_rt_span_layout_debug,
    chic_rt_span_slice_readonly, chic_rt_span_to_readonly,
};
use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{
    ChicVecView, VecError, chic_rt_vec_get_ptr, chic_rt_vec_inline_ptr, chic_rt_vec_uses_inline,
};
use runtime_vec::ManagedVec;

unsafe extern "C" {
    #[link_name = "chic_rt_vec_view"]
    fn chic_rt_vec_view_raw(vec: *const chic::runtime::vec::ChicVec, out: *mut ChicVecView) -> i32;
}

fn empty_readonly_span() -> ChicReadOnlySpan {
    ChicReadOnlySpan {
        data: ValueConstPtr {
            ptr: std::ptr::null(),
            size: 0,
            align: 0,
        },
        len: 0,
        elem_size: 1,
        elem_align: 1,
    }
}

fn runtime_span_abi_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        if !cfg!(target_os = "macos") {
            return true;
        }
        if std::env::var_os("CHIC_ENABLE_NATIVE_RUNTIME_ABI_COVERAGE").is_some() {
            return true;
        }
        eprintln!(
            "skipping span ABI tests on macOS (set CHIC_ENABLE_NATIVE_RUNTIME_ABI_COVERAGE=1 to run)"
        );
        false
    })
}

#[test]
fn span_layout_matches_runtime_contract() {
    let mut layout = MaybeUninit::<SpanLayoutInfo>::uninit();
    unsafe { chic_rt_span_layout_debug(layout.as_mut_ptr()) };
    let layout = unsafe { layout.assume_init() };
    assert_eq!(layout.size, 48);
    assert_eq!(layout.offset_data, 0);
    assert_eq!(layout.offset_reserved, 24);
    assert_eq!(layout.offset_len, 24);
    assert_eq!(layout.offset_elem_size, 32);
    assert_eq!(layout.offset_elem_align, 40);
}

#[test]
fn vec_inline_pointer_stays_inline_after_first_push() {
    let mut vec = ManagedVec::<u32>::new();
    vec.push(1);

    let uses_inline = unsafe { chic_rt_vec_uses_inline(vec.as_ptr()) };
    assert_eq!(uses_inline, 1, "vector should report inline storage");

    let data = unsafe { chic_rt_vec_get_ptr(vec.as_ptr()) };
    let inline_ptr = unsafe { chic_rt_vec_inline_ptr(vec.as_mut_ptr()) };
    assert_eq!(
        data.ptr, inline_ptr.ptr,
        "vector data pointer should point at inline storage after first push"
    );
    std::mem::forget(vec);
}

#[test]
fn readonly_span_multiple_views_share_data() {
    if !runtime_span_abi_enabled() {
        return;
    }
    let mut vec = ManagedVec::<u32>::new();
    vec.push(10);
    vec.push(20);
    vec.push(30);

    let span = unsafe { chic_rt_span_from_raw_mut(vec.as_value_mut_ptr(), vec.len()) };
    assert_eq!(span.len, 3);

    let readonly = unsafe { chic_rt_span_to_readonly(&span) };
    assert_eq!(readonly.len, 3);

    let mut view_out = MaybeUninit::<ChicVecView>::uninit();
    let status = unsafe { chic_rt_vec_view_raw(vec.as_ptr(), view_out.as_mut_ptr()) };
    assert_eq!(status, VecError::Success as i32);
    let view = unsafe { view_out.assume_init() };
    let readonly2 = unsafe { chic_rt_span_from_raw_const(vec_view_handle(view), view.len) };
    assert_eq!(readonly2.len, 3);

    let slice1 = unsafe { slice::from_raw_parts(readonly.data.ptr.cast::<u32>(), readonly.len) };
    let slice2 = unsafe { slice::from_raw_parts(readonly2.data.ptr.cast::<u32>(), readonly2.len) };
    assert_eq!(slice1, &[10, 20, 30]);
    assert_eq!(slice2, &[10, 20, 30]);

    let second_ptr = unsafe { elem_ptr_at_readonly(&readonly2, 1) } as *const u32;
    assert_eq!(unsafe { *second_ptr }, 20);
}

#[test]
fn readonly_span_copies_observe_shared_data() {
    if !runtime_span_abi_enabled() {
        return;
    }
    let mut vec = ManagedVec::<u32>::new();
    for value in 0..16u32 {
        vec.push(value);
    }

    let span = unsafe { chic_rt_span_from_raw_mut(vec.as_value_mut_ptr(), vec.len()) };
    let readonly = unsafe { chic_rt_span_to_readonly(&span) };

    let view_a = readonly;
    let view_b = readonly;

    let sum_a: u32 = (0..view_a.len)
        .map(|idx| unsafe {
            let ptr = elem_ptr_at_readonly(&view_a, idx) as *const u32;
            *ptr
        })
        .sum();
    let sum_b: u32 = (0..view_b.len)
        .rev()
        .map(|idx| unsafe {
            let ptr = elem_ptr_at_readonly(&view_b, idx) as *const u32;
            *ptr
        })
        .sum();

    assert_eq!(sum_a, (0..16u32).sum::<u32>());
    assert_eq!(sum_a, sum_b);
}

#[test]
fn readonly_span_slice_reports_out_of_bounds() {
    if !runtime_span_abi_enabled() {
        return;
    }
    let mut vec = ManagedVec::<u32>::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);

    let span = unsafe { chic_rt_span_from_raw_mut(vec.as_value_mut_ptr(), vec.len()) };
    let readonly = unsafe { chic_rt_span_to_readonly(&span) };

    let mut dest = empty_readonly_span();
    let status = unsafe { chic_rt_span_slice_readonly(&readonly, 2, 4, &mut dest) };
    assert_eq!(status, SpanError::OutOfBounds as i32);
}

#[test]
fn readonly_span_copy_to_smaller_destination_fails() {
    if !runtime_span_abi_enabled() {
        return;
    }
    let mut vec = ManagedVec::<u16>::new();
    let uses_inline = unsafe { chic_rt_vec_uses_inline(vec.as_ptr()) };
    assert_eq!(uses_inline, 1);
    vec.push(10);
    vec.push(20);
    vec.push(30);

    let span = unsafe { chic_rt_span_from_raw_mut(vec.as_value_mut_ptr(), vec.len()) };
    let readonly = unsafe { chic_rt_span_to_readonly(&span) };

    let mut dest = vec![0u16; 2];
    let dest_span = ChicSpan {
        data: ValueMutPtr {
            ptr: dest.as_mut_ptr().cast::<u8>(),
            size: std::mem::size_of::<u16>(),
            align: std::mem::align_of::<u16>(),
        },
        len: dest.len(),
        elem_size: std::mem::size_of::<u16>(),
        elem_align: std::mem::align_of::<u16>(),
    };
    let status = unsafe { chic_rt_span_copy_to(&readonly, &dest_span) };
    assert_eq!(status, SpanError::OutOfBounds as i32);
}

#[test]
fn span_from_array_mut_exposes_mutable_data() {
    if !runtime_span_abi_enabled() {
        return;
    }
    let mut vec = ManagedVec::<u32>::new();
    vec.push(10);
    vec.push(20);
    vec.push(30);

    let span = unsafe { chic_rt_span_from_raw_mut(vec.as_value_mut_ptr(), vec.len()) };
    assert_eq!(span.len, 3);
    assert_eq!(span.elem_size, std::mem::size_of::<u32>());

    let ptr = unsafe { elem_ptr_at_mut(&span, 1) as *mut u32 };
    let base_ptr = unsafe { (*vec.as_ptr()).ptr as usize };
    let ptr_addr = ptr as usize;
    assert_eq!(ptr_addr.wrapping_sub(base_ptr), std::mem::size_of::<u32>());
    unsafe { *ptr = 99 };

    let repr = unsafe { &*vec.as_mut_ptr() };
    let slice = unsafe { slice::from_raw_parts(repr.ptr.cast::<u32>(), repr.len) };
    assert_eq!(slice, &[10, 99, 30]);
}

fn vec_view_handle(view: ChicVecView) -> ValueConstPtr {
    let data = if view.elem_size == 0 {
        NonNull::<u8>::dangling().as_ptr()
    } else {
        view.data
    };
    ValueConstPtr {
        ptr: data,
        size: view.elem_size,
        align: view.elem_align,
    }
}

unsafe fn elem_ptr_at_readonly(span: &ChicReadOnlySpan, index: usize) -> *const u8 {
    assert!(index < span.len);
    unsafe { span.data.ptr.add(index * span.elem_size) }
}

unsafe fn elem_ptr_at_mut(span: &ChicSpan, index: usize) -> *mut u8 {
    assert!(index < span.len);
    unsafe { span.data.ptr.add(index * span.elem_size) }
}
