#![allow(unsafe_code)]

use chic::runtime::span::chic_rt_span_from_raw_mut;
use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{
    ChicVec, chic_rt_vec_inline_ptr, chic_rt_vec_len, chic_rt_vec_new, chic_rt_vec_push,
    chic_rt_vec_uses_inline,
};
use std::mem::{MaybeUninit, align_of, size_of};
use std::ptr;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VecLayoutInfo {
    size: usize,
    offset_ptr: isize,
    offset_len: isize,
    offset_cap: isize,
    offset_elem_size: isize,
    offset_elem_align: isize,
    offset_drop_fn: isize,
    offset_region_ptr: isize,
    offset_uses_inline: isize,
    offset_inline_storage: isize,
}

unsafe extern "C" {
    fn chic_rt_vec_layout_debug() -> VecLayoutInfo;
}

fn rust_layout() -> VecLayoutInfo {
    let uninit = MaybeUninit::<ChicVec>::uninit();
    let base = uninit.as_ptr() as *const u8;
    // SAFETY: We never read the uninitialized data; we only compute pointer arithmetic.
    unsafe {
        let vec_ptr = base;
        let len_ptr = (&(*uninit.as_ptr()).len as *const _ as *const u8).offset_from(base);
        let cap_ptr = (&(*uninit.as_ptr()).cap as *const _ as *const u8).offset_from(base);
        let elem_size_ptr =
            (&(*uninit.as_ptr()).elem_size as *const _ as *const u8).offset_from(base);
        let elem_align_ptr =
            (&(*uninit.as_ptr()).elem_align as *const _ as *const u8).offset_from(base);
        let drop_fn_ptr = (&(*uninit.as_ptr()).drop_fn as *const _ as *const u8).offset_from(base);
        let region_ptr = (&(*uninit.as_ptr()).region as *const _ as *const u8).offset_from(base);
        let uses_inline_ptr =
            (&(*uninit.as_ptr()).uses_inline as *const _ as *const u8).offset_from(base);
        let inline_storage_ptr =
            (&(*uninit.as_ptr()).inline_storage as *const _ as *const u8).offset_from(base);

        VecLayoutInfo {
            size: size_of::<ChicVec>(),
            offset_ptr: vec_ptr.offset_from(base),
            offset_len: len_ptr,
            offset_cap: cap_ptr,
            offset_elem_size: elem_size_ptr,
            offset_elem_align: elem_align_ptr,
            offset_drop_fn: drop_fn_ptr,
            offset_region_ptr: region_ptr,
            offset_uses_inline: uses_inline_ptr,
            offset_inline_storage: inline_storage_ptr,
        }
    }
}

#[test]
fn native_vec_layout_matches_rust() {
    let rust = rust_layout();
    let native = unsafe { chic_rt_vec_layout_debug() };
    assert_eq!(
        rust, native,
        "Rust vec layout {:?} does not match native {:?}",
        rust, native
    );
}

#[test]
fn vec_new_and_push_fields_are_stable() {
    let mut vec = unsafe { chic_rt_vec_new(size_of::<u32>(), align_of::<u32>(), None) };
    assert_eq!(vec.elem_size, size_of::<u32>());
    assert_eq!(vec.elem_align, align_of::<u32>());
    assert_eq!(vec.len, 0);
    // Inline storage is activated lazily on first capacity request (push/reserve).
    let uses_inline = unsafe { chic_rt_vec_uses_inline(&vec) };
    println!(
        "vec_new uses_inline={uses_inline} len={} cap={} ptr={:?}",
        vec.len, vec.cap, vec.ptr
    );
    assert_eq!(uses_inline, 0);
    assert_eq!(vec.cap, 0);
    assert!(vec.ptr.is_null());

    let value = ValueConstPtr {
        ptr: &1u32 as *const u32 as *const u8,
        size: size_of::<u32>(),
        align: align_of::<u32>(),
    };
    let status = unsafe { chic_rt_vec_push(&mut vec, value) };
    assert_eq!(status, 0, "push returned {}", status);
    assert_eq!(vec.len, 1);
    let uses_inline_after_push = unsafe { chic_rt_vec_uses_inline(&vec) };
    let len_via_rt = unsafe { chic_rt_vec_len(&vec) };
    println!(
        "after push: len field={} len via rt={} ptr={:?}",
        vec.len, len_via_rt, vec.ptr
    );
    assert_eq!(uses_inline_after_push, 1);
    assert_eq!(vec.cap >= 1, true);
    assert!(!vec.ptr.is_null());
    let inline_ptr = unsafe { chic_rt_vec_inline_ptr(&mut vec) };
    assert_eq!(vec.ptr, inline_ptr.ptr);
    let bytes = unsafe { std::slice::from_raw_parts(vec.ptr as *const u8, 4) };
    println!("inline bytes {:?}", bytes);
    assert_eq!(unsafe { ptr::read(vec.ptr as *const u32) }, 1);
}

#[test]
fn span_from_vec_reflects_length() {
    let mut vec = unsafe { chic_rt_vec_new(size_of::<u32>(), align_of::<u32>(), None) };
    for value in 0..3u32 {
        let handle = ValueConstPtr {
            ptr: &value as *const u32 as *const u8,
            size: size_of::<u32>(),
            align: align_of::<u32>(),
        };
        let status = unsafe { chic_rt_vec_push(&mut vec, handle) };
        assert_eq!(status, 0, "push status {status}");
    }
    println!(
        "vec after pushes len={} cap={} ptr={:?}",
        vec.len, vec.cap, vec.ptr
    );
    let data = ValueMutPtr {
        ptr: vec.ptr,
        size: size_of::<u32>(),
        align: align_of::<u32>(),
    };
    let span = unsafe { chic_rt_span_from_raw_mut(data, vec.len) };
    println!("span {:?}", span);
    assert_eq!(span.len, vec.len);
    assert_eq!(span.elem_size, size_of::<u32>());
}

#[test]
fn vec_push_u16_succeeds() {
    let mut vec = unsafe { chic_rt_vec_new(size_of::<u16>(), align_of::<u16>(), None) };
    for value in [0xBEEF_u16, 0x1234_u16, 0xABCD_u16] {
        let handle = ValueConstPtr {
            ptr: &value as *const u16 as *const u8,
            size: size_of::<u16>(),
            align: align_of::<u16>(),
        };
        let status = unsafe { chic_rt_vec_push(&mut vec, handle) };
        let bytes = unsafe { std::slice::from_raw_parts(vec.ptr as *const u8, vec.len * 2) };
        println!(
            "u16 push value {:04x} status {} len {} cap {} ptr {:?} bytes {:?}",
            value, status, vec.len, vec.cap, vec.ptr, bytes
        );
        assert_eq!(status, 0);
    }
    assert_eq!(vec.len, 3);
    let bytes = unsafe { std::slice::from_raw_parts(vec.ptr as *const u8, 6) };
    assert_eq!(bytes, [0xEF, 0xBE, 0x34, 0x12, 0xCD, 0xAB]);
}
