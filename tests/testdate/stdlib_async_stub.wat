(module
  (type (;0;) (func (param i32)))
  (type (;1;) (func (param i32) (result i32)))
  (type (;2;) (func (param i32) (param i32) (result i32)))
  (type (;3;) (func (param i32) (param i32) (param i32) (result i32)))
  (type (;4;) (func (result i32)))
  (type (;5;) (func (param i32) (param i64) (result i32)))
  (type (;6;) (func (param i64) (result i32)))
  (type (;7;) (func (param i32) (param i64)))
  (type (;8;) (func (param i32) (param i32)))
  (type (;9;) (func (param i64) (param i32) (param i64) (param i64) (param i64) (param i64)))
  (type (;10;) (func (param i64)))
  (type (;11;) (func (param i32) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (type (;12;) (func (param i32) (param i32) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (type (;13;) (func (param i32) (param i64) (param i64) (param i32) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (type (;14;) (func (param i32) (param f32) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (type (;15;) (func (param i32) (param f64) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (type (;16;) (func (param i32) (result i32) (result i32)))
  (type (;17;) (func (param i32) (param i32) (param i32) (param i32) (param i32) (param i32) (param i32) (param i32) (result i32)))
  (type (;18;) (func (param i64) (param i32) (param i32) (result i64)))
  (type (;19;) (func (param i64) (param i64) (param i32) (param i32)))
  (type (;20;) (func (param i32) (param i32) (param i32) (param i32) (param i32)))
  (type (;21;) (func (param i32) (param i32) (param i32) (param i32) (param i32) (param i32)))
  (type (;22;) (func (param i32) (param i32) (param i32)))
  (import "chic_rt" "object_new" (func $chic_rt__object_new (type 6)))
  (import "chic_rt" "panic" (func $chic_rt__panic (type 0)))
  (import "chic_rt" "abort" (func $chic_rt__abort (type 0)))
  (import "chic_rt" "throw" (func $chic_rt__throw (type 7)))
  (import "chic_rt" "await" (func $chic_rt__await (type 2)))
  (import "chic_rt" "yield" (func $chic_rt__yield (type 1)))
  (import "chic_rt" "async_cancel" (func $chic_rt__async_cancel (type 1)))
  (import "chic_rt" "async_spawn" (func $chic_rt__async_spawn (type 1)))
  (import "chic_rt" "async_spawn_local" (func $chic_rt__async_spawn_local (type 1)))
  (import "chic_rt" "async_scope" (func $chic_rt__async_scope (type 1)))
  (import "chic_rt" "async_task_header" (func $chic_rt__async_task_header (type 1)))
  (import "chic_rt" "async_task_result" (func $chic_rt__async_task_result (type 3)))
  (import "chic_rt" "async_token_state" (func $chic_rt__async_token_state (type 1)))
  (import "chic_rt" "async_token_new" (func $chic_rt__async_token_new (type 4)))
  (import "chic_rt" "async_token_cancel" (func $chic_rt__async_token_cancel (type 1)))
  (import "chic_rt" "borrow_shared" (func $chic_rt__borrow_shared (type 8)))
  (import "chic_rt" "borrow_unique" (func $chic_rt__borrow_unique (type 8)))
  (import "chic_rt" "borrow_release" (func $chic_rt__borrow_release (type 0)))
  (import "chic_rt" "drop_resource" (func $chic_rt__drop_resource (type 0)))
  (import "chic_rt" "drop_missing" (func $chic_rt__drop_missing (type 0)))
  (import "chic_rt" "trace_enter" (func $chic_rt__trace_enter (type 9)))
  (import "chic_rt" "trace_exit" (func $chic_rt__trace_exit (type 10)))
  (import "chic_rt" "trace_flush" (func $chic_rt__trace_flush (type 5)))
  (import "chic_rt" "string_clone" (func $chic_rt__string_clone (type 2)))
  (import "chic_rt" "string_clone_slice" (func $chic_rt__string_clone_slice (type 3)))
  (import "chic_rt" "string_drop" (func $chic_rt__string_drop (type 0)))
  (import "chic_rt" "vec_clone" (func $chic_rt__vec_clone (type 2)))
  (import "chic_rt" "vec_into_array" (func $chic_rt__vec_into_array (type 2)))
  (import "chic_rt" "vec_copy_to_array" (func $chic_rt__vec_copy_to_array (type 2)))
  (import "chic_rt" "vec_drop" (func $chic_rt__vec_drop (type 0)))
  (import "chic_rt" "array_into_vec" (func $chic_rt__array_into_vec (type 2)))
  (import "chic_rt" "array_copy_to_vec" (func $chic_rt__array_copy_to_vec (type 2)))
  (import "chic_rt" "rc_clone" (func $chic_rt__rc_clone (type 2)))
  (import "chic_rt" "rc_drop" (func $chic_rt__rc_drop (type 0)))
  (import "chic_rt" "arc_clone" (func $chic_rt__arc_clone (type 2)))
  (import "chic_rt" "arc_drop" (func $chic_rt__arc_drop (type 0)))
  (import "chic_rt" "string_append_slice" (func $chic_rt__string_append_slice (type 11)))
  (import "chic_rt" "string_append_bool" (func $chic_rt__string_append_bool (type 12)))
  (import "chic_rt" "string_append_char" (func $chic_rt__string_append_char (type 12)))
  (import "chic_rt" "string_append_signed" (func $chic_rt__string_append_signed (type 13)))
  (import "chic_rt" "string_append_unsigned" (func $chic_rt__string_append_unsigned (type 13)))
  (import "chic_rt" "string_append_f32" (func $chic_rt__string_append_f32 (type 14)))
  (import "chic_rt" "string_append_f64" (func $chic_rt__string_append_f64 (type 15)))
  (import "chic_rt" "string_as_slice" (func $chic_rt__string_as_slice (type 16)))
  (import "chic_rt" "span_copy_to" (func $chic_rt__span_copy_to (type 17)))
  (import "chic_rt" "mmio_read" (func $chic_rt__mmio_read (type 18)))
  (import "chic_rt" "mmio_write" (func $chic_rt__mmio_write (type 19)))
  (import "chic_rt" "decimal_add_out" (func $chic_rt__decimal_add_out (type 20)))
  (import "chic_rt" "decimal_add_simd_out" (func $chic_rt__decimal_add_simd_out (type 20)))
  (import "chic_rt" "decimal_sub_out" (func $chic_rt__decimal_sub_out (type 20)))
  (import "chic_rt" "decimal_sub_simd_out" (func $chic_rt__decimal_sub_simd_out (type 20)))
  (import "chic_rt" "decimal_mul_out" (func $chic_rt__decimal_mul_out (type 20)))
  (import "chic_rt" "decimal_mul_simd_out" (func $chic_rt__decimal_mul_simd_out (type 20)))
  (import "chic_rt" "decimal_div_out" (func $chic_rt__decimal_div_out (type 20)))
  (import "chic_rt" "decimal_div_simd_out" (func $chic_rt__decimal_div_simd_out (type 20)))
  (import "chic_rt" "decimal_rem_out" (func $chic_rt__decimal_rem_out (type 20)))
  (import "chic_rt" "decimal_rem_simd_out" (func $chic_rt__decimal_rem_simd_out (type 20)))
  (import "chic_rt" "decimal_fma_out" (func $chic_rt__decimal_fma_out (type 21)))
  (import "chic_rt" "decimal_fma_simd_out" (func $chic_rt__decimal_fma_simd_out (type 21)))
  (import "chic_rt" "decimal_sum_out" (func $chic_rt__decimal_sum_out (type 20)))
  (import "chic_rt" "decimal_sum_simd_out" (func $chic_rt__decimal_sum_simd_out (type 20)))
  (import "chic_rt" "decimal_dot_out" (func $chic_rt__decimal_dot_out (type 21)))
  (import "chic_rt" "decimal_dot_simd_out" (func $chic_rt__decimal_dot_simd_out (type 21)))
  (import "chic_rt" "decimal_matmul" (func $chic_rt__decimal_matmul (type 17)))
  (import "chic_rt" "decimal_matmul_simd" (func $chic_rt__decimal_matmul_simd (type 17)))
  (import "chic_rt" "closure_env_alloc" (func $chic_rt__closure_env_alloc (type 2)))
  (import "chic_rt" "closure_env_clone" (func $chic_rt__closure_env_clone (type 3)))
  (import "chic_rt" "closure_env_free" (func $chic_rt__closure_env_free (type 22)))
  (table (;0;) 105 funcref)
  (memory (;0;) 2)
  (global (;0;) (mut i32) i32.const 4096)
  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_register_future (type 0) (local i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_spawn (type 0) (local i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_block_on (type 0) (local i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_scope (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_spawn_local (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_await (type 2) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 8
    global.set 0
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 7
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 2
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_yield (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_cancel (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_task_result (type 3) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 9
    global.set 0
    i32.const 0
    local.set 8
    i32.const 0
    local.set 4
    (block
      (loop
        local.get 4
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 8
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 3
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_token_state (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_token_new (type 4) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 0
    return
  )

  (func $Std__Async__RuntimeIntrinsics__chic_rt_async_token_cancel (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__Task__SpawnLocal (type 0) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 1
          i32.const 1
          local.set 2
          br 1
        )
        local.get 2
        i32.const 1
        i32.eq
        (if
          local.get 1
          call $chic_rt__async_spawn_local
          drop
          i32.const 2
          local.set 2
          br 1
        )
        local.get 2
        i32.const 2
        i32.eq
        (if
          i32.const 3
          local.set 2
          br 1
        )
        local.get 2
        i32.const 3
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__Task__Scope (type 0) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 1
          i32.const 1
          local.set 2
          br 1
        )
        local.get 2
        i32.const 1
        i32.eq
        (if
          local.get 1
          call $chic_rt__async_scope
          drop
          i32.const 2
          local.set 2
          br 1
        )
        local.get 2
        i32.const 2
        i32.eq
        (if
          i32.const 3
          local.set 2
          br 1
        )
        local.get 2
        i32.const 3
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__Task__Scope_1 (type 5) (local i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 9
    global.set 0
    i32.const 0
    local.set 8
    i32.const 0
    local.set 4
    (block
      (loop
        local.get 4
        i32.const 0
        i32.eq
        (if
          local.get 0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 3
          i32.const 1
          local.set 4
          br 1
        )
        local.get 4
        i32.const 1
        i32.eq
        (if
          local.get 3
          call $chic_rt__async_scope
          drop
          i32.const 2
          local.set 4
          br 1
        )
        local.get 4
        i32.const 2
        i32.eq
        (if
          i32.const 3
          local.set 4
          br 1
        )
        local.get 4
        i32.const 3
        i32.eq
        (if
          local.get 0
          i32.const 40
          i32.add
          i32.load offset=0
          local.set 2
          global.get 0
          local.get 8
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 2
    return
  )

  (func $Std__Async__RuntimeExports__TaskHeader (type 1) (local i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 8
    global.set 0
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          local.get 0
          local.set 2
          local.get 2
          local.set 1
          global.get 0
          local.get 7
          i32.add
          global.set 0
          br 2
        )
        local.get 3
        i32.const 1
        i32.eq
        (if
          global.get 0
          local.get 7
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeExports__TaskBoolResult (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          i32.const 40
          i32.add
          i32.load offset=0
          local.set 1
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__RuntimeExports__TaskIntResult (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          i32.const 40
          i32.add
          i32.load offset=0
          local.set 1
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__CancellationTokenSource__Create (type 4) (local i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    i32.const 8
    i32.sub
    local.tee 8
    global.set 0
    local.get 8
    local.set 0
    local.get 8
    i32.const 4
    i32.add
    local.set 1
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          call $chic_rt__async_token_new
          local.set 2
          i32.const 1
          local.set 3
          br 1
        )
        local.get 3
        i32.const 1
        i32.eq
        (if
          local.get 1
          local.get 2
          i32.store offset=0
          i32.const 2
          local.set 3
          br 1
        )
        local.get 3
        i32.const 2
        i32.eq
        (if
          local.get 0
          local.get 1
          i32.load offset=0
          i32.store offset=0
          global.get 0
          local.get 7
          i32.add
          i32.const 8
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 0
    return
  )

  (func $Std__Async__CancellationTokenSource__Token (type 1) (local i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    i32.const 8
    i32.sub
    local.tee 8
    global.set 0
    local.get 8
    local.set 1
    local.get 8
    i32.const 4
    i32.add
    local.set 2
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          local.get 2
          local.get 0
          i32.load offset=0
          i32.store offset=0
          i32.const 1
          local.set 3
          br 1
        )
        local.get 3
        i32.const 1
        i32.eq
        (if
          local.get 1
          local.get 2
          i32.load offset=0
          i32.store offset=0
          global.get 0
          local.get 7
          i32.add
          i32.const 8
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__CancellationTokenSource__Cancel (type 0) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          i32.load offset=0
          i32.const 0
          i32.eq
          local.set 1
          local.get 1
          local.set 3
          local.get 3
          i32.const 1
          i32.eq
          (if
            i32.const 1
            local.set 2
            br 1
          )
          i32.const 2
          local.set 2
          br 1
        )
        local.get 2
        i32.const 1
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        local.get 2
        i32.const 2
        i32.eq
        (if
          local.get 0
          i32.load offset=0
          call $chic_rt__async_token_cancel
          drop
          i32.const 3
          local.set 2
          br 1
        )
        local.get 2
        i32.const 3
        i32.eq
        (if
          i32.const 4
          local.set 2
          br 1
        )
        local.get 2
        i32.const 4
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__CancellationTokenSource__StubMarker (type 1) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          i32.const 1234
          local.set 1
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__CancellationTokenSource__get_IsCanceled (type 1) (local i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 11
    global.set 0
    i32.const 0
    local.set 10
    i32.const 0
    local.set 6
    (block
      (loop
        local.get 6
        i32.const 0
        i32.eq
        (if
          local.get 0
          i32.load offset=0
          i32.const 0
          i32.eq
          local.set 2
          local.get 2
          local.set 7
          local.get 7
          i32.const 1
          i32.eq
          (if
            i32.const 1
            local.set 6
            br 1
          )
          i32.const 2
          local.set 6
          br 1
        )
        local.get 6
        i32.const 1
        i32.eq
        (if
          i32.const 0
          local.set 1
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        local.get 6
        i32.const 2
        i32.eq
        (if
          local.get 0
          i32.load offset=0
          call $Std__Async__RuntimeIntrinsics__chic_rt_async_token_state
          local.set 4
          i32.const 3
          local.set 6
          br 1
        )
        local.get 6
        i32.const 3
        i32.eq
        (if
          local.get 4
          local.set 3
          local.get 3
          i32.const 0
          i32.ne
          local.set 5
          local.get 5
          local.set 1
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        local.get 6
        i32.const 4
        i32.eq
        (if
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__CancellationToken__IsCancellationRequested (type 1) (local i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 11
    global.set 0
    i32.const 0
    local.set 10
    i32.const 0
    local.set 6
    (block
      (loop
        local.get 6
        i32.const 0
        i32.eq
        (if
          local.get 0
          i32.load offset=0
          i32.const 0
          i32.eq
          local.set 2
          local.get 2
          local.set 7
          local.get 7
          i32.const 1
          i32.eq
          (if
            i32.const 1
            local.set 6
            br 1
          )
          i32.const 2
          local.set 6
          br 1
        )
        local.get 6
        i32.const 1
        i32.eq
        (if
          i32.const 0
          local.set 1
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        local.get 6
        i32.const 2
        i32.eq
        (if
          local.get 0
          i32.load offset=0
          call $Std__Async__RuntimeIntrinsics__chic_rt_async_token_state
          local.set 4
          i32.const 3
          local.set 6
          br 1
        )
        local.get 6
        i32.const 3
        i32.eq
        (if
          local.get 4
          local.set 3
          local.get 3
          i32.const 0
          i32.ne
          local.set 5
          local.get 5
          local.set 1
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        local.get 6
        i32.const 4
        i32.eq
        (if
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__Runtime__Spawn (type 0) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 1
          i32.const 1
          local.set 2
          br 1
        )
        local.get 2
        i32.const 1
        i32.eq
        (if
          local.get 1
          call $chic_rt__async_spawn
          drop
          i32.const 2
          local.set 2
          br 1
        )
        local.get 2
        i32.const 2
        i32.eq
        (if
          i32.const 3
          local.set 2
          br 1
        )
        local.get 2
        i32.const 3
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__Runtime__BlockOn (type 0) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 7
    global.set 0
    i32.const 0
    local.set 6
    i32.const 0
    local.set 2
    (block
      (loop
        local.get 2
        i32.const 0
        i32.eq
        (if
          local.get 0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 1
          i32.const 1
          local.set 2
          br 1
        )
        local.get 2
        i32.const 1
        i32.eq
        (if
          local.get 1
          call $chic_rt__async_scope
          drop
          i32.const 2
          local.set 2
          br 1
        )
        local.get 2
        i32.const 2
        i32.eq
        (if
          i32.const 3
          local.set 2
          br 1
        )
        local.get 2
        i32.const 3
        i32.eq
        (if
          global.get 0
          local.get 6
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $Std__Async__Runtime__Cancel (type 1) (local i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 11
    global.set 0
    i32.const 0
    local.set 10
    i32.const 0
    local.set 6
    (block
      (loop
        local.get 6
        i32.const 0
        i32.eq
        (if
          local.get 0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 3
          i32.const 1
          local.set 6
          br 1
        )
        local.get 6
        i32.const 1
        i32.eq
        (if
          local.get 3
          call $Std__Async__RuntimeIntrinsics__chic_rt_async_cancel
          local.set 4
          i32.const 2
          local.set 6
          br 1
        )
        local.get 6
        i32.const 2
        i32.eq
        (if
          local.get 4
          local.set 2
          local.get 2
          i32.const 1
          i32.eq
          local.set 5
          local.get 5
          local.set 1
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        local.get 6
        i32.const 3
        i32.eq
        (if
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $Std__Async__Runtime__CancelInt (type 1) (local i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 11
    global.set 0
    i32.const 0
    local.set 10
    i32.const 0
    local.set 6
    (block
      (loop
        local.get 6
        i32.const 0
        i32.eq
        (if
          i32.const 1
          i32.const 2
          i32.or
          local.set 3
          local.get 3
          i32.const 4
          i32.or
          local.set 4
          local.get 4
          local.set 2
          local.get 0
          i32.const 12
          i32.add
          local.get 2
          i32.store offset=0
          local.get 0
          i32.const 16
          i32.add
          local.get 2
          i32.store offset=0
          local.get 0
          i32.const 32
          i32.add
          local.get 2
          i32.store offset=0
          local.get 0
          i32.const 36
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 0
          i32.const 40
          i32.add
          i32.const 0
          i32.store offset=0
          local.get 0
          i32.load offset=0
          call $Std__Async__RuntimeExports__TaskHeader
          local.set 5
          i32.const 1
          local.set 6
          br 1
        )
        local.get 6
        i32.const 1
        i32.eq
        (if
          local.get 5
          call $Std__Async__RuntimeIntrinsics__chic_rt_async_cancel
          i32.const 2
          local.set 6
          br 1
        )
        local.get 6
        i32.const 2
        i32.eq
        (if
          i32.const 3
          local.set 6
          br 1
        )
        local.get 6
        i32.const 3
        i32.eq
        (if
          i32.const 1
          local.set 1
          global.get 0
          local.get 10
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $AsyncCli__CountAsync (type 1) (local i32 i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    i32.const 80
    i32.sub
    local.tee 10
    global.set 0
    local.get 10
    local.set 1
    i32.const 0
    local.set 9
    i32.const 0
    local.set 5
    (block
      (loop
        local.get 5
        i32.const 0
        i32.eq
        (if
          i32.const 0
          local.set 1
          local.get 0
          i32.const 2
          i32.add
          local.set 4
          local.get 4
          local.set 3
          local.get 1
          i32.const 24
          i32.add
          i32.const 3
          i32.store offset=0
          local.get 1
          i32.const 32
          i32.add
          i32.const 3
          i32.store offset=0
          local.get 1
          i32.const 64
          i32.add
          i32.const 3
          i32.store offset=0
          local.get 1
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 1
          i32.const 76
          i32.add
          local.get 3
          i32.store offset=0
          local.get 10
          local.set 1
          local.get 1
          i32.const 8
          i32.add
          i32.const 0
          i32.store offset=0
          local.get 1
          i32.const 24
          i32.add
          local.tee 6
          i32.load offset=0
          local.set 8
          local.get 8
          i32.const 3
          i32.or
          local.set 8
          local.get 6
          local.get 8
          i32.store offset=0
          local.get 1
          i32.const 32
          i32.add
          local.tee 6
          i32.load offset=0
          local.set 8
          local.get 8
          i32.const 3
          i32.or
          local.set 8
          local.get 6
          local.get 8
          i32.store offset=0
          local.get 1
          i32.const 64
          i32.add
          local.tee 6
          i32.load offset=0
          local.set 8
          local.get 8
          i32.const 3
          i32.or
          local.set 8
          local.get 6
          local.get 8
          i32.store offset=0
          local.get 1
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 1
          i32.const 76
          i32.add
          local.get 3
          i32.store offset=0
          global.get 0
          local.get 9
          i32.add
          i32.const 80
          i32.add
          global.set 0
          local.get 1
          return
        )
        unreachable
      )
    )
    local.get 1
    return
  )

  (func $AsyncCli__AsyncWorkflow (type 4) (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    i32.const 88
    i32.sub
    local.tee 15
    global.set 0
    local.get 15
    local.set 0
    local.get 15
    i32.const 80
    i32.add
    local.set 4
    local.get 15
    i32.const 84
    i32.add
    local.set 7
    i32.const 0
    local.set 14
    i32.const 0
    local.set 10
    (block
      (loop
        local.get 10
        i32.const 0
        i32.eq
        (if
          i32.const 0
          local.set 0
          i32.const 1
          call $AsyncCli__CountAsync
          local.set 3
          i32.const 1
          local.set 10
          br 1
        )
        local.get 10
        i32.const 1
        i32.eq
        (if
          local.get 3
          local.set 11
          local.get 1
          local.get 11
          call $chic_rt__await
          local.set 13
          local.get 13
          i32.const 1
          i32.eq
          (if
            local.get 11
            i32.const 76
            i32.add
            local.get 4
            i32.const 4
            call $chic_rt__async_task_result
            i32.const 2
            local.set 10
            br 1
          )
          i32.const 3
          local.set 10
          br 1
        )
        local.get 10
        i32.const 2
        i32.eq
        (if
          local.get 4
          i32.load offset=0
          local.set 2
          local.get 2
          call $AsyncCli__CountAsync
          local.set 6
          i32.const 4
          local.set 10
          br 1
        )
        local.get 10
        i32.const 3
        i32.eq
        (if
          local.get 15
          local.set 0
          local.get 0
          i32.const 8
          i32.add
          i32.const 8
          i32.store offset=0
          local.get 0
          i32.const 24
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 32
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 64
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 0
          i32.const 76
          i32.add
          local.get 8
          i32.store offset=0
          global.get 0
          local.get 14
          i32.add
          i32.const 88
          i32.add
          global.set 0
          local.get 0
          return
        )
        local.get 10
        i32.const 4
        i32.eq
        (if
          local.get 6
          local.set 11
          local.get 1
          local.get 11
          call $chic_rt__await
          local.set 13
          local.get 13
          i32.const 1
          i32.eq
          (if
            local.get 11
            i32.const 76
            i32.add
            local.get 7
            i32.const 4
            call $chic_rt__async_task_result
            i32.const 5
            local.set 10
            br 1
          )
          i32.const 6
          local.set 10
          br 1
        )
        local.get 10
        i32.const 5
        i32.eq
        (if
          local.get 7
          i32.load offset=0
          local.set 5
          local.get 5
          i32.const 5
          i32.eq
          local.set 9
          local.get 9
          local.set 8
          local.get 15
          local.set 0
          local.get 0
          i32.const 8
          i32.add
          i32.const 8
          i32.store offset=0
          local.get 0
          i32.const 24
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 32
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 64
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 0
          i32.const 76
          i32.add
          local.get 8
          i32.store offset=0
          global.get 0
          local.get 14
          i32.add
          i32.const 88
          i32.add
          global.set 0
          local.get 0
          return
        )
        local.get 10
        i32.const 6
        i32.eq
        (if
          local.get 15
          local.set 0
          local.get 0
          i32.const 8
          i32.add
          i32.const 8
          i32.store offset=0
          local.get 0
          i32.const 24
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 32
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 64
          i32.add
          local.tee 11
          i32.load offset=0
          local.set 13
          local.get 13
          i32.const 3
          i32.or
          local.set 13
          local.get 11
          local.get 13
          i32.store offset=0
          local.get 0
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 0
          i32.const 76
          i32.add
          local.get 8
          i32.store offset=0
          global.get 0
          local.get 14
          i32.add
          i32.const 88
          i32.add
          global.set 0
          local.get 0
          return
        )
        unreachable
      )
    )
    local.get 0
    return
  )

  (func $AsyncCli__AsyncFailure (type 4) (local i32 i32 i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    i32.const 80
    i32.sub
    local.tee 8
    global.set 0
    local.get 8
    local.set 0
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          i32.const 0
          local.set 0
          i32.const 0
          local.set 2
          local.get 0
          i32.const 24
          i32.add
          i32.const 3
          i32.store offset=0
          local.get 0
          i32.const 32
          i32.add
          i32.const 3
          i32.store offset=0
          local.get 0
          i32.const 64
          i32.add
          i32.const 3
          i32.store offset=0
          local.get 0
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 0
          i32.const 73
          i32.add
          local.get 2
          i32.store offset=0
          local.get 8
          local.set 0
          local.get 0
          i32.const 8
          i32.add
          i32.const 16
          i32.store offset=0
          local.get 0
          i32.const 24
          i32.add
          local.tee 4
          i32.load offset=0
          local.set 6
          local.get 6
          i32.const 3
          i32.or
          local.set 6
          local.get 4
          local.get 6
          i32.store offset=0
          local.get 0
          i32.const 32
          i32.add
          local.tee 4
          i32.load offset=0
          local.set 6
          local.get 6
          i32.const 3
          i32.or
          local.set 6
          local.get 4
          local.get 6
          i32.store offset=0
          local.get 0
          i32.const 64
          i32.add
          local.tee 4
          i32.load offset=0
          local.set 6
          local.get 6
          i32.const 3
          i32.or
          local.set 6
          local.get 4
          local.get 6
          i32.store offset=0
          local.get 0
          i32.const 72
          i32.add
          i32.const 1
          i32.store offset=0
          local.get 0
          i32.const 76
          i32.add
          local.get 2
          i32.store offset=0
          global.get 0
          local.get 7
          i32.add
          i32.const 80
          i32.add
          global.set 0
          local.get 0
          return
        )
        unreachable
      )
    )
    local.get 0
    return
  )

  (func $AsyncCli__CountAsync__poll (type 2) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 8
    global.set 0
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          i32.const 1
          local.set 2
          global.get 0
          local.get 7
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 2
    return
  )

  (func $AsyncCli__CountAsync__drop (type 0) (local i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $AsyncCli__AsyncWorkflow__poll (type 2) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 8
    global.set 0
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          i32.const 1
          local.set 2
          global.get 0
          local.get 7
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 2
    return
  )

  (func $AsyncCli__AsyncWorkflow__drop (type 0) (local i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (func $AsyncCli__AsyncFailure__poll (type 2) (local i32 i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 8
    global.set 0
    i32.const 0
    local.set 7
    i32.const 0
    local.set 3
    (block
      (loop
        local.get 3
        i32.const 0
        i32.eq
        (if
          i32.const 1
          local.set 2
          global.get 0
          local.get 7
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    local.get 2
    return
  )

  (func $AsyncCli__AsyncFailure__drop (type 0) (local i32 i32 i64 i32 i32 i32)
    global.get 0
    local.tee 6
    global.set 0
    i32.const 0
    local.set 5
    i32.const 0
    local.set 1
    (block
      (loop
        local.get 1
        i32.const 0
        i32.eq
        (if
          global.get 0
          local.get 5
          i32.add
          global.set 0
          br 2
        )
        unreachable
      )
    )
    return
  )

  (elem (table 0) (i32.const 0) func $chic_rt__object_new func $chic_rt__panic func $chic_rt__abort func $chic_rt__throw func $chic_rt__await func $chic_rt__yield func $chic_rt__async_cancel func $chic_rt__async_spawn func $chic_rt__async_spawn_local func $chic_rt__async_scope func $chic_rt__async_task_header func $chic_rt__async_task_result func $chic_rt__async_token_state func $chic_rt__async_token_new func $chic_rt__async_token_cancel func $chic_rt__borrow_shared func $chic_rt__borrow_unique func $chic_rt__borrow_release func $chic_rt__drop_resource func $chic_rt__drop_missing func $chic_rt__trace_enter func $chic_rt__trace_exit func $chic_rt__trace_flush func $chic_rt__string_clone func $chic_rt__string_clone_slice func $chic_rt__string_drop func $chic_rt__vec_clone func $chic_rt__vec_into_array func $chic_rt__vec_copy_to_array func $chic_rt__vec_drop func $chic_rt__array_into_vec func $chic_rt__array_copy_to_vec func $chic_rt__rc_clone func $chic_rt__rc_drop func $chic_rt__arc_clone func $chic_rt__arc_drop func $chic_rt__string_append_slice func $chic_rt__string_append_bool func $chic_rt__string_append_char func $chic_rt__string_append_signed func $chic_rt__string_append_unsigned func $chic_rt__string_append_f32 func $chic_rt__string_append_f64 func $chic_rt__string_as_slice func $chic_rt__span_copy_to func $chic_rt__mmio_read func $chic_rt__mmio_write func $chic_rt__decimal_add_out func $chic_rt__decimal_add_simd_out func $chic_rt__decimal_sub_out func $chic_rt__decimal_sub_simd_out func $chic_rt__decimal_mul_out func $chic_rt__decimal_mul_simd_out func $chic_rt__decimal_div_out func $chic_rt__decimal_div_simd_out func $chic_rt__decimal_rem_out func $chic_rt__decimal_rem_simd_out func $chic_rt__decimal_fma_out func $chic_rt__decimal_fma_simd_out func $chic_rt__decimal_sum_out func $chic_rt__decimal_sum_simd_out func $chic_rt__decimal_dot_out func $chic_rt__decimal_dot_simd_out func $chic_rt__decimal_matmul func $chic_rt__decimal_matmul_simd func $chic_rt__closure_env_alloc func $chic_rt__closure_env_clone func $chic_rt__closure_env_free func $Std__Async__RuntimeIntrinsics__chic_rt_async_register_future func $Std__Async__RuntimeIntrinsics__chic_rt_async_spawn func $Std__Async__RuntimeIntrinsics__chic_rt_async_block_on func $Std__Async__RuntimeIntrinsics__chic_rt_async_scope func $Std__Async__RuntimeIntrinsics__chic_rt_async_spawn_local func $Std__Async__RuntimeIntrinsics__chic_rt_await func $Std__Async__RuntimeIntrinsics__chic_rt_yield func $Std__Async__RuntimeIntrinsics__chic_rt_async_cancel func $Std__Async__RuntimeIntrinsics__chic_rt_async_task_result func $Std__Async__RuntimeIntrinsics__chic_rt_async_token_state func $Std__Async__RuntimeIntrinsics__chic_rt_async_token_new func $Std__Async__RuntimeIntrinsics__chic_rt_async_token_cancel func $Std__Async__Task__SpawnLocal func $Std__Async__Task__Scope func $Std__Async__Task__Scope_1 func $Std__Async__RuntimeExports__TaskHeader func $Std__Async__RuntimeExports__TaskBoolResult func $Std__Async__RuntimeExports__TaskIntResult func $Std__Async__CancellationTokenSource__Create func $Std__Async__CancellationTokenSource__Token func $Std__Async__CancellationTokenSource__Cancel func $Std__Async__CancellationTokenSource__StubMarker func $Std__Async__CancellationTokenSource__get_IsCanceled func $Std__Async__CancellationToken__IsCancellationRequested func $Std__Async__Runtime__Spawn func $Std__Async__Runtime__BlockOn func $Std__Async__Runtime__Cancel func $Std__Async__Runtime__CancelInt func $AsyncCli__CountAsync func $AsyncCli__AsyncWorkflow func $AsyncCli__AsyncFailure func $AsyncCli__CountAsync__poll func $AsyncCli__CountAsync__drop func $AsyncCli__AsyncWorkflow__poll func $AsyncCli__AsyncWorkflow__drop func $AsyncCli__AsyncFailure__poll func $AsyncCli__AsyncFailure__drop)
  (export "chic_rt_async_task_bool_result" (func $Std__Async__RuntimeExports__TaskBoolResult))
  (export "chic_rt_async_task_header" (func $Std__Async__RuntimeExports__TaskHeader))
  (export "chic_rt_async_task_int_result" (func $Std__Async__RuntimeExports__TaskIntResult))
  (export "test::AsyncCli::AsyncFailure" (func $AsyncCli__AsyncFailure))
  (export "test::AsyncCli::AsyncWorkflow" (func $AsyncCli__AsyncWorkflow))
)
