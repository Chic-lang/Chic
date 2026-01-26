#![allow(clippy::nursery)]
#![allow(clippy::use_self)]
#![allow(unused_unsafe)]

#[path = "support/runtime_string.rs"]
mod runtime_string;
#[path = "support/runtime_vec.rs"]
mod runtime_vec;

use std::alloc::Layout;
use std::ffi::CStr;
use std::mem::{align_of, size_of};
use std::ptr;
use std::slice;
use std::sync::{Mutex, OnceLock};

use runtime_string::{ManagedString, bytes_to_chic, str_to_chic};
use runtime_vec::ManagedVec;

use chic::decimal::{Decimal128, DecimalRoundingMode};
use chic::runtime::{
    DECIMAL_INTRINSICS, Decimal128Parts, DecimalConstPtr, DecimalIntrinsicVariant, DecimalMutPtr,
    DecimalRoundingAbi, DecimalRuntimeStatus, InterfaceDefaultDescriptor, chic_rt_decimal_add,
    chic_rt_decimal_clone, chic_rt_decimal_div, chic_rt_install_interface_defaults,
    chic_rt_interface_defaults_len, chic_rt_interface_defaults_ptr,
};

use chic::runtime::drop_glue::{
    __drop_noop, DropGlueEntry, chic_rt_drop_clear, chic_rt_drop_missing, chic_rt_drop_register,
    chic_rt_drop_resolve, chic_rt_install_drop_table,
};
use chic::runtime::span::{
    ChicReadOnlySpan, ChicSpan, SpanError, chic_rt_span_copy_to, chic_rt_span_fill,
    chic_rt_span_from_raw_const, chic_rt_span_from_raw_mut, chic_rt_span_slice_mut,
    chic_rt_span_slice_readonly, chic_rt_span_to_readonly,
};
use chic::runtime::startup::{
    ENTRY_FLAG_ASYNC, ENTRY_FLAG_PARAM_ARGS, EntryDescriptor, STARTUP_DESCRIPTOR_VERSION,
    StartupDescriptor, TESTCASE_FLAG_ASYNC, TestCaseDescriptor, TestSuiteDescriptor,
};
use chic::runtime::string::{
    ChicStr, StringError, chic_rt_string_append_bool, chic_rt_string_append_char,
    chic_rt_string_append_f32, chic_rt_string_append_f64, chic_rt_string_append_signed,
    chic_rt_string_append_slice, chic_rt_string_append_unsigned, chic_rt_string_as_slice,
    chic_rt_string_clone, chic_rt_string_clone_slice, chic_rt_string_push_slice,
    chic_rt_string_reserve, chic_rt_string_truncate,
};
use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{ChicVecView, chic_rt_vec_view};
use chic::runtime::{
    RuntimeTypeMetadata, TypeMetadataEntry, TypeMetadataStatus, VarianceSlice,
    chic_rt_install_type_metadata, chic_rt_object_new, chic_rt_type_align, chic_rt_type_clone_glue,
    chic_rt_type_drop_glue, chic_rt_type_metadata, chic_rt_type_metadata_clear, chic_rt_type_size,
};
use chic::type_metadata::TypeFlags;

static DROP_TEST_GUARD: Mutex<()> = Mutex::new(());

fn runtime_abi_coverage_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        if !cfg!(target_os = "macos") {
            return true;
        }
        if std::env::var_os("CHIC_ENABLE_NATIVE_RUNTIME_ABI_COVERAGE").is_some() {
            return true;
        }
        eprintln!(
            "skipping runtime ABI coverage on macOS (set CHIC_ENABLE_NATIVE_RUNTIME_ABI_COVERAGE=1 to run)"
        );
        false
    })
}

fn managed_string_contents(s: &ManagedString) -> String {
    unsafe { s.as_rust_str().to_string() }
}

