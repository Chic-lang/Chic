#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; link the Chic-native runtime."
);

mod native;
mod shim_types;
pub use native::{
    chic_rt_array_copy_to_vec, chic_rt_array_data, chic_rt_array_into_vec, chic_rt_array_is_empty,
    chic_rt_array_len, chic_rt_array_ptr_at, chic_rt_array_view, chic_rt_vec_capacity,
    chic_rt_vec_clear, chic_rt_vec_clone, chic_rt_vec_copy_to_array, chic_rt_vec_data,
    chic_rt_vec_data_mut, chic_rt_vec_drop, chic_rt_vec_elem_align, chic_rt_vec_elem_size,
    chic_rt_vec_get_drop, chic_rt_vec_get_ptr, chic_rt_vec_inline_capacity, chic_rt_vec_inline_ptr,
    chic_rt_vec_insert, chic_rt_vec_into_array, chic_rt_vec_is_empty, chic_rt_vec_iter,
    chic_rt_vec_iter_next, chic_rt_vec_iter_next_ptr, chic_rt_vec_len, chic_rt_vec_mark_inline,
    chic_rt_vec_new, chic_rt_vec_new_in_region, chic_rt_vec_pop, chic_rt_vec_ptr_at,
    chic_rt_vec_push, chic_rt_vec_remove, chic_rt_vec_reserve, chic_rt_vec_set_cap,
    chic_rt_vec_set_drop, chic_rt_vec_set_elem_align, chic_rt_vec_set_elem_size,
    chic_rt_vec_set_len, chic_rt_vec_set_ptr, chic_rt_vec_shrink_to_fit, chic_rt_vec_swap_remove,
    chic_rt_vec_truncate, chic_rt_vec_uses_inline, chic_rt_vec_view, chic_rt_vec_with_capacity,
    chic_rt_vec_with_capacity_in_region,
};

pub use shim_types::{ChicVec, ChicVecIter, ChicVecView, VecDropFn, VecError};

#[cfg(test)]
mod tests;
