use crate::codegen::wasm::{ValueType, local_requires_memory, map_type};
use crate::error::Error;
use crate::mir::{FnTy, LocalKind, MirFunction, ParamMode, Terminator, Ty, TypeLayoutTable};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FunctionSignature {
    pub(crate) params: Vec<crate::codegen::wasm::ValueType>,
    pub(crate) results: Vec<crate::codegen::wasm::ValueType>,
}

impl FunctionSignature {
    pub(crate) fn from_mir(function: &MirFunction, layouts: &TypeLayoutTable) -> Self {
        let needs_sret = !matches!(function.signature.ret, Ty::Unit)
            && local_requires_memory(&function.signature.ret, layouts);
        let mut params = Vec::new();
        if needs_sret {
            params.push(map_type(&Ty::Pointer(Box::new(
                crate::mir::PointerTy::new(Ty::Unit, true),
            ))));
        }
        let mut arg_len = function.body.arg_count;
        for decl in &function.body.locals {
            if let LocalKind::Arg(index) = decl.kind {
                arg_len = arg_len.max(index + 1);
            }
        }
        if arg_len > 0 {
            let mut arg_types = vec![None; arg_len];
            for decl in &function.body.locals {
                if let LocalKind::Arg(index) = decl.kind {
                    if let Some(slot) = arg_types.get_mut(index) {
                        let value_ty = if matches!(
                            decl.param_mode,
                            Some(ParamMode::In | ParamMode::Ref | ParamMode::Out)
                        ) {
                            ValueType::I32
                        } else {
                            map_type(&decl.ty)
                        };
                        *slot = Some(value_ty);
                    }
                }
            }
            for (index, entry) in arg_types.into_iter().enumerate() {
                if let Some(value_ty) = entry {
                    params.push(value_ty);
                } else if let Some(param) = function.signature.params.get(index) {
                    params.push(map_type(param));
                } else {
                    params.push(map_type(&Ty::Unknown));
                }
            }
        } else {
            for param in &function.signature.params {
                params.push(map_type(param));
            }
        }
        let mut results = Vec::new();
        if !matches!(function.signature.ret, Ty::Unit) {
            if needs_sret {
                results.push(map_type(&Ty::Pointer(Box::new(
                    crate::mir::PointerTy::new(Ty::Unit, true),
                ))));
            } else {
                results.push(map_type(&function.signature.ret));
            }
        }
        Self { params, results }
    }

    pub(crate) fn from_fn_ty(fn_ty: &FnTy, layouts: &TypeLayoutTable) -> Self {
        let needs_sret =
            !matches!(*fn_ty.ret, Ty::Unit) && local_requires_memory(fn_ty.ret.as_ref(), layouts);
        let mut params = Vec::with_capacity(fn_ty.params.len() + 2);
        if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            params.push(map_type(&Ty::Pointer(Box::new(
                crate::mir::PointerTy::new(Ty::Unit, true),
            ))));
        }
        if needs_sret {
            params.push(map_type(&Ty::Pointer(Box::new(
                crate::mir::PointerTy::new(Ty::Unit, true),
            ))));
        }
        params.extend(fn_ty.params.iter().map(map_type));
        let mut results = Vec::new();
        if !matches!(*fn_ty.ret, Ty::Unit) {
            if needs_sret {
                results.push(map_type(&Ty::Pointer(Box::new(
                    crate::mir::PointerTy::new(Ty::Unit, true),
                ))));
            } else {
                results.push(map_type(fn_ty.ret.as_ref()));
            }
        }
        Self { params, results }
    }
}

pub(crate) fn ensure_supported_function(function: &MirFunction) -> Result<(), Error> {
    wasm_debug!(
        "ensure_supported_function `{}`: scanning {} blocks",
        function.name,
        function.body.blocks.len()
    );

    for block in &function.body.blocks {
        if let Some(Terminator::Pending(_)) = block.terminator {
            return Err(Error::codegen(format!(
                "MIR for `{}` still contains pending lowering operations; rerun compiler with `--full-lowering`",
                function.name
            )));
        }
    }

    Ok(())
}