fn empty_mut_span() -> ChicSpan {
    ChicSpan {
        data: ValueMutPtr {
            ptr: ptr::null_mut(),
            size: 0,
            align: 0,
        },
        len: 0,
        elem_size: 0,
        elem_align: 0,
    }
}

fn empty_readonly_span() -> ChicReadOnlySpan {
    ChicReadOnlySpan {
        data: ValueConstPtr {
            ptr: ptr::null(),
            size: 0,
            align: 0,
        },
        len: 0,
        elem_size: 0,
        elem_align: 0,
    }
}

#[test]
fn drop_glue_exports_behave() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let _guard = DROP_TEST_GUARD.lock().unwrap();
    unsafe {
        chic_rt_drop_missing(ptr::null_mut());
        __drop_noop(ptr::null_mut());
        chic_rt_drop_clear();
        let missing = chic_rt_drop_missing as chic::runtime::drop_glue::DropGlueFn;
        assert!(
            matches!(chic_rt_drop_resolve(0xDEADBEEF), Some(func) if func as usize == missing as usize),
            "missing drop glue should resolve to the runtime sentinel"
        );
        chic_rt_drop_register(0xDEADBEEF, Some(__drop_noop));
        assert!(chic_rt_drop_resolve(0xDEADBEEF).is_some());
        chic_rt_drop_register(0xDEADBEEF, None);
        assert!(chic_rt_drop_resolve(0xDEADBEEF).is_none());
    }
}

#[test]
fn drop_table_installer_registers_entries() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let _guard = DROP_TEST_GUARD.lock().unwrap();
    unsafe { chic_rt_drop_clear() };
    let entries = [DropGlueEntry {
        type_id: 0xC0FF_EE00,
        func: __drop_noop,
    }];
    unsafe {
        chic_rt_install_drop_table(entries.as_ptr(), entries.len());
        assert!(
            chic_rt_drop_resolve(0xC0FF_EE00).is_some(),
            "drop table installer should expose registered entries"
        );
        chic_rt_drop_clear();
    }
}

#[test]
fn type_metadata_installer_registers_entries() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    unsafe { chic_rt_type_metadata_clear() };
    let entries = [TypeMetadataEntry {
        type_id: 0xABCD_EF01,
        size: 32,
        align: 16,
        drop_fn: __drop_noop as *const () as usize as isize,
        variance: VarianceSlice::EMPTY,
        flags: TypeFlags::empty().bits(),
    }];
    unsafe { chic_rt_install_type_metadata(entries.as_ptr(), entries.len()) };
    let mut record = RuntimeTypeMetadata {
        size: 0,
        align: 0,
        drop_fn: 0,
        variance: VarianceSlice::EMPTY,
        flags: 0,
    };
    let status = unsafe { chic_rt_type_metadata(0xABCD_EF01, &mut record) };
    assert_eq!(status, TypeMetadataStatus::Success as i32);
    assert_eq!(record.size, 32);
    assert_eq!(record.align, 16);
    assert_ne!(record.drop_fn, 0, "drop glue should be recorded");
    assert_eq!(TypeFlags::from_bits(record.flags), TypeFlags::empty());
    assert_eq!(unsafe { chic_rt_type_size(0xABCD_EF01) }, 32);
    assert_eq!(unsafe { chic_rt_type_align(0xABCD_EF01) }, 16);
    assert_ne!(unsafe { chic_rt_type_drop_glue(0xABCD_EF01) }, 0);
    assert_eq!(
        unsafe { chic_rt_type_clone_glue(0xABCD_EF01) },
        0,
        "clone glue defaults to null until Clone derives land"
    );
    assert_eq!(unsafe { chic_rt_type_size(0xDEAD) }, 0);
    assert_eq!(unsafe { chic_rt_type_align(0xDEAD) }, 0);
    assert_eq!(unsafe { chic_rt_type_drop_glue(0xDEAD) }, 0);
    assert_eq!(unsafe { chic_rt_type_clone_glue(0xDEAD) }, 0);
    unsafe { chic_rt_type_metadata_clear() };
}

