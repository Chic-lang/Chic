pub use crate::runtime::character::{
    CharError, chic_rt_char_from_codepoint, chic_rt_char_is_digit, chic_rt_char_is_letter,
    chic_rt_char_is_scalar, chic_rt_char_is_whitespace, chic_rt_char_status, chic_rt_char_to_lower,
    chic_rt_char_to_upper, chic_rt_char_value,
};
pub use crate::runtime::clone::chic_rt_clone_invoke;
pub use crate::runtime::closure::{
    chic_rt_closure_env_alloc, chic_rt_closure_env_clone, chic_rt_closure_env_free,
};
pub use crate::runtime::decimal::{
    DECIMAL_INTRINSICS, Decimal128Parts, DecimalConstPtr, DecimalIntrinsicEntry,
    DecimalIntrinsicVariant, DecimalMutPtr, DecimalRoundingAbi, DecimalRuntimeResult,
    DecimalRuntimeStatus, chic_rt_decimal_add, chic_rt_decimal_clone, chic_rt_decimal_div,
    chic_rt_decimal_dot, chic_rt_decimal_fma, chic_rt_decimal_matmul, chic_rt_decimal_mul,
    chic_rt_decimal_rem, chic_rt_decimal_sub, chic_rt_decimal_sum,
};
pub use crate::runtime::drop_glue::{
    __drop_noop, chic_rt_drop_clear, chic_rt_drop_invoke, chic_rt_drop_missing,
    chic_rt_drop_register, chic_rt_drop_resolve, chic_rt_install_drop_table,
};
pub use crate::runtime::eq_glue::{
    EqGlueEntry, EqGlueFn, chic_rt_eq_clear, chic_rt_eq_invoke, chic_rt_eq_register,
    chic_rt_eq_resolve, chic_rt_install_eq_table,
};
pub use crate::runtime::error::{RuntimeThrownException, chic_rt_throw, exception_type_identity};
pub use crate::runtime::ffi::ChicFfiDescriptor;
pub use crate::runtime::flags::{
    FlagIter, FlagMapping, FlagParseError, combine as combine_flags, contains_all, contains_any,
    format_flags, iter_flags, parse_flags,
};
pub use crate::runtime::float_env::{
    chic_rt_float_flags_clear, chic_rt_float_flags_read, chic_rt_rounding_mode,
    chic_rt_set_rounding_mode,
};
pub use crate::runtime::float_ops::{chic_rt_f32_rem, chic_rt_f64_rem};
pub use crate::runtime::hash_glue::{
    HashGlueEntry, HashGlueFn, chic_rt_hash_clear, chic_rt_hash_invoke, chic_rt_hash_register,
    chic_rt_hash_resolve, chic_rt_install_hash_table,
};
pub use crate::runtime::int128::{
    Int128Parts, UInt128Parts, chic_rt_i128_add, chic_rt_i128_and, chic_rt_i128_cmp,
    chic_rt_i128_div, chic_rt_i128_eq, chic_rt_i128_mul, chic_rt_i128_neg, chic_rt_i128_not,
    chic_rt_i128_or, chic_rt_i128_rem, chic_rt_i128_shl, chic_rt_i128_shr, chic_rt_i128_sub,
    chic_rt_i128_xor, chic_rt_u128_add, chic_rt_u128_and, chic_rt_u128_cmp, chic_rt_u128_div,
    chic_rt_u128_eq, chic_rt_u128_mul, chic_rt_u128_not, chic_rt_u128_or, chic_rt_u128_rem,
    chic_rt_u128_shl, chic_rt_u128_shr, chic_rt_u128_sub, chic_rt_u128_xor,
};
pub use crate::runtime::interface_defaults::{
    InterfaceDefaultDescriptor, InterfaceDefaultRecord, chic_rt_install_interface_defaults,
    chic_rt_interface_defaults_len, chic_rt_interface_defaults_ptr,
};
pub use crate::runtime::memory::{
    chic_rt_alloc, chic_rt_alloc_zeroed, chic_rt_free, chic_rt_realloc,
};
pub use crate::runtime::numeric::{
    NUMERIC_INTRINSICS, NumericIntrinsicEntry, NumericIntrinsicKind, NumericWidth,
    numeric_intrinsics_with_pointer,
};
pub use crate::runtime::region::{
    RegionHandle, RegionTelemetry, chic_rt_region_alloc, chic_rt_region_alloc_zeroed,
    chic_rt_region_enter, chic_rt_region_exit, chic_rt_region_reset_stats,
    chic_rt_region_telemetry,
};
pub use crate::runtime::shared::{
    ChicArc, ChicRc, ChicWeak, ChicWeakRc, SharedError, chic_rt_arc_clone, chic_rt_arc_downgrade,
    chic_rt_arc_drop, chic_rt_arc_get, chic_rt_arc_get_mut, chic_rt_arc_new,
    chic_rt_arc_strong_count, chic_rt_arc_weak_count, chic_rt_object_new, chic_rt_rc_clone,
    chic_rt_rc_downgrade, chic_rt_rc_drop, chic_rt_rc_get, chic_rt_rc_get_mut, chic_rt_rc_new,
    chic_rt_rc_strong_count, chic_rt_rc_weak_count, chic_rt_weak_clone, chic_rt_weak_drop,
    chic_rt_weak_rc_clone, chic_rt_weak_rc_drop, chic_rt_weak_rc_upgrade, chic_rt_weak_upgrade,
};
pub use crate::runtime::span::{
    ChicReadOnlySpan, ChicSpan, SpanError, SpanLayoutInfo, chic_rt_span_copy_to, chic_rt_span_fill,
    chic_rt_span_from_raw_const, chic_rt_span_from_raw_mut, chic_rt_span_layout_debug,
    chic_rt_span_ptr_at_mut, chic_rt_span_ptr_at_readonly, chic_rt_span_slice_mut,
    chic_rt_span_slice_readonly, chic_rt_span_to_readonly,
};
pub use crate::runtime::string::{
    ChicCharSpan, ChicStr, ChicString, StringError, chic_rt_str_as_chars,
    chic_rt_string_append_bool, chic_rt_string_append_char, chic_rt_string_append_f16,
    chic_rt_string_append_f32, chic_rt_string_append_f64, chic_rt_string_append_f128,
    chic_rt_string_append_signed, chic_rt_string_append_slice, chic_rt_string_append_unsigned,
    chic_rt_string_as_chars, chic_rt_string_as_slice, chic_rt_string_clone,
    chic_rt_string_clone_slice, chic_rt_string_drop, chic_rt_string_error_message,
    chic_rt_string_from_char, chic_rt_string_from_slice, chic_rt_string_get_cap,
    chic_rt_string_get_len, chic_rt_string_get_ptr, chic_rt_string_inline_capacity,
    chic_rt_string_inline_ptr, chic_rt_string_new, chic_rt_string_push_slice,
    chic_rt_string_reserve, chic_rt_string_set_cap, chic_rt_string_set_len, chic_rt_string_set_ptr,
    chic_rt_string_truncate, chic_rt_string_with_capacity,
};
pub use crate::runtime::sync::{
    chic_rt_atomic_bool_compare_exchange, chic_rt_atomic_bool_load, chic_rt_atomic_bool_store,
    chic_rt_atomic_i32_compare_exchange, chic_rt_atomic_i32_fetch_add,
    chic_rt_atomic_i32_fetch_sub, chic_rt_atomic_i32_load, chic_rt_atomic_i32_store,
    chic_rt_atomic_i64_compare_exchange, chic_rt_atomic_i64_fetch_add,
    chic_rt_atomic_i64_fetch_sub, chic_rt_atomic_i64_load, chic_rt_atomic_i64_store,
    chic_rt_atomic_u32_compare_exchange, chic_rt_atomic_u32_fetch_add,
    chic_rt_atomic_u32_fetch_sub, chic_rt_atomic_u32_load, chic_rt_atomic_u32_store,
    chic_rt_atomic_u64_compare_exchange, chic_rt_atomic_u64_fetch_add,
    chic_rt_atomic_u64_fetch_sub, chic_rt_atomic_u64_load, chic_rt_atomic_u64_store,
    chic_rt_atomic_usize_fetch_add, chic_rt_atomic_usize_fetch_sub, chic_rt_atomic_usize_load,
    chic_rt_atomic_usize_store, chic_rt_condvar_create, chic_rt_condvar_destroy,
    chic_rt_condvar_notify_all, chic_rt_condvar_notify_one, chic_rt_condvar_wait,
    chic_rt_lock_create, chic_rt_lock_destroy, chic_rt_lock_enter, chic_rt_lock_exit,
    chic_rt_lock_is_held, chic_rt_lock_is_held_by_current_thread, chic_rt_lock_try_enter,
    chic_rt_mutex_create, chic_rt_mutex_destroy, chic_rt_mutex_lock, chic_rt_mutex_try_lock,
    chic_rt_mutex_unlock, chic_rt_once_complete, chic_rt_once_create, chic_rt_once_destroy,
    chic_rt_once_is_completed, chic_rt_once_try_begin, chic_rt_once_wait, chic_rt_rwlock_create,
    chic_rt_rwlock_destroy, chic_rt_rwlock_read_lock, chic_rt_rwlock_read_unlock,
    chic_rt_rwlock_try_read_lock, chic_rt_rwlock_try_write_lock, chic_rt_rwlock_write_lock,
    chic_rt_rwlock_write_unlock,
};
pub use crate::runtime::test_executor::{
    TestExecutionError, TestExecutor, execute_main, execute_tests,
};
pub use crate::runtime::thread::{
    ThreadHandle, ThreadStart, ThreadStatus, chic_rt_thread_detach, chic_rt_thread_join,
    chic_rt_thread_sleep_ms, chic_rt_thread_spawn, chic_rt_thread_spin_wait, chic_rt_thread_yield,
};
pub use crate::runtime::tracing::{chic_rt_trace_enter, chic_rt_trace_exit, chic_rt_trace_flush};
pub use crate::runtime::type_metadata::{
    RuntimeGenericVariance, RuntimeTypeMetadata, TypeMetadataEntry, TypeMetadataStatus,
    VarianceSlice, chic_rt_install_type_metadata, chic_rt_type_align, chic_rt_type_clone_glue,
    chic_rt_type_drop_glue, chic_rt_type_eq_glue, chic_rt_type_hash_glue, chic_rt_type_metadata,
    chic_rt_type_metadata_clear, chic_rt_type_metadata_register, chic_rt_type_size,
};
pub use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
pub use crate::runtime::vec::{
    ChicVec, ChicVecIter, ChicVecView, VecDropFn, VecError, chic_rt_array_copy_to_vec,
    chic_rt_array_data, chic_rt_array_into_vec, chic_rt_array_is_empty, chic_rt_array_len,
    chic_rt_array_ptr_at, chic_rt_array_view, chic_rt_vec_capacity, chic_rt_vec_clear,
    chic_rt_vec_clone, chic_rt_vec_copy_to_array, chic_rt_vec_data, chic_rt_vec_data_mut,
    chic_rt_vec_drop, chic_rt_vec_elem_align, chic_rt_vec_elem_size, chic_rt_vec_get_drop,
    chic_rt_vec_get_ptr, chic_rt_vec_inline_capacity, chic_rt_vec_inline_ptr, chic_rt_vec_insert,
    chic_rt_vec_into_array, chic_rt_vec_is_empty, chic_rt_vec_iter, chic_rt_vec_iter_next,
    chic_rt_vec_iter_next_ptr, chic_rt_vec_len, chic_rt_vec_mark_inline, chic_rt_vec_new,
    chic_rt_vec_new_in_region, chic_rt_vec_pop, chic_rt_vec_ptr_at, chic_rt_vec_push,
    chic_rt_vec_remove, chic_rt_vec_reserve, chic_rt_vec_set_cap, chic_rt_vec_set_drop,
    chic_rt_vec_set_elem_align, chic_rt_vec_set_elem_size, chic_rt_vec_set_len,
    chic_rt_vec_set_ptr, chic_rt_vec_shrink_to_fit, chic_rt_vec_swap_remove, chic_rt_vec_truncate,
    chic_rt_vec_uses_inline, chic_rt_vec_view, chic_rt_vec_with_capacity,
    chic_rt_vec_with_capacity_in_region,
};
pub use crate::runtime::wasm_executor::{
    WasmExecutionError, WasmExecutionOptions, WasmExecutionTrace, WasmProgram,
    WasmProgramExportOutcome, WasmRunOutcome, WasmValue, execute_wasm, execute_wasm_with_options,
    hooks,
};
