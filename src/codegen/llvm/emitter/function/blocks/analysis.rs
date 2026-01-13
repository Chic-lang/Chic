use crate::abi::{CAbiPass, CAbiReturn};
use crate::codegen::llvm::signatures::{canonical_function_name, resolve_function_name};
use crate::codegen::llvm::types::{infer_const_type, map_type_owned};
use crate::error::Error;
use crate::mir::{
    BasicBlock, ConstValue, FunctionKind, GenericArg, LocalKind, Operand, ParamMode,
    PendingOperand, ProjectionElem, StatementKind, Terminator, Ty,
};
use std::collections::HashSet;

use super::super::builder::FunctionEmitter;

const STARTUP_DESCRIPTOR_LLVM_TYPE: &str = "{ i32, [4 x i8], { i64, i32, i32 }, { i64, i64 } }";
const STARTUP_TESTCASE_DESCRIPTOR_LLVM_TYPE: &str = "{ i64, i64, i64, i32, i32 }";

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn call_operand_repr(&self, operand: &Operand) -> Result<String, Error> {
        match operand {
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => Ok(name.clone()),
                _ => Err(Error::Codegen(
                    "only symbol constants can be call targets in LLVM backend".into(),
                )),
            },
            Operand::Pending(PendingOperand { repr, .. }) => Ok(repr.clone()),
            _ => Err(Error::Codegen(format!(
                "only constant or pending operands can be call targets (in {} got {operand:?})",
                self.function.name
            ))),
        }
    }

    pub(crate) fn compute_local_types(&self) -> Result<Vec<Option<String>>, Error> {
        let body = &self.function.body;
        let mut types = vec![None; body.locals.len()];

        for (index, local) in body.locals.iter().enumerate() {
            let inferred = match local.kind {
                LocalKind::Return => {
                    let mut ret_ty = self.function.signature.ret.clone();
                    if self.function.body.async_machine.is_some() {
                        if let Some(result) = &self.function.async_result {
                            ret_ty = Ty::named_generic(
                                "Std::Async::Task",
                                vec![GenericArg::Type(result.clone())],
                            );
                        }
                    }
                    if matches!(ret_ty, Ty::Unit)
                        && matches!(self.function.kind, FunctionKind::Testcase)
                    {
                        ret_ty = Ty::named("int");
                    }
                    map_type_owned(&ret_ty, Some(self.type_layouts))?
                }
                LocalKind::Arg(arg_idx) => {
                    let mode = local.param_mode.unwrap_or(ParamMode::Value);
                    if arg_idx >= self.function.signature.params.len() {
                        eprintln!(
                            "codegen: missing param index {} for function {} ({} params); defaulting to {}",
                            arg_idx,
                            self.function.name,
                            self.function.signature.params.len(),
                            match mode {
                                ParamMode::Value => "i32",
                                _ => "ptr",
                            }
                        );
                        match mode {
                            ParamMode::Value => Some("i32".to_string()),
                            ParamMode::In | ParamMode::Ref | ParamMode::Out => {
                                Some("ptr".to_string())
                            }
                        }
                    } else {
                        let ty =
                            self.function.signature.params.get(arg_idx).ok_or_else(|| {
                                Error::Codegen("argument index out of range".into())
                            })?;
                        if let Some(sig) = self.signatures.get(self.function.name.as_str())
                            && let Some(c_abi) = sig.c_abi.as_ref()
                            && let Some(param) = c_abi.params.get(arg_idx)
                            && matches!(param.pass, CAbiPass::Direct)
                            && param.coerce.is_some()
                            && mode == ParamMode::Value
                        {
                            // Materialise coerced C-ABI aggregates in their native Chic layout for local access.
                            map_type_owned(&param.ty, Some(self.type_layouts))?
                        } else {
                            match mode {
                                ParamMode::Value => map_type_owned(ty, Some(self.type_layouts))?,
                                ParamMode::In | ParamMode::Ref | ParamMode::Out => {
                                    Some("ptr".to_string())
                                }
                            }
                        }
                    }
                }
                LocalKind::Local | LocalKind::Temp => match &local.ty {
                    crate::mir::Ty::Unknown => None,
                    ty => map_type_owned(ty, Some(self.type_layouts))?,
                },
            };
            types[index] = inferred;
        }

        let mut changed = true;
        while changed {
            changed = false;
            for block in &body.blocks {
                changed |= self.propagate_statement_types(block, &mut types)?;
                changed |= self.propagate_call_types(block, &mut types);
            }
        }

        let decimal_result_ty_string = map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalIntrinsicResult"),
            Some(self.type_layouts),
        )
        .ok()
        .flatten()
        .unwrap_or_else(|| "{ i32, [12 x i8], i128, i32, [12 x i8] }".to_string());
        let decimal_runtime_ty_string = map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalRuntimeCall"),
            Some(self.type_layouts),
        )
        .ok()
        .flatten()
        .unwrap_or_else(|| "{ i32, { i32, i32, i32, i32 } }".to_string());
        let mut decimal_locals: HashSet<usize> = HashSet::new();
        let mut decimal_runtime_locals: HashSet<usize> = HashSet::new();
        for block in &body.blocks {
            if let Some(Terminator::Call {
                func,
                destination: Some(place),
                ..
            }) = &block.terminator
            {
                if self.is_decimal_wrapper_target(func) {
                    decimal_locals.insert(place.local.0);
                } else if self.is_decimal_runtime_target(func) {
                    if let Some(ret_ty) = self.call_destination_type(func) {
                        if ret_ty == decimal_runtime_ty_string {
                            decimal_runtime_locals.insert(place.local.0);
                        }
                    }
                }
            }
        }
        Self::propagate_decimal_structs(&body.blocks, &mut decimal_locals, true);
        Self::propagate_decimal_structs(&body.blocks, &mut decimal_runtime_locals, false);
        for local in &decimal_locals {
            if matches!(
                body.locals.get(*local).map(|l| l.kind),
                Some(LocalKind::Return)
            ) {
                continue;
            }
            if types[*local].as_deref() != Some(decimal_result_ty_string.as_str()) {
                types[*local] = Some(decimal_result_ty_string.clone());
            }
        }
        for local in &decimal_runtime_locals {
            if matches!(
                body.locals.get(*local).map(|l| l.kind),
                Some(LocalKind::Return)
            ) {
                continue;
            }
            if types[*local].as_deref() != Some(decimal_runtime_ty_string.as_str()) {
                types[*local] = Some(decimal_runtime_ty_string.clone());
            }
        }
        if !decimal_locals.is_empty() || !decimal_runtime_locals.is_empty() {
            let mut changed = true;
            while changed {
                changed = false;
                for block in &body.blocks {
                    changed |= self.propagate_statement_types(block, &mut types)?;
                }
            }
        }

        if let Some(signature) = self.signatures.get(&self.function.name) {
            let arg_offset = match signature.c_abi.as_ref().map(|c_abi| &c_abi.ret) {
                Some(CAbiReturn::IndirectSret { .. }) => 1usize,
                _ => 0usize,
            };
            for (index, local) in body.locals.iter().enumerate() {
                if let LocalKind::Arg(arg_idx) = local.kind {
                    if signature
                        .c_abi
                        .as_ref()
                        .and_then(|c_abi| c_abi.params.get(arg_idx))
                        .is_some_and(|param| {
                            matches!(
                                param.pass,
                                CAbiPass::IndirectByVal { .. } | CAbiPass::IndirectPtr { .. }
                            )
                        })
                    {
                        continue;
                    }
                    if let Some(param_ty) = signature.params.get(arg_idx + arg_offset) {
                        types[index] = Some(param_ty.clone());
                    }
                }
            }
        }

        for (index, ty) in types.iter_mut().enumerate() {
            if ty.is_none() && !matches!(body.locals[index].kind, LocalKind::Return) {
                *ty = Some("i32".into());
            }
        }

        Ok(types)
    }

    fn is_decimal_wrapper_target(&self, func: &Operand) -> bool {
        if let Ok(repr) = self.call_operand_repr(func) {
            let canonical = repr.replace('.', "::").to_ascii_lowercase();
            return canonical.contains("std::decimal::intrinsics::");
        }
        false
    }

    fn is_decimal_runtime_target(&self, func: &Operand) -> bool {
        if let Ok(repr) = self.call_operand_repr(func) {
            let canonical = repr.replace('.', "::").to_ascii_lowercase();
            return canonical.contains("runtimeintrinsics::chic_rt_decimal_");
        }
        false
    }

    pub(crate) fn propagate_statement_types(
        &self,
        block: &BasicBlock,
        types: &mut [Option<String>],
    ) -> Result<bool, Error> {
        let mut changed = false;
        for statement in &block.statements {
            if let StatementKind::Assign { place, value } = &statement.kind {
                let idx = place.local.0;
                if types[idx].is_none() {
                    if let Some(ty) = self.infer_rvalue_type(value, types)? {
                        types[idx] = Some(ty);
                        changed = true;
                    }
                }
            }
        }
        Ok(changed)
    }

    pub(crate) fn propagate_call_types(
        &self,
        block: &BasicBlock,
        types: &mut [Option<String>],
    ) -> bool {
        if let Some(Terminator::Call {
            func,
            destination: Some(place),
            ..
        }) = &block.terminator
        {
            let idx = place.local.0;
            if types[idx].is_none()
                && let Some(ret_ty) = self.call_destination_type(func)
            {
                types[idx] = Some(ret_ty);
                return true;
            }
            if types[idx].is_none()
                && self
                    .function
                    .name
                    .contains("Std::Numeric::NumericBitOperations::Rotate")
            {
                if let Operand::Copy(copy) | Operand::Move(copy) = func {
                    if copy.projection.is_empty() {
                        if let Some(source_ty) = types.get(copy.local.0).and_then(|ty| ty.clone()) {
                            types[idx] = Some(source_ty);
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub(crate) fn call_destination_type(&self, func: &Operand) -> Option<String> {
        if let Some(fn_ty) = self.call_operand_fn_ty(func) {
            return map_type_owned(&fn_ty.ret, Some(self.type_layouts))
                .ok()
                .flatten();
        }
        let repr = match func {
            Operand::Pending(pending) => Some(pending.repr.as_str()),
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => Some(name.as_str()),
                _ => None,
            },
            _ => None,
        }?;
        let canonical_name = canonical_function_name(repr);
        if let Some(ret) = self.runtime_call_return_type(&canonical_name) {
            return Some(ret);
        }
        let canonical = resolve_function_name(self.signatures, &canonical_name)?;
        self.signatures.get(&canonical)?.ret.clone()
    }

    fn runtime_call_return_type(&self, canonical: &str) -> Option<String> {
        let pointer_width = match self.arch {
            crate::target::TargetArch::X86_64 | crate::target::TargetArch::Aarch64 => "i64",
        };
        let matches = |name: &str| -> bool {
            name == canonical
                || canonical.ends_with(&format!("::{name}"))
                || canonical.ends_with(&format!("::RuntimeIntrinsics::{name}"))
                || canonical.ends_with(&format!("::StartupRuntimeState::{name}"))
        };
        if matches("chic_rt_startup_raw_argv")
            || matches("chic_rt_startup_raw_envp")
            || matches("chic_rt_startup_ptr_at")
            || matches("chic_rt_startup_call_entry_async")
            || matches("chic_rt_startup_call_testcase_async")
        {
            return Some(pointer_width.to_string());
        }
        if matches("chic_rt_object_new") {
            return Some("ptr".to_string());
        }
        if matches("chic_rt_startup_descriptor_snapshot") {
            return Some(STARTUP_DESCRIPTOR_LLVM_TYPE.to_string());
        }
        if matches("chic_rt_startup_test_descriptor") {
            return Some(STARTUP_TESTCASE_DESCRIPTOR_LLVM_TYPE.to_string());
        }
        if matches("chic_rt_startup_cstr_to_string")
            || matches("chic_rt_startup_slice_to_string")
            || matches("chic_rt_startup_i32_to_string")
            || matches("chic_rt_startup_usize_to_string")
        {
            return Some(crate::codegen::llvm::emitter::literals::LLVM_STRING_TYPE.to_string());
        }
        let canonical_lower = canonical.to_ascii_lowercase();
        if canonical_lower.contains("std::decimal::intrinsics::") {
            return map_type_owned(
                &Ty::named("Std::Numeric::Decimal::DecimalIntrinsicResult"),
                Some(self.type_layouts),
            )
            .ok()
            .flatten();
        }
        if canonical_lower.contains("runtimeintrinsics::chic_rt_decimal_") {
            let status_ty = map_type_owned(
                &Ty::named("Std::Numeric::Decimal::DecimalStatus"),
                Some(self.type_layouts),
            )
            .ok()
            .flatten()
            .unwrap_or_else(|| "i32".to_string());
            let is_sum_or_dot = canonical_lower.ends_with("_sum")
                || canonical_lower.ends_with("_sum_simd")
                || canonical_lower.ends_with("_dot")
                || canonical_lower.ends_with("_dot_simd");
            if is_sum_or_dot {
                return map_type_owned(
                    &Ty::named("Std::Numeric::Decimal::DecimalRuntimeCall"),
                    Some(self.type_layouts),
                )
                .ok()
                .flatten()
                .or(Some("{ i32, { i32, i32, i32, i32 } }".to_string()));
            }
            return Some(status_ty);
        }
        None
    }

    pub(crate) fn operand_type(&self, operand: &Operand) -> Result<Option<String>, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.place_type(place),
            Operand::Const(constant) => {
                infer_const_type(&constant.value, constant.literal.as_ref())
            }
            Operand::Mmio(spec) => {
                let ty = if spec.width_bits <= 32 { "i32" } else { "i64" };
                Ok(Some(ty.to_string()))
            }
            Operand::Borrow(_) => Ok(Some("ptr".into())),
            Operand::Pending(_) => Ok(None),
        }
    }

    fn is_decimal_field_projection(place: &crate::mir::Place) -> bool {
        place.projection.iter().any(|elem| {
            matches!(
                elem,
                ProjectionElem::FieldNamed(name)
                    if matches!(name.as_str(), "Status" | "Value" | "Variant")
            )
        })
    }

    fn copy_source_local(value: &crate::mir::Rvalue) -> Option<usize> {
        match value {
            crate::mir::Rvalue::Use(Operand::Copy(place) | Operand::Move(place))
                if place.projection.is_empty() =>
            {
                Some(place.local.0)
            }
            _ => None,
        }
    }

    fn propagate_decimal_structs(
        blocks: &[BasicBlock],
        tracked: &mut HashSet<usize>,
        include_field_projections: bool,
    ) {
        let mut propagate_changed = true;
        while propagate_changed {
            propagate_changed = false;
            for block in blocks {
                for statement in &block.statements {
                    if let StatementKind::Assign { place, value } = &statement.kind {
                        if include_field_projections
                            && Self::is_decimal_field_projection(place)
                            && tracked.insert(place.local.0)
                        {
                            propagate_changed = true;
                            continue;
                        }
                        if let Some(src_local) = Self::copy_source_local(value) {
                            if tracked.contains(&src_local) && tracked.insert(place.local.0) {
                                propagate_changed = true;
                            }
                        }
                    }
                }
            }
        }
    }
}