#[test]
fn object_allocation_uses_type_metadata() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    unsafe { chic_rt_type_metadata_clear() };
    let entries = [TypeMetadataEntry {
        type_id: 0xBEEF,
        size: 24,
        align: 8,
        drop_fn: 0,
        variance: VarianceSlice::EMPTY,
        flags: TypeFlags::empty().bits(),
    }];
    unsafe { chic_rt_install_type_metadata(entries.as_ptr(), entries.len()) };
    let ptr = unsafe { chic_rt_object_new(0xBEEF) };
    assert_eq!(unsafe { chic_rt_type_size(0xBEEF) }, 24);
    assert_eq!(unsafe { chic_rt_type_align(0xBEEF) }, 8);
    assert!(
        !ptr.is_null(),
        "object allocation should succeed when metadata is registered"
    );
    unsafe {
        let bytes = std::slice::from_raw_parts(ptr, 24);
        assert!(
            bytes.iter().all(|&b| b == 0),
            "allocated objects must be zero initialised"
        );
        std::alloc::dealloc(ptr, Layout::from_size_align(24, 8).unwrap());
    }
    unsafe { chic_rt_type_metadata_clear() };
}

#[test]
fn object_allocation_without_metadata_returns_null() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    unsafe { chic_rt_type_metadata_clear() };
    let ptr = unsafe { chic_rt_object_new(0xFEED) };
    assert!(
        ptr.is_null(),
        "allocator should return null when metadata is missing"
    );
}

#[test]
fn interface_default_installer_records_bindings() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    extern "C" fn default_draw() {}
    let implementer = std::ffi::CString::new("Demo::Widget").unwrap();
    let interface = std::ffi::CString::new("Demo::IRenderable").unwrap();
    let method = std::ffi::CString::new("Draw").unwrap();
    let entries = [InterfaceDefaultDescriptor {
        implementer: implementer.as_ptr() as *const u8,
        interface: interface.as_ptr() as *const u8,
        method: method.as_ptr() as *const u8,
        symbol: default_draw as *const (),
    }];
    unsafe { chic_rt_install_interface_defaults(entries.as_ptr(), entries.len() as u64) };
    let ptr = unsafe { chic_rt_interface_defaults_ptr() };
    let len = unsafe { chic_rt_interface_defaults_len() } as usize;
    let snapshot = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert_eq!(
        snapshot.len(),
        1,
        "interface defaults should record entries"
    );
    let record = &snapshot[0];
    let implementer = unsafe { CStr::from_ptr(record.implementer.cast()) }
        .to_str()
        .unwrap();
    let interface = unsafe { CStr::from_ptr(record.interface.cast()) }
        .to_str()
        .unwrap();
    let method = unsafe { CStr::from_ptr(record.method.cast()) }
        .to_str()
        .unwrap();
    assert_eq!(implementer, "Demo::Widget");
    assert_eq!(interface, "Demo::IRenderable");
    assert_eq!(method, "Draw");
    assert_ne!(record.symbol, ptr::null());
}

#[test]
fn string_construction_and_clone_paths() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let primary = ManagedString::from_str("hello");
    let mut dest = ManagedString::new();

    let status = unsafe { chic_rt_string_clone(dest.as_mut_ptr(), primary.as_raw()) };
    assert_eq!(status, StringError::Success as i32);
    assert_eq!(managed_string_contents(&dest), "hello");

    let status = unsafe { chic_rt_string_clone(ptr::null_mut(), primary.as_raw()) };
    assert_eq!(status, StringError::InvalidPointer as i32);

    let slice_status =
        unsafe { chic_rt_string_clone_slice(dest.as_mut_ptr(), str_to_chic(" world")) };
    assert_eq!(slice_status, StringError::Success as i32);
    assert_eq!(managed_string_contents(&dest), " world");

    let mut capacity = ManagedString::with_capacity(32);
    let reserve = unsafe { chic_rt_string_reserve(capacity.as_mut_ptr(), 48) };
    assert_eq!(reserve, StringError::Success as i32);

    let push = unsafe { chic_rt_string_push_slice(capacity.as_mut_ptr(), str_to_chic("héllo")) };
    assert_eq!(push, StringError::Success as i32);

    let truncate_invalid = unsafe { chic_rt_string_truncate(capacity.as_mut_ptr(), 2) };
    assert_eq!(truncate_invalid, StringError::Utf8 as i32);
    let truncate_valid = unsafe { chic_rt_string_truncate(capacity.as_mut_ptr(), "hé".len()) };
    assert_eq!(truncate_valid, 0);

    let view = unsafe { chic_rt_string_as_slice(primary.as_raw()) };
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    assert_eq!(bytes, b"hello");
}

