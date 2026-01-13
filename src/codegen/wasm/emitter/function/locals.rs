use crate::error::Error;
use crate::mir::{
    ConstOperand, ConstValue, FloatWidth, LocalKind, MirFunction, Operand, ParamMode, Rvalue,
    StatementKind, Ty, TypeLayoutTable,
};

use crate::codegen::wasm::{
    AggregateAllocation, ValueType, align_to, compute_aggregate_allocation, ensure_u32,
    local_requires_memory, map_type,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LocalRepresentation {
    Scalar,
    PointerParam,
    FrameAllocated,
}

pub(crate) struct LocalPlan {
    pub(crate) locals: Vec<Option<u32>>,
    pub(crate) local_types: Vec<ValueType>,
    pub(crate) local_tys: Vec<Ty>,
    pub(crate) value_types: Vec<Option<ValueType>>,
    pub(crate) representations: Vec<LocalRepresentation>,
    pub(crate) aggregate_allocations: Vec<Option<AggregateAllocation>>,
    pub(crate) return_local: Option<u32>,
    pub(crate) block_local: u32,
    pub(crate) temp_local: u32,
    pub(crate) float_temp_local: u32,
    pub(crate) double_temp_local: u32,
    pub(crate) wide_temp_local: u32,
    pub(crate) wide_temp_local_hi: u32,
    pub(crate) stack_temp_local: u32,
    pub(crate) stack_adjust_local: u32,
    pub(crate) scratch_local: u32,
    pub(crate) frame_local: Option<u32>,
    pub(crate) frame_size: u32,
}

use std::collections::HashSet;

fn value_type_from_const(constant: &ConstOperand) -> Option<ValueType> {
    match &constant.value {
        ConstValue::Float(float) => match float.width {
            FloatWidth::F16 | FloatWidth::F32 => Some(ValueType::F32),
            FloatWidth::F64 | FloatWidth::F128 => Some(ValueType::F64),
        },
        _ => None,
    }
}

fn value_type_from_rvalue(rvalue: &Rvalue) -> Option<ValueType> {
    match rvalue {
        Rvalue::Use(Operand::Const(constant)) => value_type_from_const(constant),
        Rvalue::Unary { operand, .. } => {
            if let Operand::Const(constant) = operand {
                value_type_from_const(constant)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn infer_unknown_scalar_value_types(function: &MirFunction) -> Vec<Option<ValueType>> {
    let mut hints = vec![None; function.body.locals.len()];
    for block in &function.body.blocks {
        for statement in &block.statements {
            let StatementKind::Assign { place, value } = &statement.kind else {
                continue;
            };
            if !matches!(
                function.body.locals.get(place.local.0).map(|decl| &decl.ty),
                Some(Ty::Unknown)
            ) {
                continue;
            }
            if let Some(value_ty) = value_type_from_rvalue(value) {
                hints[place.local.0] = Some(value_ty);
            }
        }
    }
    hints
}

fn scalar_allocation(ty: &Ty) -> AggregateAllocation {
    match map_type(ty) {
        ValueType::I64 | ValueType::F64 => AggregateAllocation {
            offset: 0,
            size: 8,
            align: 8,
        },
        _ => AggregateAllocation {
            offset: 0,
            size: 4,
            align: 4,
        },
    }
}

pub(crate) fn plan_locals(
    function: &MirFunction,
    layouts: &TypeLayoutTable,
    forced_frame: &HashSet<usize>,
    wasm_arg_count: usize,
    has_sret: bool,
) -> Result<LocalPlan, Error> {
    let mut locals = Vec::new();
    let mut local_types = Vec::new();
    let mut local_tys = function
        .body
        .locals
        .iter()
        .map(|decl| decl.ty.clone())
        .collect::<Vec<_>>();
    if !matches!(function.signature.ret, Ty::Unit) {
        if let Some((idx, _)) = function
            .body
            .locals
            .iter()
            .enumerate()
            .find(|(_, decl)| matches!(decl.kind, LocalKind::Return))
        {
            local_tys[idx] = function.signature.ret.clone();
        }
    }
    let mut representations = Vec::new();
    let mut aggregate_allocations = Vec::new();
    let mut value_types = Vec::new();
    let param_shift: u32 = if has_sret { 1 } else { 0 };
    let mut index = initial_local_index(wasm_arg_count)?;
    let mut return_local = None;
    let mut frame_size: u32 = 0;
    let scalar_hints = infer_unknown_scalar_value_types(function);

    for (local_index, decl) in function.body.locals.iter().enumerate() {
        let ty = &local_tys[local_index];
        let base_needs_memory = local_requires_memory(ty, layouts);
        let mut needs_memory = base_needs_memory || forced_frame.contains(&local_index);
        if function.is_async && matches!(decl.kind, LocalKind::Local | LocalKind::Temp) {
            // Async state machines may resume after the await site that produced a value,
            // so scalar locals still need addressable slots in the frame.
            needs_memory = true;
        }
        if decl
            .name
            .as_ref()
            .map(|name| name == "self" || name == "this")
            .unwrap_or(false)
            || ty
                .as_named()
                .map(|named| named.as_str() == "Self")
                .unwrap_or(false)
        {
            needs_memory = true;
        }
        if matches!(decl.kind, LocalKind::Local | LocalKind::Temp)
            && matches!(ty, Ty::Arc(_) | Ty::Rc(_))
        {
            // Arc/Rc handles are plain pointer-sized values, but the runtime
            // clone/drop hooks expect an addressable slot. Force stack
            // allocation for locals so we can pass pointers to the hooks
            // rather than uninitialised zeroes.
            needs_memory = true;
        }
        if let Ty::Fn(fn_ty) = ty {
            if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                needs_memory = true;
            }
        }
        if let Ty::Named(named) = ty {
            let canonical = ty.canonical_name();
            if canonical.starts_with("Std::Sync::Atomic") || named.as_str().contains("Atomic") {
                needs_memory = true;
            }
            let base_name = canonical
                .split('<')
                .next()
                .unwrap_or_else(|| named.as_str());
            if canonical.starts_with("Std::Async::Task")
                || canonical.starts_with("Std::Async::Future")
                || base_name.ends_with("::Task")
                || base_name.ends_with("::Future")
                || base_name == "Task"
                || base_name == "Future"
            {
                needs_memory = true;
            }
        }
        let is_class = if let Ty::Named(name) = ty {
            let canonical = ty.canonical_name();
            layouts.class_layout_info(name.as_str()).is_some()
                || layouts.class_layout_info(&canonical).is_some()
        } else {
            false
        };
        if is_class && !forced_frame.contains(&local_index) {
            needs_memory = false;
        }
        match decl.kind {
            LocalKind::Arg(param) => {
                let wasm_index = checked_param_index_shifted(param, param_shift)?;
                let address_taken = forced_frame.contains(&local_index);
                let mut representation = if matches!(
                    decl.param_mode,
                    Some(ParamMode::In | ParamMode::Ref | ParamMode::Out)
                ) {
                    LocalRepresentation::PointerParam
                } else if address_taken || (needs_memory && !base_needs_memory) {
                    // Parameter value is passed by value but needs an addressable slot
                    // (e.g. captured by an async frame or taken by reference).
                    LocalRepresentation::FrameAllocated
                } else if base_needs_memory {
                    LocalRepresentation::PointerParam
                } else {
                    LocalRepresentation::Scalar
                };
                if decl
                    .name
                    .as_ref()
                    .map(|name| name == "self" || name == "this")
                    .unwrap_or(false)
                    && ty
                        .as_named()
                        .map(|named| named.as_str() == "Self")
                        .unwrap_or(false)
                {
                    representation = LocalRepresentation::PointerParam;
                }
                match representation {
                    LocalRepresentation::Scalar | LocalRepresentation::PointerParam => {
                        locals.push(Some(wasm_index));
                        aggregate_allocations.push(None);
                        let value_ty = if representation == LocalRepresentation::PointerParam {
                            ValueType::I32
                        } else if matches!(ty, Ty::Unknown) {
                            scalar_hints
                                .get(local_index)
                                .copied()
                                .flatten()
                                .unwrap_or_else(|| map_type(ty))
                        } else {
                            map_type(ty)
                        };
                        value_types.push(Some(value_ty));
                    }
                    LocalRepresentation::FrameAllocated => {
                        record_regular_local(
                            &mut locals,
                            &mut local_types,
                            &mut index,
                            ValueType::I32,
                        )?;
                        let allocation = compute_aggregate_allocation(ty, layouts)
                            .unwrap_or_else(|| scalar_allocation(ty));
                        let offset = align_to(frame_size, allocation.align);
                        frame_size = offset + allocation.size;
                        aggregate_allocations.push(Some(AggregateAllocation {
                            offset,
                            size: allocation.size,
                            align: allocation.align.max(1),
                        }));
                        value_types.push(Some(ValueType::I32));
                    }
                }
                representations.push(representation);
            }
            LocalKind::Return => {
                if matches!(function.signature.ret, Ty::Unit) {
                    locals.push(None);
                    representations.push(LocalRepresentation::Scalar);
                    aggregate_allocations.push(None);
                    value_types.push(None);
                } else {
                    let ret_ty = &function.signature.ret;
                    let mut ret_needs_memory = local_requires_memory(ret_ty, layouts);
                    if function.is_async {
                        ret_needs_memory = true;
                    }
                    if ret_needs_memory {
                        if has_sret {
                            // Return slot is provided by the caller (sret).
                            locals.push(Some(0));
                            representations.push(LocalRepresentation::PointerParam);
                            aggregate_allocations.push(None);
                            value_types.push(Some(ValueType::I32));
                            return_local = Some(0);
                        } else {
                            record_regular_local(
                                &mut locals,
                                &mut local_types,
                                &mut index,
                                ValueType::I32,
                            )?;
                            representations.push(LocalRepresentation::FrameAllocated);
                            let allocation = compute_aggregate_allocation(ret_ty, layouts)
                                .unwrap_or_else(|| scalar_allocation(ret_ty));
                            let offset = align_to(frame_size, allocation.align);
                            frame_size = offset + allocation.size;
                            aggregate_allocations.push(Some(AggregateAllocation {
                                offset,
                                size: allocation.size,
                                align: allocation.align.max(1),
                            }));
                            value_types.push(Some(ValueType::I32));
                            return_local = locals.last().copied().flatten();
                        }
                    } else {
                        let wasm_ty = map_type(ret_ty);
                        record_regular_local(&mut locals, &mut local_types, &mut index, wasm_ty)?;
                        return_local = locals.last().copied().flatten();
                        representations.push(LocalRepresentation::Scalar);
                        aggregate_allocations.push(None);
                        value_types.push(Some(wasm_ty));
                    }
                }
            }
            _ => {
                if needs_memory {
                    record_regular_local(
                        &mut locals,
                        &mut local_types,
                        &mut index,
                        ValueType::I32,
                    )?;
                    representations.push(LocalRepresentation::FrameAllocated);
                    let allocation = compute_aggregate_allocation(ty, layouts)
                        .unwrap_or_else(|| scalar_allocation(ty));
                    let offset = align_to(frame_size, allocation.align);
                    frame_size = offset + allocation.size;
                    aggregate_allocations.push(Some(AggregateAllocation {
                        offset,
                        size: allocation.size,
                        align: allocation.align.max(1),
                    }));
                    value_types.push(Some(ValueType::I32));
                } else {
                    let wasm_ty = if matches!(ty, Ty::Unknown) {
                        scalar_hints
                            .get(local_index)
                            .copied()
                            .flatten()
                            .unwrap_or_else(|| map_type(ty))
                    } else {
                        map_type(ty)
                    };
                    record_regular_local(&mut locals, &mut local_types, &mut index, wasm_ty)?;
                    representations.push(LocalRepresentation::Scalar);
                    aggregate_allocations.push(None);
                    value_types.push(Some(wasm_ty));
                }
            }
        }
    }

    let block_local = allocate_extra_local(&mut local_types, &mut index, ValueType::I32)?;
    let temp_local = allocate_extra_local(&mut local_types, &mut index, ValueType::I32)?;
    let float_temp_local = allocate_extra_local(&mut local_types, &mut index, ValueType::F32)?;
    let double_temp_local = allocate_extra_local(&mut local_types, &mut index, ValueType::F64)?;
    let wide_temp_local = allocate_extra_local(&mut local_types, &mut index, ValueType::I64)?;
    let wide_temp_local_hi = allocate_extra_local(&mut local_types, &mut index, ValueType::I64)?;
    let stack_temp_local = allocate_extra_local(&mut local_types, &mut index, ValueType::I32)?;
    let stack_adjust_local = allocate_extra_local(&mut local_types, &mut index, ValueType::I32)?;
    let scratch_local = allocate_extra_local(&mut local_types, &mut index, ValueType::I32)?;
    let frame_local = Some(allocate_extra_local(
        &mut local_types,
        &mut index,
        ValueType::I32,
    )?);

    if std::env::var_os("CHIC_DEBUG_WASM_LOCALS").is_some() {
        eprintln!(
            "[wasm-local-plan] func={} locals={}",
            function.name,
            locals.len()
        );
        for (idx, decl) in function.body.locals.iter().enumerate() {
            let repr = representations
                .get(idx)
                .copied()
                .unwrap_or(LocalRepresentation::Scalar);
            let ty = local_tys
                .get(idx)
                .map(|ty| ty.canonical_name())
                .unwrap_or_else(|| "<unknown>".into());
            let slot = locals.get(idx).copied().flatten();
            let alloc = aggregate_allocations.get(idx).copied().flatten();
            let name = decl.name.clone().unwrap_or_else(|| "_".into());
            eprintln!(
                "  local {idx} name={name} kind={:?} ty={ty} repr={repr:?} slot={slot:?} alloc={alloc:?}",
                decl.kind
            );
        }
    }

    Ok(LocalPlan {
        locals,
        local_types,
        local_tys,
        value_types,
        representations,
        aggregate_allocations,
        return_local,
        block_local,
        temp_local,
        float_temp_local,
        double_temp_local,
        wide_temp_local,
        wide_temp_local_hi,
        frame_local,
        stack_temp_local,
        stack_adjust_local,
        scratch_local,
        frame_size,
    })
}

fn initial_local_index(arg_count: usize) -> Result<u32, Error> {
    ensure_u32(
        arg_count,
        "function argument count exceeds WebAssembly limits",
    )
}

fn checked_param_index(param: usize) -> Result<u32, Error> {
    ensure_u32(param, "argument index exceeds WebAssembly limits")
}

fn checked_param_index_shifted(param: usize, shift: u32) -> Result<u32, Error> {
    let shifted = param
        .checked_add(shift as usize)
        .ok_or_else(|| Error::Codegen("argument index exceeds WebAssembly limits".into()))?;
    checked_param_index(shifted)
}

fn record_regular_local(
    locals: &mut Vec<Option<u32>>,
    local_types: &mut Vec<ValueType>,
    index: &mut u32,
    ty: ValueType,
) -> Result<(), Error> {
    locals.push(Some(*index));
    local_types.push(ty);
    *index = bump_index(*index)?;
    Ok(())
}

fn bump_index(index: u32) -> Result<u32, Error> {
    index
        .checked_add(1)
        .ok_or_else(|| Error::Codegen("local index exceeds WebAssembly limits".into()))
}

fn allocate_extra_local(
    local_types: &mut Vec<ValueType>,
    index: &mut u32,
    ty: ValueType,
) -> Result<u32, Error> {
    let slot = *index;
    local_types.push(ty);
    *index = bump_index(slot)?;
    Ok(slot)
}
