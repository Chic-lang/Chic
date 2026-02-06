mod args;
mod dispatch;
mod runtime;

use std::fmt::Write;

use crate::abi::CAbiReturn;
use crate::codegen::llvm::signatures::{
    LlvmFunctionSignature, canonical_function_name, resolve_function_name,
};
use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::{BlockId, CallDispatch, FnTy, Operand, Place, Ty};

use super::super::builder::FunctionEmitter;
use super::super::values::ValueRef;
use args::{render_args_for_c_abi_params, render_args_for_signature};

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
        unwind: Option<BlockId>,
        dispatch: Option<&CallDispatch>,
    ) -> Result<(), Error> {
        if let Some(dispatch) = dispatch {
            match dispatch {
                CallDispatch::Trait(trait_dispatch) => {
                    return self.emit_trait_object_call(
                        args,
                        destination,
                        target,
                        unwind,
                        trait_dispatch,
                    );
                }
                CallDispatch::Virtual(virtual_dispatch) => {
                    return self.emit_virtual_call(
                        func,
                        args,
                        destination,
                        target,
                        unwind,
                        virtual_dispatch,
                    );
                }
            }
        }
        if let Some(fn_ty) = self.call_operand_fn_ty(func) {
            self.emit_indirect_call(func, args, destination, target, unwind, fn_ty)?;
            return Ok(());
        }

        if self
            .function
            .name
            .contains("Std::Numeric::NumericBitOperations::RotateLeft")
        {
            if matches!(func, Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty())
                && args.len() == 1
            {
                let (llvm_ty, bit_width) = if self.function.name.contains("64") {
                    ("i64", 64u32)
                } else if self.function.name.contains("32") {
                    ("i32", 32u32)
                } else if self.function.name.contains("16") {
                    ("i16", 16u32)
                } else {
                    ("i8", 8u32)
                };
                let normalize_int = |emitter: &mut FunctionEmitter<'a>,
                                     value: ValueRef|
                 -> Result<ValueRef, Error> {
                    if value.ty() == llvm_ty {
                        return Ok(value);
                    }
                    if let Some(width) = value
                        .ty()
                        .strip_prefix('i')
                        .and_then(|bits| bits.parse::<u32>().ok())
                    {
                        if width < bit_width {
                            let tmp = emitter.new_temp();
                            writeln!(
                                &mut emitter.builder,
                                "  {tmp} = zext {value_ty} {} to {llvm_ty}",
                                value.repr(),
                                value_ty = value.ty()
                            )
                            .ok();
                            return Ok(ValueRef::new(tmp, llvm_ty));
                        } else if width > bit_width {
                            let tmp = emitter.new_temp();
                            writeln!(
                                &mut emitter.builder,
                                "  {tmp} = trunc {value_ty} {} to {llvm_ty}",
                                value.repr(),
                                value_ty = value.ty()
                            )
                            .ok();
                            return Ok(ValueRef::new(tmp, llvm_ty));
                        }
                    }
                    Ok(ValueRef::new(value.repr().to_string(), llvm_ty))
                };
                let bits_raw = self.emit_operand(func, None)?;
                let bits = normalize_int(self, bits_raw)?;
                let rhs_raw = self.emit_operand(&args[0], None)?;
                let shift_rhs = normalize_int(self, rhs_raw)?;
                let shift_lhs = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {shift_lhs} = sub {llvm_ty} {bit_width}, {}",
                    shift_rhs.repr()
                )
                .ok();
                let left = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {left} = shl {llvm_ty} {}, {}",
                    bits.repr(),
                    shift_lhs
                )
                .ok();
                let right = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {right} = lshr {llvm_ty} {}, {}",
                    bits.repr(),
                    shift_rhs.repr()
                )
                .ok();
                let rotated = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {rotated} = or {llvm_ty} {left}, {right}"
                )
                .ok();
                if let Some(dest) = destination {
                    if let Some(entry) = self.local_tys.get_mut(dest.local.0) {
                        *entry = Some(llvm_ty.to_string());
                    }
                    let value = ValueRef::new(rotated.clone(), llvm_ty);
                    self.store_place(dest, &value)?;
                }
                let dest_label = self.block_label(target)?;
                self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
                return Ok(());
            }
        }

        let mut repr = match self.call_operand_repr(func) {
            Ok(repr) => repr,
            Err(err) => {
                if std::env::var("CHIC_DEBUG_CALLS").is_ok() {
                    eprintln!(
                        "[chic-debug] call operand error in {}: func={func:?} args={args:?}",
                        self.function.name
                    );
                }
                return Err(err);
            }
        };
        if let Some(resolved) = self.resolve_constructor_init_name(&repr, args.len()) {
            repr = resolved;
        }
        if std::env::var_os("CHIC_DEBUG_ASYNC_READY").is_some() {
            let canonical = canonical_function_name(&repr);
            if !self.signatures.contains_key(&canonical) {
                let mut related: Vec<_> = self
                    .signatures
                    .keys()
                    .filter(|k| k.contains("Async::Runtime"))
                    .cloned()
                    .collect();
                related.sort();
                eprintln!(
                    "[chic-debug] missing signature for {repr} (canonical {canonical}); async-runtime keys: {:?}",
                    related
                );
            }
        }
        if self.try_emit_object_new_call(&repr, args, destination, target)? {
            return Ok(());
        }
        if self.emit_decimal_runtime_by_repr(&repr, args, destination, target)? {
            return Ok(());
        }
        if self.try_emit_intrinsic_call(&repr, args, destination, target)? {
            return Ok(());
        }
        if self.try_emit_startup_runtime_call(&repr, args, destination, target)? {
            return Ok(());
        }
        if let Some(signature) = self.try_emit_native_runtime_helper(&repr)? {
            let canonical = canonical_function_name(&repr);
            self.emit_direct_call(&canonical, &signature, args, destination, target, unwind)?;
            return Ok(());
        }
        self.note_async_runtime_intrinsic(&repr);
        let callee = match resolve_function_name(self.signatures, &repr) {
            Some(name) => name,
            None => {
                if let Some(dest) = destination {
                    if let Some(dest_ty) = self.place_type(dest)? {
                        let literal = if dest_ty.starts_with('{') || dest_ty.starts_with('[') {
                            "zeroinitializer".to_string()
                        } else if dest_ty == "ptr" {
                            "null".to_string()
                        } else {
                            "0".to_string()
                        };
                        let value = ValueRef::new_literal(literal, &dest_ty);
                        self.store_place(dest, &value)?;
                    }
                }
                let dest_label = self.block_label(target)?;
                self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
                return Ok(());
            }
        };
        let signature = self
            .signatures
            .get(&callee)
            .ok_or_else(|| Error::Codegen(format!("missing signature for `{callee}`")))?;

        self.emit_direct_call(&callee, signature, args, destination, target, unwind)
    }

    fn emit_branch_to_labels_or_unwind(
        &mut self,
        dest_label: &str,
        unwind: Option<BlockId>,
    ) -> Result<(), Error> {
        if let Some(unwind_target) = unwind {
            let unwind_label = self.block_label(unwind_target)?;
            self.externals.insert("chic_rt_has_pending_exception");
            let pending_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {pending_tmp} = call i32 @chic_rt_has_pending_exception()"
            )
            .ok();
            let cond_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cond_tmp} = icmp ne i32 {pending_tmp}, 0"
            )
            .ok();
            writeln!(
                &mut self.builder,
                "  br i1 {cond_tmp}, label %{unwind_label}, label %{dest_label}"
            )
            .ok();
            return Ok(());
        }
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    fn resolve_constructor_init_name(&self, repr: &str, args_len: usize) -> Option<String> {
        let canonical = canonical_function_name(repr);
        if let Some(owner) = canonical.strip_suffix("::init#super") {
            let owner_key = self.type_layouts.resolve_type_key(owner).unwrap_or(owner);
            let class = self.type_layouts.class_layout_info(owner_key)?;
            let base = class.bases.first()?;
            let base_key = base.replace('.', "::");
            let canonical_base = self
                .type_layouts
                .resolve_type_key(base_key.as_str())
                .unwrap_or(base_key.as_str());
            let mut prefixes = vec![format!("{canonical_base}::init#")];
            if canonical_base != base_key {
                prefixes.push(format!("{base_key}::init#"));
            }
            return self.match_constructor_by_arity(&prefixes, args_len);
        }
        if let Some(owner) = canonical.strip_suffix("::init#self") {
            let owner_key = self
                .type_layouts
                .resolve_type_key(owner)
                .unwrap_or(owner)
                .to_string();
            let prefix = format!("{owner_key}::init#");
            return self.match_constructor_by_arity(&[prefix], args_len);
        }
        None
    }

    fn match_constructor_by_arity(&self, prefixes: &[String], args_len: usize) -> Option<String> {
        let mut candidates: Vec<&String> = self
            .signatures
            .keys()
            .filter(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        candidates.sort();
        if let Some(name) = candidates.iter().find(|name| {
            self.signatures
                .get(name.as_str())
                .map(|sig| sig.params.len() == args_len)
                .unwrap_or(false)
        }) {
            return Some((*name).clone());
        }
        candidates.first().cloned().cloned()
    }

    fn emit_direct_call(
        &mut self,
        callee: &str,
        signature: &LlvmFunctionSignature,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
        unwind: Option<BlockId>,
    ) -> Result<(), Error> {
        let dest_label = self.block_label(target)?;
        self.ensure_std_runtime_intrinsic_owner(callee, signature.symbol.as_str())?;

        if let Some(sym) = match signature.symbol.as_str() {
            "chic_rt_decimal_sum" => Some("chic_rt_decimal_sum"),
            "chic_rt_decimal_dot" => Some("chic_rt_decimal_dot"),
            "chic_rt_decimal_matmul" => Some("chic_rt_decimal_matmul"),
            "chic_rt_decimal_clone" => Some("chic_rt_decimal_clone"),
            "chic_rt_region_alloc" => Some("chic_rt_region_alloc"),
            "chic_rt_region_alloc_zeroed" => Some("chic_rt_region_alloc_zeroed"),
            "chic_rt_region_enter" => Some("chic_rt_region_enter"),
            "chic_rt_region_exit" => Some("chic_rt_region_exit"),
            "chic_rt_region_telemetry" => Some("chic_rt_region_telemetry"),
            "chic_rt_region_reset_stats" => Some("chic_rt_region_reset_stats"),
            "chic_rt_alloc" => Some("chic_rt_alloc"),
            "chic_rt_alloc_zeroed" => Some("chic_rt_alloc_zeroed"),
            "chic_rt_realloc" => Some("chic_rt_realloc"),
            "chic_rt_free" => Some("chic_rt_free"),
            "chic_rt_alloc_stats" => Some("chic_rt_alloc_stats"),
            "chic_rt_reset_alloc_stats" => Some("chic_rt_reset_alloc_stats"),
            "chic_rt_allocator_install" => Some("chic_rt_allocator_install"),
            "chic_rt_allocator_reset" => Some("chic_rt_allocator_reset"),
            "chic_rt_memcpy" => Some("chic_rt_memcpy"),
            "chic_rt_memmove" => Some("chic_rt_memmove"),
            "chic_rt_memset" => Some("chic_rt_memset"),
            "chic_rt_ptr_is_null_mut" => Some("chic_rt_ptr_is_null_mut"),
            "chic_rt_ptr_is_null_const" => Some("chic_rt_ptr_is_null_const"),
            "chic_rt_type_drop_glue" => Some("chic_rt_type_drop_glue"),
            "chic_rt_type_clone_glue" => Some("chic_rt_type_clone_glue"),
            "chic_rt_type_hash_glue" => Some("chic_rt_type_hash_glue"),
            "chic_rt_type_eq_glue" => Some("chic_rt_type_eq_glue"),
            "chic_rt_type_size" => Some("chic_rt_type_size"),
            "chic_rt_type_align" => Some("chic_rt_type_align"),
            "chic_rt_type_metadata" => Some("chic_rt_type_metadata"),
            "chic_rt_type_metadata_register" => Some("chic_rt_type_metadata_register"),
            "chic_rt_type_metadata_clear" => Some("chic_rt_type_metadata_clear"),
            "__drop_noop" => Some("__drop_noop"),
            "chic_rt_drop_noop_ptr" => Some("chic_rt_drop_noop_ptr"),
            "chic_rt_drop_register" => Some("chic_rt_drop_register"),
            "chic_rt_drop_clear" => Some("chic_rt_drop_clear"),
            "chic_rt_drop_resolve" => Some("chic_rt_drop_resolve"),
            "chic_rt_drop_invoke" => Some("chic_rt_drop_invoke"),
            "chic_rt_drop_missing" => Some("chic_rt_drop_missing"),
            "chic_rt_install_drop_table" => Some("chic_rt_install_drop_table"),
            "chic_rt_install_hash_table" => Some("chic_rt_install_hash_table"),
            "chic_rt_install_eq_table" => Some("chic_rt_install_eq_table"),
            "chic_rt_throw" => Some("chic_rt_throw"),
            "chic_rt_closure_env_alloc" => Some("chic_rt_closure_env_alloc"),
            "chic_rt_closure_env_free" => Some("chic_rt_closure_env_free"),
            "chic_rt_closure_env_clone" => Some("chic_rt_closure_env_clone"),
            "chic_rt_clone_invoke" => Some("chic_rt_clone_invoke"),
            "chic_rt_hash_invoke" => Some("chic_rt_hash_invoke"),
            "chic_rt_eq_invoke" => Some("chic_rt_eq_invoke"),
            "chic_rt_abort" => Some("chic_rt_abort"),
            "chic_rt_span_from_raw_mut" => Some("chic_rt_span_from_raw_mut"),
            "chic_rt_span_from_raw_const" => Some("chic_rt_span_from_raw_const"),
            "chic_rt_span_copy_to" => Some("chic_rt_span_copy_to"),
            "chic_rt_span_fill" => Some("chic_rt_span_fill"),
            "chic_rt_span_slice_mut" => Some("chic_rt_span_slice_mut"),
            "chic_rt_span_slice_readonly" => Some("chic_rt_span_slice_readonly"),
            "chic_rt_span_ptr_at_mut" => Some("chic_rt_span_ptr_at_mut"),
            "chic_rt_span_ptr_at_readonly" => Some("chic_rt_span_ptr_at_readonly"),
            "chic_rt_span_to_readonly" => Some("chic_rt_span_to_readonly"),
            "chic_rt_string_as_slice" => Some("chic_rt_string_as_slice"),
            "chic_rt_string_as_chars" => Some("chic_rt_string_as_chars"),
            "chic_rt_str_as_chars" => Some("chic_rt_str_as_chars"),
            "chic_rt_string_new" => Some("chic_rt_string_new"),
            "chic_rt_string_with_capacity" => Some("chic_rt_string_with_capacity"),
            "chic_rt_string_from_slice" => Some("chic_rt_string_from_slice"),
            "chic_rt_string_from_char" => Some("chic_rt_string_from_char"),
            "chic_rt_string_drop" => Some("chic_rt_string_drop"),
            "chic_rt_string_clone" => Some("chic_rt_string_clone"),
            "chic_rt_string_clone_slice" => Some("chic_rt_string_clone_slice"),
            "chic_rt_string_append_slice" => Some("chic_rt_string_append_slice"),
            "chic_rt_string_append_bool" => Some("chic_rt_string_append_bool"),
            "chic_rt_string_append_char" => Some("chic_rt_string_append_char"),
            "chic_rt_string_append_signed" => Some("chic_rt_string_append_signed"),
            "chic_rt_string_append_unsigned" => Some("chic_rt_string_append_unsigned"),
            "chic_rt_string_append_f16" => Some("chic_rt_string_append_f16"),
            "chic_rt_string_append_f32" => Some("chic_rt_string_append_f32"),
            "chic_rt_string_append_f64" => Some("chic_rt_string_append_f64"),
            "chic_rt_string_append_f128" => Some("chic_rt_string_append_f128"),
            "chic_rt_string_reserve" => Some("chic_rt_string_reserve"),
            "chic_rt_string_truncate" => Some("chic_rt_string_truncate"),
            "chic_rt_string_push_slice" => Some("chic_rt_string_push_slice"),
            "chic_rt_string_get_ptr" => Some("chic_rt_string_get_ptr"),
            "chic_rt_string_set_ptr" => Some("chic_rt_string_set_ptr"),
            "chic_rt_string_get_len" => Some("chic_rt_string_get_len"),
            "chic_rt_string_set_len" => Some("chic_rt_string_set_len"),
            "chic_rt_string_get_cap" => Some("chic_rt_string_get_cap"),
            "chic_rt_string_set_cap" => Some("chic_rt_string_set_cap"),
            "chic_rt_string_inline_ptr" => Some("chic_rt_string_inline_ptr"),
            "chic_rt_string_inline_capacity" => Some("chic_rt_string_inline_capacity"),
            "chic_rt_string_error_message" => Some("chic_rt_string_error_message"),
            "chic_rt_string_allocations" => Some("chic_rt_string_allocations"),
            "chic_rt_string_frees" => Some("chic_rt_string_frees"),
            "chic_rt_string_debug_ping" => Some("chic_rt_string_debug_ping"),
            "chic_rt_vec_new" => Some("chic_rt_vec_new"),
            "chic_rt_vec_with_capacity" => Some("chic_rt_vec_with_capacity"),
            "chic_rt_vec_reserve" => Some("chic_rt_vec_reserve"),
            "chic_rt_vec_shrink_to_fit" => Some("chic_rt_vec_shrink_to_fit"),
            "chic_rt_vec_push" => Some("chic_rt_vec_push"),
            "chic_rt_vec_pop" => Some("chic_rt_vec_pop"),
            "chic_rt_vec_insert" => Some("chic_rt_vec_insert"),
            "chic_rt_vec_remove" => Some("chic_rt_vec_remove"),
            "chic_rt_vec_swap_remove" => Some("chic_rt_vec_swap_remove"),
            "chic_rt_vec_truncate" => Some("chic_rt_vec_truncate"),
            "chic_rt_vec_clear" => Some("chic_rt_vec_clear"),
            "chic_rt_vec_drop" => Some("chic_rt_vec_drop"),
            "chic_rt_vec_set_len" => Some("chic_rt_vec_set_len"),
            "chic_rt_vec_len" => Some("chic_rt_vec_len"),
            "chic_rt_vec_capacity" => Some("chic_rt_vec_capacity"),
            "chic_rt_vec_clone" => Some("chic_rt_vec_clone"),
            "chic_rt_vec_data" => Some("chic_rt_vec_data"),
            "chic_rt_vec_data_mut" => Some("chic_rt_vec_data_mut"),
            "chic_rt_vec_view" => Some("chic_rt_vec_view"),
            "chic_rt_vec_iter" => Some("chic_rt_vec_iter"),
            "chic_rt_vec_iter_next" => Some("chic_rt_vec_iter_next"),
            "chic_rt_vec_iter_next_ptr" => Some("chic_rt_vec_iter_next_ptr"),
            "chic_rt_vec_is_empty" => Some("chic_rt_vec_is_empty"),
            "chic_rt_char_from_codepoint" => Some("chic_rt_char_from_codepoint"),
            "chic_rt_char_is_digit" => Some("chic_rt_char_is_digit"),
            "chic_rt_char_is_letter" => Some("chic_rt_char_is_letter"),
            "chic_rt_char_is_scalar" => Some("chic_rt_char_is_scalar"),
            "chic_rt_char_is_whitespace" => Some("chic_rt_char_is_whitespace"),
            "chic_rt_char_status" => Some("chic_rt_char_status"),
            "chic_rt_char_to_lower" => Some("chic_rt_char_to_lower"),
            "chic_rt_char_to_upper" => Some("chic_rt_char_to_upper"),
            "chic_rt_char_value" => Some("chic_rt_char_value"),
            "chic_rt_thread_spawn" => Some("chic_rt_thread_spawn"),
            "chic_rt_thread_join" => Some("chic_rt_thread_join"),
            "chic_rt_thread_detach" => Some("chic_rt_thread_detach"),
            "chic_rt_thread_sleep_ms" => Some("chic_rt_thread_sleep_ms"),
            "chic_rt_thread_yield" => Some("chic_rt_thread_yield"),
            "chic_rt_thread_spin_wait" => Some("chic_rt_thread_spin_wait"),
            "chic_rt_startup_argv" => Some("chic_rt_startup_argv"),
            "chic_rt_startup_env" => Some("chic_rt_startup_env"),
            "chic_rt_atomic_bool_load" => Some("chic_rt_atomic_bool_load"),
            "chic_rt_atomic_bool_store" => Some("chic_rt_atomic_bool_store"),
            "chic_rt_atomic_bool_compare_exchange" => Some("chic_rt_atomic_bool_compare_exchange"),
            "chic_rt_atomic_usize_load" => Some("chic_rt_atomic_usize_load"),
            "chic_rt_atomic_usize_store" => Some("chic_rt_atomic_usize_store"),
            "chic_rt_atomic_usize_fetch_add" => Some("chic_rt_atomic_usize_fetch_add"),
            "chic_rt_atomic_usize_fetch_sub" => Some("chic_rt_atomic_usize_fetch_sub"),
            "chic_rt_atomic_i32_load" => Some("chic_rt_atomic_i32_load"),
            "chic_rt_atomic_i32_store" => Some("chic_rt_atomic_i32_store"),
            "chic_rt_atomic_i32_compare_exchange" => Some("chic_rt_atomic_i32_compare_exchange"),
            "chic_rt_atomic_i32_fetch_add" => Some("chic_rt_atomic_i32_fetch_add"),
            "chic_rt_atomic_i32_fetch_sub" => Some("chic_rt_atomic_i32_fetch_sub"),
            "chic_rt_atomic_u32_load" => Some("chic_rt_atomic_u32_load"),
            "chic_rt_atomic_u32_store" => Some("chic_rt_atomic_u32_store"),
            "chic_rt_atomic_u32_compare_exchange" => Some("chic_rt_atomic_u32_compare_exchange"),
            "chic_rt_atomic_u32_fetch_add" => Some("chic_rt_atomic_u32_fetch_add"),
            "chic_rt_atomic_u32_fetch_sub" => Some("chic_rt_atomic_u32_fetch_sub"),
            "chic_rt_atomic_i64_load" => Some("chic_rt_atomic_i64_load"),
            "chic_rt_atomic_i64_store" => Some("chic_rt_atomic_i64_store"),
            "chic_rt_atomic_i64_compare_exchange" => Some("chic_rt_atomic_i64_compare_exchange"),
            "chic_rt_atomic_i64_fetch_add" => Some("chic_rt_atomic_i64_fetch_add"),
            "chic_rt_atomic_i64_fetch_sub" => Some("chic_rt_atomic_i64_fetch_sub"),
            "chic_rt_atomic_u64_load" => Some("chic_rt_atomic_u64_load"),
            "chic_rt_atomic_u64_store" => Some("chic_rt_atomic_u64_store"),
            "chic_rt_atomic_u64_compare_exchange" => Some("chic_rt_atomic_u64_compare_exchange"),
            "chic_rt_atomic_u64_fetch_add" => Some("chic_rt_atomic_u64_fetch_add"),
            "chic_rt_atomic_u64_fetch_sub" => Some("chic_rt_atomic_u64_fetch_sub"),
            "chic_rt_arc_new" => Some("chic_rt_arc_new"),
            "chic_rt_arc_downgrade" => Some("chic_rt_arc_downgrade"),
            "chic_rt_arc_get" => Some("chic_rt_arc_get"),
            "chic_rt_arc_get_mut" => Some("chic_rt_arc_get_mut"),
            "chic_rt_arc_drop" => Some("chic_rt_arc_drop"),
            "chic_rt_arc_clone" => Some("chic_rt_arc_clone"),
            "chic_rt_weak_clone" => Some("chic_rt_weak_clone"),
            "chic_rt_weak_drop" => Some("chic_rt_weak_drop"),
            "chic_rt_weak_upgrade" => Some("chic_rt_weak_upgrade"),
            "chic_rt_rc_new" => Some("chic_rt_rc_new"),
            "chic_rt_rc_clone" => Some("chic_rt_rc_clone"),
            "chic_rt_rc_drop" => Some("chic_rt_rc_drop"),
            "chic_rt_rc_get" => Some("chic_rt_rc_get"),
            "chic_rt_rc_get_mut" => Some("chic_rt_rc_get_mut"),
            "chic_rt_rc_downgrade" => Some("chic_rt_rc_downgrade"),
            "chic_rt_rc_strong_count" => Some("chic_rt_rc_strong_count"),
            "chic_rt_rc_weak_count" => Some("chic_rt_rc_weak_count"),
            "chic_rt_weak_rc_clone" => Some("chic_rt_weak_rc_clone"),
            "chic_rt_weak_rc_drop" => Some("chic_rt_weak_rc_drop"),
            "chic_rt_weak_rc_upgrade" => Some("chic_rt_weak_rc_upgrade"),
            "chic_rt_mutex_create" => Some("chic_rt_mutex_create"),
            "chic_rt_mutex_destroy" => Some("chic_rt_mutex_destroy"),
            "chic_rt_mutex_lock" => Some("chic_rt_mutex_lock"),
            "chic_rt_mutex_try_lock" => Some("chic_rt_mutex_try_lock"),
            "chic_rt_mutex_unlock" => Some("chic_rt_mutex_unlock"),
            "chic_rt_lock_create" => Some("chic_rt_lock_create"),
            "chic_rt_lock_destroy" => Some("chic_rt_lock_destroy"),
            "chic_rt_lock_enter" => Some("chic_rt_lock_enter"),
            "chic_rt_lock_try_enter" => Some("chic_rt_lock_try_enter"),
            "chic_rt_lock_exit" => Some("chic_rt_lock_exit"),
            "chic_rt_lock_is_held" => Some("chic_rt_lock_is_held"),
            "chic_rt_lock_is_held_by_current_thread" => {
                Some("chic_rt_lock_is_held_by_current_thread")
            }
            "chic_rt_rwlock_create" => Some("chic_rt_rwlock_create"),
            "chic_rt_rwlock_destroy" => Some("chic_rt_rwlock_destroy"),
            "chic_rt_rwlock_read_lock" => Some("chic_rt_rwlock_read_lock"),
            "chic_rt_rwlock_try_read_lock" => Some("chic_rt_rwlock_try_read_lock"),
            "chic_rt_rwlock_read_unlock" => Some("chic_rt_rwlock_read_unlock"),
            "chic_rt_rwlock_write_lock" => Some("chic_rt_rwlock_write_lock"),
            "chic_rt_rwlock_try_write_lock" => Some("chic_rt_rwlock_try_write_lock"),
            "chic_rt_rwlock_write_unlock" => Some("chic_rt_rwlock_write_unlock"),
            "chic_rt_condvar_create" => Some("chic_rt_condvar_create"),
            "chic_rt_condvar_destroy" => Some("chic_rt_condvar_destroy"),
            "chic_rt_condvar_notify_one" => Some("chic_rt_condvar_notify_one"),
            "chic_rt_condvar_notify_all" => Some("chic_rt_condvar_notify_all"),
            "chic_rt_condvar_wait" => Some("chic_rt_condvar_wait"),
            "chic_rt_once_create" => Some("chic_rt_once_create"),
            "chic_rt_once_destroy" => Some("chic_rt_once_destroy"),
            "chic_rt_once_try_begin" => Some("chic_rt_once_try_begin"),
            "chic_rt_once_complete" => Some("chic_rt_once_complete"),
            "chic_rt_once_wait" => Some("chic_rt_once_wait"),
            "chic_rt_once_is_completed" => Some("chic_rt_once_is_completed"),
            "chic_rt_hashset_new" => Some("chic_rt_hashset_new"),
            "chic_rt_hashset_with_capacity" => Some("chic_rt_hashset_with_capacity"),
            "chic_rt_hashset_drop" => Some("chic_rt_hashset_drop"),
            "chic_rt_hashset_clear" => Some("chic_rt_hashset_clear"),
            "chic_rt_hashset_reserve" => Some("chic_rt_hashset_reserve"),
            "chic_rt_hashset_shrink_to" => Some("chic_rt_hashset_shrink_to"),
            "chic_rt_hashset_len" => Some("chic_rt_hashset_len"),
            "chic_rt_hashset_capacity" => Some("chic_rt_hashset_capacity"),
            "chic_rt_hashset_tombstones" => Some("chic_rt_hashset_tombstones"),
            "chic_rt_hashset_insert" => Some("chic_rt_hashset_insert"),
            "chic_rt_hashset_replace" => Some("chic_rt_hashset_replace"),
            "chic_rt_hashset_contains" => Some("chic_rt_hashset_contains"),
            "chic_rt_hashset_get_ptr" => Some("chic_rt_hashset_get_ptr"),
            "chic_rt_hashset_take" => Some("chic_rt_hashset_take"),
            "chic_rt_hashset_remove" => Some("chic_rt_hashset_remove"),
            "chic_rt_hashset_take_at" => Some("chic_rt_hashset_take_at"),
            "chic_rt_hashset_bucket_state" => Some("chic_rt_hashset_bucket_state"),
            "chic_rt_hashset_bucket_hash" => Some("chic_rt_hashset_bucket_hash"),
            "chic_rt_hashset_iter" => Some("chic_rt_hashset_iter"),
            "chic_rt_hashset_iter_next" => Some("chic_rt_hashset_iter_next"),
            "chic_rt_hashset_iter_next_ptr" => Some("chic_rt_hashset_iter_next_ptr"),
            "chic_rt_hashmap_new" => Some("chic_rt_hashmap_new"),
            "chic_rt_hashmap_with_capacity" => Some("chic_rt_hashmap_with_capacity"),
            "chic_rt_hashmap_drop" => Some("chic_rt_hashmap_drop"),
            "chic_rt_hashmap_clear" => Some("chic_rt_hashmap_clear"),
            "chic_rt_hashmap_reserve" => Some("chic_rt_hashmap_reserve"),
            "chic_rt_hashmap_shrink_to" => Some("chic_rt_hashmap_shrink_to"),
            "chic_rt_hashmap_len" => Some("chic_rt_hashmap_len"),
            "chic_rt_hashmap_capacity" => Some("chic_rt_hashmap_capacity"),
            "chic_rt_hashmap_insert" => Some("chic_rt_hashmap_insert"),
            "chic_rt_hashmap_contains" => Some("chic_rt_hashmap_contains"),
            "chic_rt_hashmap_get_ptr" => Some("chic_rt_hashmap_get_ptr"),
            "chic_rt_hashmap_take" => Some("chic_rt_hashmap_take"),
            "chic_rt_hashmap_remove" => Some("chic_rt_hashmap_remove"),
            "chic_rt_hashmap_bucket_state" => Some("chic_rt_hashmap_bucket_state"),
            "chic_rt_hashmap_bucket_hash" => Some("chic_rt_hashmap_bucket_hash"),
            "chic_rt_hashmap_take_at" => Some("chic_rt_hashmap_take_at"),
            "chic_rt_hashmap_iter" => Some("chic_rt_hashmap_iter"),
            "chic_rt_hashmap_iter_next" => Some("chic_rt_hashmap_iter_next"),
            _ => None,
        } {
            self.externals.insert(sym);
        }

        if let Some(c_abi) = signature.c_abi.as_ref()
            && matches!(c_abi.ret, CAbiReturn::IndirectSret { .. })
        {
            return self.emit_direct_call_sret(signature, args, destination, target, unwind);
        }

        let rendered = if signature.c_abi.is_some() {
            render_args_for_c_abi_params(self, signature, args, "direct call", 0)?
        } else {
            render_args_for_signature(self, signature, args, "direct call")?
        };
        if let Some(dest) = destination {
            let dest_ty = self
                .place_type(dest)?
                .ok_or_else(|| Error::Codegen("call destination missing type".into()))?;
            if dest_ty == "void" {
                return Err(Error::Codegen(
                    "void call cannot have an assignment destination in LLVM backend".into(),
                ));
            }
            let call_ret_ty = signature.ret.clone().unwrap_or_else(|| dest_ty.clone());
            let callee_repr = if signature.variadic {
                let params_proto = if signature.params.is_empty() {
                    "...".to_string()
                } else {
                    format!("{}, ...", signature.params.join(", "))
                };
                format!("({params_proto}) @{}", signature.symbol)
            } else {
                format!("@{}", signature.symbol)
            };
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = call {call_ret_ty} {callee_repr}({})",
                rendered.repr
            )
            .ok();
            if let Some(c_abi) = signature.c_abi.as_ref() {
                self.store_direct_c_abi_return(
                    dest,
                    &ValueRef::new(tmp.clone(), &call_ret_ty),
                    &c_abi.ret,
                )?;
            } else {
                let mut value = ValueRef::new(tmp.clone(), &call_ret_ty);
                if dest_ty != call_ret_ty {
                    let is_int_like = |ty: &str| {
                        ty.starts_with('i') && ty.chars().skip(1).all(|c| c.is_ascii_digit())
                    };
                    let is_ptr_like = |ty: &str| ty == "ptr" || ty.ends_with('*');
                    let cast_tmp = self.new_temp();
                    if dest_ty == "ptr" && is_int_like(&call_ret_ty) {
                        writeln!(
                            &mut self.builder,
                            "  {cast_tmp} = inttoptr {call_ret_ty} {tmp} to ptr"
                        )
                        .ok();
                        value = ValueRef::new(cast_tmp, "ptr");
                    } else if is_ptr_like(&call_ret_ty) && is_int_like(&dest_ty) {
                        writeln!(
                            &mut self.builder,
                            "  {cast_tmp} = ptrtoint ptr {tmp} to {dest_ty}"
                        )
                        .ok();
                        value = ValueRef::new(cast_tmp, &dest_ty);
                    } else if call_ret_ty.starts_with("ptr")
                        && (dest_ty.starts_with('{') || dest_ty.starts_with('['))
                    {
                        writeln!(
                            &mut self.builder,
                            "  {cast_tmp} = load {dest_ty}, ptr {tmp}"
                        )
                        .ok();
                        value = ValueRef::new(cast_tmp, &dest_ty);
                    } else if (call_ret_ty.starts_with('{') || call_ret_ty.starts_with('['))
                        && (dest_ty.starts_with('{') || dest_ty.starts_with('['))
                    {
                        // When the callee returns an aggregate with a different LLVM
                        // structural type than the destination, spill and reload to
                        // allow LLVM to handle the representation safely.
                        let spill_ptr = self.new_temp();
                        writeln!(&mut self.builder, "  {spill_ptr} = alloca {call_ret_ty}").ok();
                        writeln!(
                            &mut self.builder,
                            "  store {call_ret_ty} {tmp}, ptr {spill_ptr}"
                        )
                        .ok();
                        let cast_ptr = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {cast_ptr} = bitcast {call_ret_ty}* {spill_ptr} to {dest_ty}*"
                        )
                        .ok();
                        let load_tmp = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {load_tmp} = load {dest_ty}, ptr {cast_ptr}"
                        )
                        .ok();
                        value = ValueRef::new(load_tmp, &dest_ty);
                    } else if call_ret_ty.starts_with('i') && dest_ty.starts_with('i') {
                        let from_bits = call_ret_ty
                            .strip_prefix('i')
                            .and_then(|b| b.parse::<u32>().ok())
                            .unwrap_or(0);
                        let to_bits = dest_ty
                            .strip_prefix('i')
                            .and_then(|b| b.parse::<u32>().ok())
                            .unwrap_or(0);
                        if to_bits > from_bits {
                            writeln!(
                                &mut self.builder,
                                "  {cast_tmp} = zext {call_ret_ty} {tmp} to {dest_ty}"
                            )
                            .ok();
                        } else if to_bits < from_bits && to_bits > 0 {
                            writeln!(
                                &mut self.builder,
                                "  {cast_tmp} = trunc {call_ret_ty} {tmp} to {dest_ty}"
                            )
                            .ok();
                        } else {
                            writeln!(
                                &mut self.builder,
                                "  {cast_tmp} = bitcast {call_ret_ty} {tmp} to {dest_ty}"
                            )
                            .ok();
                        }
                        value = ValueRef::new(cast_tmp, &dest_ty);
                    } else {
                        writeln!(
                            &mut self.builder,
                            "  {cast_tmp} = bitcast {call_ret_ty} {tmp} to {dest_ty}"
                        )
                        .ok();
                        value = ValueRef::new(cast_tmp, &dest_ty);
                    }
                }
                self.store_place(dest, &value)?;
            }
        } else {
            let ret_ty = signature.ret.clone().unwrap_or_else(|| "void".to_string());
            let callee_repr = if signature.variadic {
                let params_proto = if signature.params.is_empty() {
                    "...".to_string()
                } else {
                    format!("{}, ...", signature.params.join(", "))
                };
                format!("({params_proto}) @{}", signature.symbol)
            } else {
                format!("@{}", signature.symbol)
            };
            writeln!(
                &mut self.builder,
                "  call {ret_ty} {callee_repr}({})",
                rendered.repr
            )
            .ok();
        }

        self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
        Ok(())
    }

    fn emit_direct_call_sret(
        &mut self,
        signature: &LlvmFunctionSignature,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
        unwind: Option<BlockId>,
    ) -> Result<(), Error> {
        let dest_label = self.block_label(target)?;
        let Some(c_abi) = signature.c_abi.as_ref() else {
            return Err(Error::Codegen(
                "emit_direct_call_sret called without C ABI metadata".into(),
            ));
        };
        let CAbiReturn::IndirectSret { ty, align } = &c_abi.ret else {
            return Err(Error::Codegen(
                "emit_direct_call_sret called for non-sret signature".into(),
            ));
        };

        if signature.ret.is_some() {
            return Err(Error::Codegen(format!(
                "sret call expected void LLVM return for `{}`",
                signature.symbol
            )));
        }

        let sret_ptr = if let Some(dest) = destination {
            self.place_ptr(dest)?
        } else {
            let ret_llvm_ty = map_type_owned(ty, Some(self.type_layouts))?.ok_or_else(|| {
                Error::Codegen(format!(
                    "sret call return type `{}` lowered to void LLVM type",
                    ty.canonical_name()
                ))
            })?;
            let tmp_ptr = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp_ptr} = alloca {ret_llvm_ty}, align {align}"
            )
            .ok();
            tmp_ptr
        };

        let rendered = render_args_for_c_abi_params(self, signature, args, "direct sret call", 1)?;
        let sret_attrs = signature
            .param_attrs
            .get(0)
            .map(|attrs| {
                if attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", attrs.join(" "))
                }
            })
            .unwrap_or_default();
        let sret_arg = format!("ptr{sret_attrs} {sret_ptr}");
        let arg_list = if rendered.repr.is_empty() {
            sret_arg
        } else {
            format!("{sret_arg}, {}", rendered.repr)
        };

        writeln!(
            &mut self.builder,
            "  call void @{}({})",
            signature.symbol, arg_list
        )
        .ok();
        self.emit_branch_to_labels_or_unwind(dest_label.as_str(), unwind)?;
        Ok(())
    }

    fn ensure_std_runtime_intrinsic_owner(&self, callee: &str, symbol: &str) -> Result<(), Error> {
        const SPAN_PREFIX: &str = concat!("chic_rt_", "span_");
        const VEC_PREFIX: &str = concat!("chic_rt_", "vec_");
        const ARRAY_PREFIX: &str = concat!("chic_rt_", "array_");
        if symbol.starts_with(SPAN_PREFIX) {
            if !callee.starts_with("Std::Span::SpanIntrinsics::")
                && !callee.starts_with("Std::Runtime::Native::SpanRuntime::")
            {
                return Err(Error::Codegen(format!(
                    "span runtime intrinsic `{symbol}` must be routed through Std.Span.SpanIntrinsics (callee `{callee}`)"
                )));
            }
        }
        if symbol.starts_with(VEC_PREFIX) {
            let allowed = callee.starts_with("Foundation::Collections::VecIntrinsics::")
                || callee.starts_with("Std::Collections::VecIntrinsics::")
                || callee.starts_with("Std::Runtime::Native::VecRuntime::");
            if !allowed {
                return Err(Error::Codegen(format!(
                    "vec runtime intrinsic `{symbol}` must be routed through Std/Foundations VecIntrinsics (callee `{callee}`)"
                )));
            }
        }
        if symbol.starts_with(ARRAY_PREFIX) {
            let allowed = callee.starts_with("Foundation::Collections::VecIntrinsics::")
                || callee.starts_with("Std::Collections::VecIntrinsics::")
                || callee.starts_with("Std::Runtime::Native::VecRuntime::");
            if !allowed {
                return Err(Error::Codegen(format!(
                    "array runtime intrinsic `{symbol}` must be routed through a Std collections entrypoint (callee `{callee}`)"
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn call_operand_fn_ty(&self, operand: &Operand) -> Option<FnTy> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.local_fn_ty(place),
            Operand::Borrow(borrow) => self.local_fn_ty(&borrow.place),
            _ => None,
        }
    }

    pub(crate) fn local_fn_ty(&self, place: &Place) -> Option<FnTy> {
        if !place.projection.is_empty() {
            return None;
        }
        let decl = self.function.body.locals.get(place.local.0)?;
        match &decl.ty {
            Ty::Fn(fn_ty) => Some(fn_ty.clone()),
            Ty::Nullable(inner) => match inner.as_ref() {
                Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                Ty::Named(named) => self
                    .type_layouts
                    .delegate_signature(&named.canonical_path())
                    .cloned(),
                _ => None,
            },
            Ty::Named(named) => self
                .type_layouts
                .delegate_signature(&named.canonical_path())
                .cloned(),
            _ => None,
        }
    }

    pub(crate) fn try_emit_intrinsic_call(
        &mut self,
        repr: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<bool, Error> {
        let canonical = canonical_function_name(repr);
        let canonical_leaf = canonical
            .rsplit("::")
            .next()
            .unwrap_or_else(|| canonical.as_str());
        match canonical_leaf {
            "std::simd::f32x8::fma" => {
                self.emit_simd_f32x8_fma(args, destination, target)?;
                return Ok(true);
            }
            "std::simd::f32x4::fma" => {
                self.emit_simd_f32x4_fma(args, destination, target)?;
                return Ok(true);
            }
            "std::simd::f16x8::fma" => {
                self.emit_simd_f16x8_fma(args, destination, target)?;
                return Ok(true);
            }
            "std::linalg::int8x64::dpbusd" | "std::linalg::int8x64::mmla" => {
                self.emit_linalg_dpbusd(args, destination, target)?;
                return Ok(true);
            }
            "std::linalg::bf16x32::mmla" => {
                self.emit_linalg_bf16_mmla(args, destination, target, false)?;
                return Ok(true);
            }
            "std::linalg::bf16x32::sme_mmla" => {
                self.emit_linalg_bf16_mmla(args, destination, target, true)?;
                return Ok(true);
            }
            _ => {}
        }
        if self.try_emit_decimal_call(&canonical, repr, args, destination, target)? {
            return Ok(true);
        }
        Ok(false)
    }

    fn try_emit_native_runtime_helper(
        &mut self,
        repr: &str,
    ) -> Result<Option<LlvmFunctionSignature>, Error> {
        let canonical = canonical_function_name(repr);
        if canonical == "chic_rt_take_pending_exception" {
            self.externals.insert("chic_rt_take_pending_exception");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_take_pending_exception".to_string(),
                ret: Some("i32".to_string()),
                params: vec!["ptr".to_string(), "ptr".to_string()],
                param_attrs: vec![Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::chic_rt_ptr_to_isize") {
            self.externals.insert("chic_rt_ptr_to_isize");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_ptr_to_isize".to_string(),
                ret: Some("i64".to_string()),
                params: vec!["ptr".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::chic_rt_null_mut") {
            self.externals.insert("chic_rt_null_mut");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_null_mut".to_string(),
                ret: Some("ptr".to_string()),
                params: vec![],
                param_attrs: vec![],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::chic_rt_null_const") {
            self.externals.insert("chic_rt_null_const");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_null_const".to_string(),
                ret: Some("ptr".to_string()),
                params: vec![],
                param_attrs: vec![],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::chic_rt_const_ptr_to_isize") {
            self.externals.insert("chic_rt_const_ptr_to_isize");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_const_ptr_to_isize".to_string(),
                ret: Some("i64".to_string()),
                params: vec!["ptr".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::chic_rt_isize_to_mut_ptr") {
            self.externals.insert("chic_rt_isize_to_mut_ptr");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_isize_to_mut_ptr".to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["i64".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::chic_rt_isize_to_const_ptr") {
            self.externals.insert("chic_rt_isize_to_const_ptr");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_isize_to_const_ptr".to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["i64".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical == "chic_rt_hash_invoke" || canonical.ends_with("::chic_rt_hash_invoke") {
            self.externals.insert("chic_rt_hash_invoke");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_hash_invoke".to_string(),
                ret: Some("i64".to_string()),
                params: vec!["i64".to_string(), "ptr".to_string()],
                param_attrs: vec![Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical == "chic_rt_eq_invoke" || canonical.ends_with("::chic_rt_eq_invoke") {
            self.externals.insert("chic_rt_eq_invoke");
            return Ok(Some(LlvmFunctionSignature {
                symbol: "chic_rt_eq_invoke".to_string(),
                ret: Some("i32".to_string()),
                params: vec!["i64".to_string(), "ptr".to_string(), "ptr".to_string()],
                param_attrs: vec![Vec::new(), Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::posix_memalign") {
            return Ok(Some(LlvmFunctionSignature {
                symbol: "posix_memalign".to_string(),
                ret: Some("i32".to_string()),
                params: vec!["ptr".to_string(), "i64".to_string(), "i64".to_string()],
                param_attrs: vec![Vec::new(), Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::malloc") {
            return Ok(Some(LlvmFunctionSignature {
                symbol: "malloc".to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["i64".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::calloc") {
            return Ok(Some(LlvmFunctionSignature {
                symbol: "calloc".to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["i64".to_string(), "i64".to_string()],
                param_attrs: vec![Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::realloc") {
            return Ok(Some(LlvmFunctionSignature {
                symbol: "realloc".to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["ptr".to_string(), "i64".to_string()],
                param_attrs: vec![Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::free") {
            return Ok(Some(LlvmFunctionSignature {
                symbol: "free".to_string(),
                ret: None,
                params: vec!["ptr".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::memcpy") || canonical.ends_with("::memmove") {
            let sym = if canonical.ends_with("::memcpy") {
                "memcpy"
            } else {
                "memmove"
            };
            return Ok(Some(LlvmFunctionSignature {
                symbol: sym.to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["ptr".to_string(), "ptr".to_string(), "i64".to_string()],
                param_attrs: vec![Vec::new(), Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        if canonical.ends_with("::memset") {
            return Ok(Some(LlvmFunctionSignature {
                symbol: "memset".to_string(),
                ret: Some("ptr".to_string()),
                params: vec!["ptr".to_string(), "i8".to_string(), "i64".to_string()],
                param_attrs: vec![Vec::new(), Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            }));
        }
        Ok(None)
    }
}