fn decimal_parts(value: &str) -> Decimal128Parts {
    to_parts(Decimal128::parse_literal(value).expect("parse decimal literal"))
}

fn to_parts(value: Decimal128) -> Decimal128Parts {
    let [lo, mid, hi, flags] = value.to_bits();
    Decimal128Parts { lo, mid, hi, flags }
}

fn to_decimal(parts: Decimal128Parts) -> Decimal128 {
    Decimal128::from_bits([parts.lo, parts.mid, parts.hi, parts.flags])
}

fn decimal_string(parts: Decimal128Parts) -> String {
    to_decimal(parts).into_decimal().to_string()
}

fn rounding(mode: DecimalRoundingMode) -> DecimalRoundingAbi {
    DecimalRoundingAbi {
        value: mode.as_discriminant(),
    }
}

fn const_parts_ptr(parts: &Decimal128Parts) -> DecimalConstPtr {
    DecimalConstPtr {
        ptr: parts as *const _,
    }
}

fn mut_parts_ptr(parts: &mut Decimal128Parts) -> DecimalMutPtr {
    DecimalMutPtr {
        ptr: parts as *mut _,
    }
}

#[test]
fn decimal_runtime_scalar_and_simd_variants_succeed() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let lhs = decimal_parts("1.25");
    let rhs = decimal_parts("2.75");

    let scalar_result =
        unsafe { chic_rt_decimal_add(&lhs, &rhs, rounding(DecimalRoundingMode::TiesToEven), 0) };
    let mut out = std::mem::MaybeUninit::<chic::runtime::DecimalRuntimeResult>::uninit();
    unsafe {
        chic::runtime::decimal::chic_rt_decimal_add_out(
            out.as_mut_ptr(),
            &lhs,
            &rhs,
            rounding(DecimalRoundingMode::TiesToEven),
            0,
        );
    }
    let out = unsafe { out.assume_init() };
    assert_eq!(scalar_result.status, DecimalRuntimeStatus::Success);
    assert_eq!(decimal_string(scalar_result.value), "4");
    assert_eq!(out.status, DecimalRuntimeStatus::Success);
    assert_eq!(decimal_string(out.value), "4");

    let div_result =
        unsafe { chic_rt_decimal_div(&rhs, &lhs, rounding(DecimalRoundingMode::TiesToEven), 0) };
    assert_eq!(div_result.status, DecimalRuntimeStatus::Success);
    assert_eq!(decimal_string(div_result.value), "2.2");
}

#[test]
fn decimal_runtime_reports_invalid_inputs() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let lhs = decimal_parts("1");
    let rhs = decimal_parts("0");

    let divide_zero =
        unsafe { chic_rt_decimal_div(&lhs, &rhs, rounding(DecimalRoundingMode::TiesToEven), 0) };
    assert_eq!(divide_zero.status, DecimalRuntimeStatus::DivideByZero);

    let invalid_rounding =
        unsafe { chic_rt_decimal_add(&lhs, &lhs, DecimalRoundingAbi { value: 99 }, 0) };
    assert_eq!(
        invalid_rounding.status,
        DecimalRuntimeStatus::InvalidRounding
    );

    let null_result = unsafe {
        chic_rt_decimal_add(
            std::ptr::null(),
            std::ptr::null(),
            rounding(DecimalRoundingMode::TiesToEven),
            0,
        )
    };
    assert_eq!(null_result.status, DecimalRuntimeStatus::InvalidPointer);
}

