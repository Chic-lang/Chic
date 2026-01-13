#[path = "support/collections_runtime_fns.rs"]
mod collections_runtime_fns;
#[path = "support/collections_runtime_types.rs"]
mod collections_runtime_types;
#[path = "support/runtime_value_ptr.rs"]
mod runtime_value_ptr;

use chic::runtime::{
    VecError, chic_rt_vec_drop, chic_rt_vec_insert, chic_rt_vec_len, chic_rt_vec_new,
    chic_rt_vec_pop, chic_rt_vec_push,
};
use collections_runtime_fns::{
    chic_rt_hashmap_contains, chic_rt_hashmap_drop, chic_rt_hashmap_insert, chic_rt_hashmap_len,
    chic_rt_hashmap_new, chic_rt_hashmap_remove, chic_rt_hashmap_reserve, chic_rt_hashmap_take,
    chic_rt_hashset_contains, chic_rt_hashset_drop, chic_rt_hashset_insert, chic_rt_hashset_len,
    chic_rt_hashset_new, chic_rt_hashset_remove, chic_rt_hashset_reserve,
};
use collections_runtime_types::{HashMapError, HashSetError};
use runtime_value_ptr::{value_const_ptr_from, value_mut_ptr_from};
use std::sync::atomic::{AtomicUsize, Ordering};

static EQ_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn eq_i32(left: *const u8, right: *const u8) -> i32 {
    let _ = EQ_CALLS.fetch_add(1, Ordering::SeqCst);
    if left.is_null() || right.is_null() {
        return 0;
    }
    let lhs = unsafe { *(left as *const i32) };
    let rhs = unsafe { *(right as *const i32) };
    i32::from(lhs == rhs)
}

#[test]
fn vec_runtime_helpers_succeed() {
    unsafe {
        let mut vec = chic_rt_vec_new(
            core::mem::size_of::<i32>(),
            core::mem::size_of::<i32>(),
            None,
        );

        let value = 5;
        let push_value = value_const_ptr_from(&value);
        assert_eq!(
            chic_rt_vec_push(&mut vec, push_value),
            VecError::Success as i32
        );
        assert_eq!(chic_rt_vec_len(&vec), 1);

        let mut popped = 0;
        let pop_dest = value_mut_ptr_from(&mut popped);
        assert_eq!(
            chic_rt_vec_pop(&mut vec, pop_dest),
            VecError::Success as i32
        );
        assert_eq!(popped, 5);

        chic_rt_vec_drop(&mut vec);
    }
}

#[test]
fn vec_insert_out_of_bounds_reports_error() {
    unsafe {
        let mut vec = chic_rt_vec_new(
            core::mem::size_of::<i32>(),
            core::mem::size_of::<i32>(),
            None,
        );
        let value = 42;
        let insert_value = value_const_ptr_from(&value);
        assert_eq!(
            chic_rt_vec_insert(&mut vec, 1, insert_value),
            VecError::OutOfBounds as i32
        );
        chic_rt_vec_drop(&mut vec);
    }
}

#[test]
fn hashset_runtime_helpers_succeed() {
    unsafe {
        EQ_CALLS.store(0, Ordering::SeqCst);
        let mut set = chic_rt_hashset_new(
            core::mem::size_of::<i32>(),
            core::mem::align_of::<i32>(),
            None,
            Some(eq_i32),
        );
        assert_eq!(
            chic_rt_hashset_reserve(&mut set, 1),
            HashSetError::Success as i32
        );

        let value = 11;
        let value_ptr = value_const_ptr_from(&value);
        let mut inserted = 0;
        assert_eq!(
            chic_rt_hashset_insert(&mut set, value as u64, &value_ptr, &mut inserted,),
            HashSetError::Success as i32
        );
        assert_eq!(inserted, 1);
        assert_eq!(chic_rt_hashset_len(&set), 1);

        assert_eq!(chic_rt_hashset_contains(&set, value as u64, &value_ptr), 1);

        assert_eq!(
            chic_rt_hashset_remove(&mut set, value as u64, &value_ptr,),
            1
        );
        assert_eq!(chic_rt_hashset_len(&set), 0);
        assert!(EQ_CALLS.load(Ordering::SeqCst) > 0);

        chic_rt_hashset_drop(&mut set);
    }
}

#[test]
fn hashmap_runtime_helpers_succeed() {
    unsafe {
        EQ_CALLS.store(0, Ordering::SeqCst);
        let mut map = chic_rt_hashmap_new(
            core::mem::size_of::<i32>(),
            core::mem::align_of::<i32>(),
            core::mem::size_of::<i32>(),
            core::mem::align_of::<i32>(),
            None,
            None,
            Some(eq_i32),
        );
        let status = chic_rt_hashmap_reserve(&mut map, 1);
        assert_eq!(
            status,
            HashMapError::Success as i32,
            "hashmap_reserve failed status={status} len={} cap={} tombstones={}",
            map.len,
            map.cap,
            map.tombstones
        );

        let key = 7;
        let value = 13;
        let key_ptr = value_const_ptr_from(&key);
        let value_ptr = value_const_ptr_from(&value);
        let mut replaced = 0;
        let mut prev = 0;
        let prev_slot = value_mut_ptr_from(&mut prev);
        assert_eq!(
            chic_rt_hashmap_insert(
                &mut map,
                key as u64,
                &key_ptr,
                &value_ptr,
                &prev_slot,
                &mut replaced,
            ),
            HashMapError::Success as i32
        );
        assert_eq!(replaced, 0);
        assert_eq!(chic_rt_hashmap_len(&map), 1);
        assert_eq!(chic_rt_hashmap_contains(&map, key as u64, &key_ptr), 1);

        let mut taken = 0;
        let taken_ptr = value_mut_ptr_from(&mut taken);
        assert_eq!(
            chic_rt_hashmap_take(&mut map, key as u64, &key_ptr, &taken_ptr,),
            HashMapError::Success as i32
        );
        assert_eq!(taken, value);
        assert_eq!(chic_rt_hashmap_len(&map), 0);

        let new_value = 21;
        let new_value_ptr = value_const_ptr_from(&new_value);
        let _ = chic_rt_hashmap_insert(
            &mut map,
            key as u64,
            &key_ptr,
            &new_value_ptr,
            &prev_slot,
            &mut replaced,
        );
        let mut prev_out = 0;
        let prev_out_ptr = value_mut_ptr_from(&mut prev_out);
        let status_replace = chic_rt_hashmap_insert(
            &mut map,
            key as u64,
            &key_ptr,
            &value_ptr,
            &prev_out_ptr,
            &mut replaced,
        );
        assert_eq!(status_replace, HashMapError::Success as i32);
        assert_eq!(replaced, 1);
        assert_eq!(prev_out, new_value);

        assert_eq!(chic_rt_hashmap_remove(&mut map, key as u64, &key_ptr), 1);
        assert_eq!(chic_rt_hashmap_len(&map), 0);
        assert!(EQ_CALLS.load(Ordering::SeqCst) > 0);

        chic_rt_hashmap_drop(&mut map);
    }
}
