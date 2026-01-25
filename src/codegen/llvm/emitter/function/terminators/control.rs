use std::collections::HashSet;
use std::fmt::Write;

use crate::abi::CAbiReturn;
use crate::error::Error;
use crate::mir::AsyncStateMachine;
use crate::mir::TypeLayout;
use crate::mir::async_types::is_task_ty;
use crate::mir::casts::pointer_depth;
use crate::mir::{
    BlockId, ClassLayoutKind, ConstValue, GenericArg, LocalId, MatchArm, Operand, Pattern, Place,
    PointerTy, ProjectionElem, Terminator, Ty, class_vtable_symbol_name,
};

use super::super::builder::FunctionEmitter;
use crate::async_flags::{FUTURE_FLAG_CANCELLED, FUTURE_FLAG_COMPLETED, FUTURE_FLAG_READY};
use crate::codegen::llvm::types::map_type_owned;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_terminator(&mut self, terminator: &Terminator) -> Result<(), Error> {
        match terminator {
            Terminator::Return => self.emit_return(),
            Terminator::Goto { target } => {
                let label = self.block_label(*target)?;
                writeln!(&mut self.builder, "  br label %{label}").ok();
                Ok(())
            }
            Terminator::SwitchInt {
                discr,
                targets,
                otherwise,
            } => self.emit_switch(discr, targets, *otherwise),
            Terminator::Match {
                value,
                arms,
                otherwise,
            } => self.emit_match(value, arms, *otherwise),
            Terminator::Call {
                func,
                args,
                arg_modes: _,
                destination,
                target,
                unwind,
                dispatch,
            } => self.emit_call(
                func,
                args,
                destination.as_ref(),
                *target,
                *unwind,
                dispatch.as_ref(),
            ),
            Terminator::Throw { exception, ty } => {
                self.emit_throw(exception, ty)?;
                Ok(())
            }
            Terminator::Yield {
                value,
                resume,
                drop,
            } => self.emit_yield(value, *resume, *drop),
            Terminator::Await {
                future,
                destination,
                resume,
                drop,
            } => self.emit_await(future, destination.as_ref(), *resume, *drop),
            Terminator::Panic => {
                self.externals.insert("llvm.trap");
                writeln!(&mut self.builder, "  call void @llvm.trap()").ok();
                writeln!(&mut self.builder, "  unreachable").ok();
                Ok(())
            }
            Terminator::Unreachable => {
                writeln!(&mut self.builder, "  unreachable").ok();
                Ok(())
            }
            Terminator::Pending(pending) => Err(Error::Codegen(format!(
                "pending terminator {:?} cannot be lowered to LLVM",
                pending.kind
            ))),
        }
    }

    fn const_to_i128(value: &ConstValue) -> Option<i128> {
        match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => Some(*v),
            ConstValue::UInt(v) => Some(*v as i128),
            ConstValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            ConstValue::Char(ch) => Some(*ch as i128),
            ConstValue::Enum { discriminant, .. } => Some(*discriminant),
            _ => None,
        }
    }

    fn emit_match(
        &mut self,
        value: &Place,
        arms: &[MatchArm],
        otherwise: BlockId,
    ) -> Result<(), Error> {
        if let [arm] = arms {
            if arm.guard.is_none() && arm.bindings.is_empty() {
                match &arm.pattern {
                    Pattern::Wildcard | Pattern::Binding(_) => {
                        let target = self.block_label(arm.target)?;
                        writeln!(&mut self.builder, "  br label %{target}").ok();
                        return Ok(());
                    }
                    Pattern::Type(target_ty) => {
                        return self.emit_match_type(value, target_ty, arm.target, otherwise);
                    }
                    Pattern::Literal(_)
                    | Pattern::Tuple(_)
                    | Pattern::Struct { .. }
                    | Pattern::Enum { .. } => {}
                }
            }
        }

        let discr_operand = Operand::Copy(value.clone());
        let discr_val = self.emit_operand(&discr_operand, None)?;
        let discr_ty = discr_val.ty().to_string();
        let mut targets = Vec::new();
        let mut default = otherwise;
        for arm in arms {
            if arm.guard.is_some() || !arm.bindings.is_empty() {
                default = arm.target;
                continue;
            }
            match &arm.pattern {
                Pattern::Wildcard => {
                    default = arm.target;
                }
                Pattern::Literal(lit) => {
                    let Some(value) = Self::const_to_i128(lit) else {
                        default = arm.target;
                        continue;
                    };
                    targets.push((value, arm.target));
                }
                _ => {
                    default = arm.target;
                }
            }
        }
        if targets.is_empty() {
            let default_label = self.block_label(default)?;
            writeln!(&mut self.builder, "  br label %{default_label}").ok();
            return Ok(());
        }
        let default_label = self.block_label(default)?;
        writeln!(
            &mut self.builder,
            "  switch {discr_ty} {}, label %{default_label} [",
            discr_val.repr()
        )
        .ok();
        for (value, target) in targets {
            let label = self.block_label(target)?;
            writeln!(&mut self.builder, "    {discr_ty} {value} , label %{label}").ok();
        }
        writeln!(&mut self.builder, "  ]").ok();
        Ok(())
    }

    fn emit_match_type(
        &mut self,
        value: &Place,
        target_ty: &Ty,
        target: BlockId,
        otherwise: BlockId,
    ) -> Result<(), Error> {
        let discr_operand = Operand::Copy(value.clone());
        let discr_val = self.emit_operand(&discr_operand, None)?;
        let discr_repr = discr_val.repr().to_string();

        let canonical = target_ty.canonical_name();
        let target_name = canonical
            .split('<')
            .next()
            .unwrap_or(&canonical)
            .replace('.', "::");
        let target_key = self
            .type_layouts
            .resolve_type_key(&target_name)
            .unwrap_or(target_name.as_str());

        let match_exception_base = matches!(
            target_key,
            "Exception" | "Std::Exception" | "System::Exception"
        );

        let mut accepted = HashSet::<String>::new();
        if match_exception_base {
            for candidate in self.type_layouts.types.keys() {
                if let Some(info) = self.type_layouts.class_layout_info(candidate) {
                    if info.kind == ClassLayoutKind::Error {
                        accepted.insert(candidate.clone());
                    }
                }
            }
        } else {
            accepted.insert(target_key.to_string());
            loop {
                let mut changed = false;
                for candidate in self.type_layouts.types.keys() {
                    if accepted.contains(candidate) {
                        continue;
                    }
                    let Some(info) = self.type_layouts.class_layout_info(candidate) else {
                        continue;
                    };
                    if info.bases.iter().any(|base| accepted.contains(base)) {
                        accepted.insert(candidate.clone());
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
        }

        let mut vtables = accepted
            .into_iter()
            .filter_map(
                |candidate| match self.type_layouts.layout_for_name(&candidate) {
                    Some(TypeLayout::Class(_)) => Some(class_vtable_symbol_name(&candidate)),
                    _ => None,
                },
            )
            .collect::<Vec<_>>();
        vtables.sort();
        vtables.dedup();

        if vtables.is_empty() {
            let default_label = self.block_label(otherwise)?;
            writeln!(&mut self.builder, "  br label %{default_label}").ok();
            return Ok(());
        }

        let is_null = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {is_null} = icmp eq ptr {discr_repr}, null"
        )
        .ok();

        let non_null_label = self.new_internal_label("match_type_non_null");
        let target_label = self.block_label(target)?;
        let otherwise_label = self.block_label(otherwise)?;
        writeln!(
            &mut self.builder,
            "  br i1 {is_null}, label %{otherwise_label}, label %{non_null_label}"
        )
        .ok();

        writeln!(&mut self.builder, "{non_null_label}:").ok();

        let mut vtable_place = value.clone();
        vtable_place.projection.push(ProjectionElem::Deref);
        vtable_place
            .projection
            .push(ProjectionElem::FieldNamed("$vtable".into()));
        let vtable_val = self.emit_operand(&Operand::Copy(vtable_place), None)?;
        let vtable_repr = vtable_val.repr().to_string();

        let mut predicate = None::<String>;
        for symbol in &vtables {
            let cmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cmp} = icmp eq ptr {vtable_repr}, @{symbol}"
            )
            .ok();
            predicate = Some(match predicate {
                None => cmp,
                Some(prev) => {
                    let merged = self.new_temp();
                    writeln!(&mut self.builder, "  {merged} = or i1 {prev}, {cmp}").ok();
                    merged
                }
            });
        }

        let cmp = predicate.unwrap_or_else(|| "false".to_string());
        writeln!(
            &mut self.builder,
            "  br i1 {cmp}, label %{target_label}, label %{otherwise_label}"
        )
        .ok();

        Ok(())
    }

    fn emit_await(
        &mut self,
        future: &Place,
        destination: Option<&Place>,
        resume: BlockId,
        drop: BlockId,
    ) -> Result<(), Error> {
        let future_ty = self.mir_ty_of_place(future)?;
        let is_task = is_task_ty(&future_ty);
        let destination_ty = destination
            .as_ref()
            .map(|place| self.mir_ty_of_place(place))
            .transpose()?;
        let header_ptr = self.future_header_ptr(future)?;
        self.externals.insert("chic_rt_await");
        let status = self.new_temp();
        let ctx_ptr = self.async_runtime_context_ptr()?;
        let ctx_operand = ctx_ptr
            .map(|ptr| format!("ptr {ptr}"))
            .unwrap_or_else(|| "ptr null".to_string());
        writeln!(
            &mut self.builder,
            "  {status} = call i32 @chic_rt_await({ctx_operand}, ptr {header_ptr})"
        )
        .ok();
        let ready_flag = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {ready_flag} = icmp eq i32 {status}, 1"
        )
        .ok();
        let resume_label = self.block_label(resume)?;
        let drop_label = self.block_label(drop)?;
        if let Some(place) = destination {
            if matches!(destination_ty, Some(Ty::Unit)) {
                writeln!(
                    &mut self.builder,
                    "  br i1 {ready_flag}, label %{resume_label}, label %{drop_label}"
                )
                .ok();
                return Ok(());
            }
            if is_task {
                if let Some(result_ty) = self.task_result_ty(&future_ty) {
                    let ready_block = self.new_internal_label("await_ready");
                    let pending_block = self.new_internal_label("await_pending");
                    writeln!(
                        &mut self.builder,
                        "  br i1 {ready_flag}, label %{ready_block}, label %{pending_block}"
                    )
                    .ok();
                    writeln!(&mut self.builder, "{ready_block}:").ok();
                    self.store_task_result(future, &result_ty, place)?;
                    writeln!(&mut self.builder, "  br label %{resume_label}").ok();
                    writeln!(&mut self.builder, "{pending_block}:").ok();
                    writeln!(&mut self.builder, "  br label %{drop_label}").ok();
                    return Ok(());
                }
            } else if let Some(layout) = self.future_result_layout(&future_ty)? {
                let ready_block = self.new_internal_label("await_ready");
                let pending_block = self.new_internal_label("await_pending");
                writeln!(
                    &mut self.builder,
                    "  br i1 {ready_flag}, label %{ready_block}, label %{pending_block}"
                )
                .ok();
                writeln!(&mut self.builder, "{ready_block}:").ok();
                self.store_future_result(future, &layout, place)?;
                writeln!(&mut self.builder, "  br label %{resume_label}").ok();
                writeln!(&mut self.builder, "{pending_block}:").ok();
                writeln!(&mut self.builder, "  br label %{drop_label}").ok();
                return Ok(());
            }
        }
        writeln!(
            &mut self.builder,
            "  br i1 {ready_flag}, label %{resume_label}, label %{drop_label}"
        )
        .ok();
        Ok(())
    }

    fn emit_yield(&mut self, value: &Operand, resume: BlockId, drop: BlockId) -> Result<(), Error> {
        self.externals.insert("chic_rt_yield");
        let _ = value;
        let status = self.new_temp();
        let ctx_ptr = self.async_runtime_context_ptr()?;
        let ctx_operand = ctx_ptr
            .map(|ptr| format!("ptr {ptr}"))
            .unwrap_or_else(|| "ptr null".to_string());
        writeln!(
            &mut self.builder,
            "  {status} = call i32 @chic_rt_yield({ctx_operand})"
        )
        .ok();
        let ready_flag = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {ready_flag} = icmp eq i32 {status}, 1"
        )
        .ok();
        let resume_label = self.block_label(resume)?;
        let drop_label = self.block_label(drop)?;
        writeln!(
            &mut self.builder,
            "  br i1 {ready_flag}, label %{resume_label}, label %{drop_label}"
        )
        .ok();
        Ok(())
    }

    pub(crate) fn emit_throw(
        &mut self,
        exception: &Option<Operand>,
        ty: &Option<Ty>,
    ) -> Result<(), Error> {
        let payload_repr = if let Some(value) = exception {
            let operand_ty = self
                .operand_type(value)?
                .unwrap_or_else(|| "i32".to_string());
            let value_ref = self.emit_operand(value, Some(operand_ty.as_str()))?;
            match operand_ty.as_str() {
                "i64" => value_ref.repr().to_string(),
                "i32" => {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = zext i32 {} to i64",
                        value_ref.repr()
                    )
                    .ok();
                    tmp
                }
                ty if ty == "ptr" || pointer_depth(ty) > 0 => {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = ptrtoint {ty} {} to i64",
                        value_ref.repr()
                    )
                    .ok();
                    tmp
                }
                other => {
                    return Err(Error::Codegen(format!(
                        "throw operand type `{other}` is not supported in LLVM backend"
                    )));
                }
            }
        } else {
            "0".into()
        };

        let type_id = ty
            .as_ref()
            .map(|ty| crate::runtime::error::exception_type_identity(&ty.canonical_name()))
            .unwrap_or(0);

        self.externals.insert("chic_rt_throw");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_throw(i64 {payload_repr}, i64 {type_id})"
        )
        .ok();
        if self.current_fn_uses_sret() {
            writeln!(&mut self.builder, "  ret void").ok();
            return Ok(());
        }
        let ret_mir_ty = &self.function.signature.ret;
        if matches!(ret_mir_ty, Ty::Unit) {
            writeln!(&mut self.builder, "  ret void").ok();
            return Ok(());
        }
        let llvm_ret_ty =
            map_type_owned(ret_mir_ty, Some(self.type_layouts))?.unwrap_or_else(|| "i32".into());
        let default_value = if llvm_ret_ty == "ptr" || llvm_ret_ty.ends_with('*') {
            "null".to_string()
        } else if llvm_ret_ty.starts_with('i') {
            "0".to_string()
        } else if matches!(
            llvm_ret_ty.as_str(),
            "float" | "double" | "half" | "bfloat" | "fp128"
        ) {
            if llvm_ret_ty == "fp128" {
                "0xL00000000000000000000000000000000".to_string()
            } else {
                "0.0".to_string()
            }
        } else {
            "zeroinitializer".to_string()
        };
        writeln!(&mut self.builder, "  ret {llvm_ret_ty} {default_value}").ok();
        Ok(())
    }

    pub(crate) fn emit_return(&mut self) -> Result<(), Error> {
        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
            eprintln!(
                "[chic-debug] emit_return for {} is_async={} async_machine={} async_result={:?}",
                self.function.name,
                self.function.is_async,
                self.function.body.async_machine.is_some(),
                self.function.async_result
            );
        }
        if self.function.body.async_machine.is_some() && self.ready_task_return()? {
            return Ok(());
        }
        self.emit_trace_exit();
        if self.current_fn_uses_sret() {
            writeln!(&mut self.builder, "  ret void").ok();
            return Ok(());
        }
        if let Some(ret_idx) = self.return_local
            && let Some(ret_ty) = self
                .local_tys
                .get(ret_idx)
                .and_then(|ty| ty.as_ref())
                .cloned()
        {
            if ret_ty == "void" {
                writeln!(&mut self.builder, "  ret void").ok();
            } else {
                let coerce = self
                    .signatures
                    .get(self.function.name.as_str())
                    .and_then(|sig| sig.c_abi.as_ref())
                    .and_then(|c_abi| match &c_abi.ret {
                        CAbiReturn::Direct { coerce, .. } => coerce.clone(),
                        _ => None,
                    });
                if let Some(coerce_ty) = coerce {
                    let value = self.load_local(ret_idx, coerce_ty.as_str())?;
                    writeln!(&mut self.builder, "  ret {coerce_ty} {}", value.repr()).ok();
                } else {
                    let value = self.load_local(ret_idx, ret_ty.as_str())?;
                    writeln!(&mut self.builder, "  ret {ret_ty} {}", value.repr()).ok();
                }
            }
            return Ok(());
        }
        writeln!(&mut self.builder, "  ret void").ok();
        Ok(())
    }

    /// Synthesise a ready `Std.Async.Task` value for async functions without suspension points.
    fn ready_task_return(&mut self) -> Result<bool, Error> {
        let debug_ready = std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok();
        let mut ret_mir_ty = self.function.signature.ret.clone();
        if self.function.body.async_machine.is_some() {
            if let Some(result) = &self.function.async_result {
                ret_mir_ty =
                    Ty::named_generic("Std::Async::Task", vec![GenericArg::Type(result.clone())]);
            }
        }
        if debug_ready {
            eprintln!(
                "[chic-debug] ready_task_return check for {} (ret={}, suspend_points={}, async_result={:?})",
                self.function.name,
                ret_mir_ty.canonical_name(),
                self.function
                    .body
                    .async_machine
                    .as_ref()
                    .map(|m| m.suspend_points.len())
                    .unwrap_or(0),
                self.function.async_result
            );
        }
        let machine: Option<&AsyncStateMachine> = self.function.body.async_machine.as_ref();
        let Some(machine) = machine else {
            if debug_ready {
                eprintln!(
                    "[chic-debug] ready_task_return: no async machine for {}",
                    self.function.name
                );
            }
            return Ok(false);
        };

        let ret_ty = map_type_owned(&ret_mir_ty, Some(self.type_layouts))
            .map_err(|err| Error::Codegen(format!("async return type mapping failed: {err}")))?
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "async return type `{}` is missing an LLVM representation",
                    ret_mir_ty.canonical_name()
                ))
            })?;
        let result_local = machine
            .result_local
            .or_else(|| {
                self.function
                    .body
                    .locals
                    .iter()
                    .enumerate()
                    .find_map(|(idx, decl)| {
                        (decl.name.as_deref() == Some("async_result")).then_some(LocalId(idx))
                    })
            })
            .or_else(|| {
                machine.result_ty.as_ref().and_then(|ty| {
                    self.function
                        .body
                        .locals
                        .iter()
                        .enumerate()
                        .find_map(|(idx, decl)| (decl.ty == *ty).then_some(LocalId(idx)))
                })
            });
        let Some(result_local) = result_local else {
            eprintln!(
                "[chic-debug] ready_task_return missing result_local for {} (machine: {:?})",
                self.function.name, machine.result_ty
            );
            return Ok(false);
        };
        let Some(result_ty) = self.local_tys.get(result_local.0).and_then(|ty| ty.clone()) else {
            eprintln!(
                "[chic-debug] ready_task_return missing type for local {:?} in {}",
                result_local, self.function.name
            );
            return Ok(false);
        };
        if debug_ready {
            eprintln!(
                "[chic-debug] ready_task_return emitting for {} result_local={:?} result_ty={}",
                self.function.name, result_local, result_ty
            );
        }

        self.emit_trace_exit();
        let (ret_buf, allocated_new) = if let Some(ret_idx) = self.return_local
            && let Some(ptr) = self.local_ptrs.get(ret_idx).and_then(|p| p.clone())
        {
            (ptr, false)
        } else {
            let tmp = self.new_temp();
            let ret_align = self.target_align_for_ty(&ret_mir_ty)?;
            writeln!(
                &mut self.builder,
                "  {tmp} = alloca {ret_ty}, align {}",
                ret_align
            )
            .ok();
            writeln!(
                &mut self.builder,
                "  store {ret_ty} zeroinitializer, ptr {tmp}"
            )
            .ok();
            (tmp, true)
        };
        let header_flags_ptr = self
            .async_task_header_flags_ptr(&ret_buf, "Std::Async::Task")
            .ok_or_else(|| Error::Codegen("missing Std.Async.FutureHeader.Flags layout".into()))?;
        let preserved_cancel = if allocated_new {
            None
        } else {
            let prior_flags = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {prior_flags} = load i32, ptr {header_flags_ptr}, align 4"
            )
            .ok();
            let cancel_masked = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cancel_masked} = and i32 {prior_flags}, {}",
                FUTURE_FLAG_CANCELLED
            )
            .ok();
            Some(cancel_masked)
        };
        // Ensure the task header starts from a clean slate, even when reusing an existing return
        // slot provided by the caller.
        writeln!(
            &mut self.builder,
            "  store {ret_ty} zeroinitializer, ptr {ret_buf}"
        )
        .ok();

        // Mark task flags as ready/completed.
        let ready_bits: u32 = FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED;
        let ready_or_cancel = if let Some(masked_cancel) = preserved_cancel.as_ref() {
            let combined = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {combined} = or i32 {ready_bits}, {masked_cancel}"
            )
            .ok();
            combined
        } else {
            ready_bits.to_string()
        };
        let flags_ptr = self
            .async_task_field_ptr(&ret_buf, "Std::Async::Task", "Flags")
            .ok_or_else(|| Error::Codegen("missing Std.Async.Task.Flags layout".into()))?;
        writeln!(
            &mut self.builder,
            "  store i32 {ready_or_cancel}, ptr {flags_ptr}, align 4"
        )
        .ok();

        if let (Some(vtable_symbol), Some(vtable_ptr)) = (
            self.async_vtable_symbol().cloned(),
            self.async_task_header_vtable_ptr(&ret_buf, "Std::Async::Task"),
        ) {
            let vtable_align = self.target_align_for_ty(&Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Std::Async::FutureVTable"),
                true,
            ))))?;
            writeln!(
                &mut self.builder,
                "  store ptr @{vtable_symbol}, ptr {vtable_ptr}, align {vtable_align}"
            )
            .ok();
        }

        writeln!(
            &mut self.builder,
            "  store i32 {ready_or_cancel}, ptr {header_flags_ptr}, align 4"
        )
        .ok();

        // Populate the inner future if present.
        if let Some(inner_future_ptr) = self
            .async_task_field_ptr(&ret_buf, &ret_mir_ty.canonical_name(), "InnerFuture")
            .or_else(|| {
                machine
                    .result_ty
                    .as_ref()
                    .and_then(|result_ty| self.synthetic_inner_future_ptr(&ret_buf, result_ty))
            })
        {
            if let Some(inner_header_flags_ptr) =
                self.async_future_header_flags_ptr(&inner_future_ptr, &machine.result_ty)
            {
                writeln!(
                    &mut self.builder,
                    "  store i32 {ready_or_cancel}, ptr {inner_header_flags_ptr}, align 4"
                )
                .ok();
            }
            // completed field
            if let Some(completed_ptr) =
                self.async_future_field_ptr(&inner_future_ptr, &machine.result_ty, "Completed")
            {
                writeln!(
                    &mut self.builder,
                    "  store i8 1, ptr {completed_ptr}, align 1"
                )
                .ok();
            }
            if let Some(result_ptr) =
                self.async_future_field_ptr(&inner_future_ptr, &machine.result_ty, "Result")
            {
                let result_val = self.load_local(result_local.0, &result_ty)?;
                writeln!(
                    &mut self.builder,
                    "  store {ty} {val}, ptr {result_ptr}",
                    ty = result_ty,
                    val = result_val.repr()
                )
                .ok();
            }
        }

        if allocated_new {
            let ret_val = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {ret_val} = load {ret_ty}, ptr {ret_buf}"
            )
            .ok();
            if debug_ready {
                eprintln!(
                    "[chic-debug] ready_task_return emitted ready task for {} using local {:?} ({})",
                    self.function.name, result_local, result_ty
                );
            }
            writeln!(&mut self.builder, "  ret {ret_ty} {ret_val}").ok();
        } else {
            let ret_val = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {ret_val} = load {ret_ty}, ptr {ret_buf}"
            )
            .ok();
            if debug_ready {
                eprintln!(
                    "[chic-debug] ready_task_return emitted ready task for {} using existing return slot {:?} ({})",
                    self.function.name, result_local, result_ty
                );
            }
            writeln!(&mut self.builder, "  ret {ret_ty} {ret_val}").ok();
        }
        Ok(true)
    }

    /// Synthetic pointer computation for `Task<T>.InnerFuture` when layout metadata is missing.
    fn synthetic_inner_future_ptr(&mut self, base: &str, result_ty: &Ty) -> Option<String> {
        let header_ty = Ty::named("Std::Async::FutureHeader");
        let future_ty = Ty::named_generic(
            "Std::Async::Future",
            vec![GenericArg::Type(result_ty.clone())],
        );
        let (header_size, header_align) = self.type_layouts.size_and_align_for_ty(&header_ty)?;
        let (_, future_align) = self.type_layouts.size_and_align_for_ty(&future_ty)?;
        let mut offset = align_to(header_size, header_align);
        offset = align_to(offset + 4, future_align);
        // The base task layout is {Header, Flags, InnerFuture}; Flags is a u32.
        offset += 0; // offset already accounts for flags above
        self.offset_ptr(base, offset).ok()
    }

    fn target_align_for_ty(&self, ty: &Ty) -> Result<usize, Error> {
        let (size, align) = self.type_layouts.size_and_align_for_ty(ty).ok_or_else(|| {
            Error::Codegen(format!("missing layout for `{}`", ty.canonical_name()))
        })?;
        let mut aligned = align.max(1);
        if !aligned.is_power_of_two() {
            aligned = aligned.next_power_of_two();
        }
        // Avoid absurdly small alignments on zero-sized types.
        if aligned == 0 {
            aligned = size.max(1);
        }
        Ok(aligned)
    }

    fn async_task_header_flags_ptr(&mut self, base: &str, task_ty: &str) -> Option<String> {
        let header_offset = self.field_offset(task_ty, "Header")?;
        let header_flags = self.field_offset("Std::Async::FutureHeader", "Flags")?;
        let ptr = self.offset_ptr(base, header_offset + header_flags).ok()?;
        Some(ptr)
    }

    fn async_task_header_vtable_ptr(&mut self, base: &str, task_ty: &str) -> Option<String> {
        let header_offset = self.field_offset(task_ty, "Header")?;
        let vtable_offset = self.field_offset("Std::Async::FutureHeader", "VTablePointer")?;
        let ptr = self.offset_ptr(base, header_offset + vtable_offset).ok()?;
        Some(ptr)
    }

    fn async_task_field_ptr(&mut self, base: &str, task_ty: &str, field: &str) -> Option<String> {
        let offset = self.field_offset(task_ty, field)?;
        self.offset_ptr(base, offset).ok()
    }

    fn async_future_field_ptr(
        &mut self,
        base: &str,
        result_ty: &Option<Ty>,
        field: &str,
    ) -> Option<String> {
        let future_ty = match result_ty {
            Some(inner) => Ty::named(format!("Std::Async::Future<{}>", inner.canonical_name())),
            None => Ty::named("Std::Async::Future"),
        };
        let offset = self.field_offset(&future_ty.canonical_name(), field)?;
        self.offset_ptr(base, offset).ok()
    }

    fn async_future_header_flags_ptr(
        &mut self,
        base: &str,
        result_ty: &Option<Ty>,
    ) -> Option<String> {
        let header_offset = self.field_offset("Std::Async::FutureHeader", "Flags")?;
        let header_ptr = self.async_future_field_ptr(base, result_ty, "Header")?;
        self.offset_ptr(&header_ptr, header_offset).ok()
    }

    fn field_offset(&self, ty_name: &str, field: &str) -> Option<usize> {
        let layout = self.type_layouts.layout_for_name(ty_name)?;
        match layout {
            TypeLayout::Struct(data) | TypeLayout::Class(data) => data
                .fields
                .iter()
                .find(|f| f.name == field)
                .and_then(|f| f.offset),
            _ => None,
        }
    }

    pub(crate) fn emit_switch(
        &mut self,
        discr: &Operand,
        targets: &[(i128, BlockId)],
        otherwise: BlockId,
    ) -> Result<(), Error> {
        let discr_ty = self
            .operand_type(discr)?
            .ok_or_else(|| Error::Codegen("switch operand missing type information".into()))?;
        let emitted = self.emit_operand(discr, Some(&discr_ty))?;
        let otherwise_label = self.block_label(otherwise)?;

        let mut line = format!(
            "  switch {discr_ty} {}, label %{otherwise_label} [",
            emitted.repr()
        );
        for (value, block) in targets {
            let target_label = self.block_label(*block)?;
            write!(line, " {discr_ty} {value}, label %{target_label}").ok();
        }
        line.push_str(" ]");
        writeln!(&mut self.builder, "{line}").ok();
        Ok(())
    }
}

fn align_to(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + align - 1) / align * align
    }
}