#[test]
fn decimal_runtime_clone_copies_parts() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let value = decimal_parts("9.5");
    let mut dest = decimal_parts("0");
    let status =
        unsafe { chic_rt_decimal_clone(const_parts_ptr(&value), mut_parts_ptr(&mut dest)) };
    assert_eq!(status, DecimalRuntimeStatus::Success);
    assert_eq!(decimal_string(dest), "9.5");

    let invalid_status = unsafe {
        chic_rt_decimal_clone(
            DecimalConstPtr {
                ptr: std::ptr::null(),
            },
            mut_parts_ptr(&mut dest),
        )
    };
    assert_eq!(invalid_status, DecimalRuntimeStatus::InvalidPointer);
}

#[test]
fn decimal_intrinsic_table_includes_only_scalar_entries() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let mut scalar = 0usize;
    for entry in DECIMAL_INTRINSICS {
        match entry.variant {
            DecimalIntrinsicVariant::Scalar => scalar += 1,
        }
    }
    assert_eq!(scalar, 6, "expected six scalar decimal intrinsics");
}

#[test]
fn string_append_variants_cover_alignment_and_formats() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let mut builder = ManagedString::new();

    let (signed_low, signed_high) = {
        let value: i128 = -42;
        ((value as u128 as u64) as i64, (value >> 64) as i64)
    };
    let unsigned_low: u64 = 0xBEEF;
    let unsigned_high: u64 = 0;
    let (invalid_low, invalid_high) = {
        let value: i128 = 7;
        ((value as u128 as u64) as i64, (value >> 64) as i64)
    };

    unsafe {
        let align_slice =
            chic_rt_string_append_slice(builder.as_mut_ptr(), str_to_chic("aligned"), 10, 1);
        assert_eq!(align_slice, StringError::Success as i32);

        let append_bool =
            chic_rt_string_append_bool(builder.as_mut_ptr(), false, 8, 1, str_to_chic("u"));
        assert_eq!(append_bool, StringError::Success as i32);

        let append_char =
            chic_rt_string_append_char(builder.as_mut_ptr(), 'ß' as u16, 0, 0, ChicStr::empty());
        assert_eq!(append_char, StringError::Success as i32);

        let append_signed = chic_rt_string_append_signed(
            builder.as_mut_ptr(),
            signed_low,
            signed_high,
            16,
            0,
            0,
            str_to_chic("x4"),
        );
        assert_eq!(append_signed, StringError::Success as i32);

        let append_unsigned = chic_rt_string_append_unsigned(
            builder.as_mut_ptr(),
            unsigned_low,
            unsigned_high,
            16,
            0,
            0,
            str_to_chic("X6"),
        );
        assert_eq!(append_unsigned, StringError::Success as i32);

        let append_f32 =
            chic_rt_string_append_f32(builder.as_mut_ptr(), 3.5f32, 0, 0, str_to_chic("f2"));
        assert_eq!(append_f32, StringError::Success as i32);

        let append_f64 =
            chic_rt_string_append_f64(builder.as_mut_ptr(), -1.25f64, 0, 0, str_to_chic("e3"));
        assert_eq!(append_f64, StringError::Success as i32);

        // Unknown format tokens are accepted (runtime defaults to decimal formatting).
        static INVALID_SPEC: [u8; 1] = [0xFF];
        let invalid = chic_rt_string_append_signed(
            builder.as_mut_ptr(),
            invalid_low,
            invalid_high,
            32,
            0,
            0,
            bytes_to_chic(&INVALID_SPEC),
        );
        assert_eq!(invalid, StringError::Success as i32);
    }

    let snapshot = managed_string_contents(&builder);
    assert!(snapshot.contains("aligned"));
    assert!(snapshot.contains("FALSE"));
    assert!(snapshot.contains("ß"));
    assert!(snapshot.contains("ffd6"));
    assert!(snapshot.contains("00BEEF"));
    assert!(snapshot.contains("3.50"));
    assert!(snapshot.contains("-1.250"));
}

#[test]
fn span_construction_and_slicing() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let mut data = vec![1u32, 2, 3, 4];
    let span = unsafe {
        chic_rt_span_from_raw_mut(
            ValueMutPtr {
                ptr: data.as_mut_ptr().cast(),
                size: size_of::<u32>(),
                align: align_of::<u32>(),
            },
            data.len(),
        )
    };
    assert_eq!(span.len, 4);

    let mut sliced = empty_mut_span();
    let slice_status = unsafe { chic_rt_span_slice_mut(&span, 1, 2, &mut sliced) };
    assert_eq!(slice_status, SpanError::Success as i32);
    let slice_view =
        unsafe { slice::from_raw_parts_mut(sliced.data.ptr.cast::<u32>(), sliced.len) };
    assert_eq!(slice_view, &[2, 3]);

    // Pointer access modifies backing storage.
    let ptr = unsafe { elem_ptr_at_mut(&span, 2) } as *mut u32;
    unsafe { *ptr = 99 };
    assert_eq!(data[2], 99);

    let readonly = unsafe { chic_rt_span_to_readonly(&span) };
    let ptr_ro = unsafe { elem_ptr_at_readonly(&readonly, 1) } as *const u32;
    assert_eq!(unsafe { *ptr_ro }, 2);

    let mut readonly_slice = empty_readonly_span();
    let ro_status = unsafe { chic_rt_span_slice_readonly(&readonly, 0, 3, &mut readonly_slice) };
    assert_eq!(ro_status, SpanError::Success as i32);
    let slice_ro = unsafe { slice::from_raw_parts(readonly_slice.data.ptr.cast::<u32>(), 3) };
    assert_eq!(slice_ro, &[1, 2, 99]);

    // Copy and fill operations.
    let mut dest = vec![0u32; 4];
    let dest_span = ChicSpan {
        data: ValueMutPtr {
            ptr: dest.as_mut_ptr().cast::<u8>(),
            size: size_of::<u32>(),
            align: align_of::<u32>(),
        },
        len: dest.len(),
        elem_size: size_of::<u32>(),
        elem_align: align_of::<u32>(),
    };
    let copy_status = unsafe { chic_rt_span_copy_to(&readonly, &dest_span) };
    assert_eq!(copy_status, SpanError::Success as i32);
    assert_eq!(dest, vec![1, 2, 99, 4]);

    let fill_status = unsafe {
        let value = 0xA5A5u16;
        let span = ChicSpan {
            data: ValueMutPtr {
                ptr: dest.as_mut_ptr().cast::<u8>(),
                size: size_of::<u16>(),
                align: align_of::<u16>(),
            },
            len: dest.len(),
            elem_size: size_of::<u16>(),
            elem_align: align_of::<u16>(),
        };
        chic_rt_span_fill(&span, (&value as *const u16).cast())
    };
    assert_eq!(fill_status, SpanError::Success as i32);
}

unsafe fn elem_ptr_at_readonly(span: &ChicReadOnlySpan, index: usize) -> *const u8 {
    assert!(index < span.len);
    unsafe { span.data.ptr.add(index * span.elem_size) }
}

unsafe fn elem_ptr_at_mut(span: &ChicSpan, index: usize) -> *mut u8 {
    assert!(index < span.len);
    unsafe { span.data.ptr.add(index * span.elem_size) }
}

#[test]
fn span_from_runtime_containers() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let mut managed_vec = ManagedVec::<u64>::new();
    managed_vec.push(10);
    managed_vec.push(20);
    assert!(!managed_vec.as_mut_ptr().is_null());

    let span =
        unsafe { chic_rt_span_from_raw_mut(managed_vec.as_value_mut_ptr(), managed_vec.len()) };
    assert_eq!(span.len, 2);

    let view = unsafe { chic_rt_vec_view(managed_vec.as_ptr()) };
    let ro_from_view = unsafe { chic_rt_span_from_raw_const(vec_view_handle(view), view.len) };
    assert_eq!(ro_from_view.len, 2);

    let array_span =
        unsafe { chic_rt_span_from_raw_const(managed_vec.as_value_const_ptr(), managed_vec.len()) };
    assert_eq!(array_span.len, 2);

    let string = ManagedString::from_str("xyz");
    let string_view = unsafe { chic_rt_string_as_slice(string.as_raw()) };
    let str_span = unsafe {
        chic_rt_span_from_raw_const(
            ValueConstPtr {
                ptr: string_view.ptr.cast::<u8>(),
                size: 1,
                align: 1,
            },
            string_view.len,
        )
    };
    assert_eq!(str_span.len, 3);

    let literal = str_to_chic("hi");
    let literal_span = unsafe {
        chic_rt_span_from_raw_const(
            ValueConstPtr {
                ptr: literal.ptr.cast::<u8>(),
                size: 1,
                align: 1,
            },
            literal.len,
        )
    };
    assert_eq!(literal_span.len, 2);

    // Null inputs should yield empty spans.
    let null_vec = unsafe {
        chic_rt_span_from_raw_mut(
            ValueMutPtr {
                ptr: ptr::null_mut(),
                size: size_of::<u64>(),
                align: align_of::<u64>(),
            },
            0,
        )
    };
    assert_eq!(null_vec.len, 0);
    let null_array = unsafe {
        chic_rt_span_from_raw_const(
            ValueConstPtr {
                ptr: ptr::null(),
                size: size_of::<u64>(),
                align: align_of::<u64>(),
            },
            0,
        )
    };
    assert_eq!(null_array.len, 0);
    let null_string = unsafe {
        chic_rt_span_from_raw_const(
            ValueConstPtr {
                ptr: ptr::null(),
                size: 1,
                align: 1,
            },
            0,
        )
    };
    assert_eq!(null_string.len, 0);
}

fn vec_view_handle(view: ChicVecView) -> ValueConstPtr {
    let ptr = if view.elem_size == 0 {
        std::ptr::NonNull::<u8>::dangling().as_ptr()
    } else {
        view.data
    };
    ValueConstPtr {
        ptr,
        size: view.elem_size,
        align: view.elem_align,
    }
}

#[test]
fn startup_descriptor_helpers_cover_branches() {
    if !runtime_abi_coverage_enabled() {
        return;
    }
    let descriptor = StartupDescriptor::empty();
    assert_eq!(descriptor.version, STARTUP_DESCRIPTOR_VERSION);
    assert!(descriptor.tests.is_empty());

    let entry = EntryDescriptor {
        function: ptr::null(),
        flags: ENTRY_FLAG_ASYNC | ENTRY_FLAG_PARAM_ARGS,
        reserved: 0,
    };
    assert!(entry.has_flag(ENTRY_FLAG_ASYNC));
    assert!(!entry.has_flag(ENTRY_FLAG_PARAM_ARGS << 1));

    let testcase_name = b"demo";
    let testcase = TestCaseDescriptor {
        function: ptr::null(),
        name_ptr: testcase_name.as_ptr(),
        name_len: testcase_name.len(),
        flags: TESTCASE_FLAG_ASYNC,
        reserved: 0,
    };
    assert_eq!(unsafe { testcase.name() }, Some("demo"));
    assert!(testcase.is_async());

    let suite = TestSuiteDescriptor {
        cases: &testcase,
        len: 1,
    };
    assert!(!suite.is_empty());
    let cases = unsafe { suite.as_slice() };
    assert_eq!(cases.len(), 1);
}
