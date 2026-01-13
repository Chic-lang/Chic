use std::collections::{HashMap, HashSet};

use crate::frontend::parser::parse_type_expression_text;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, ParamMode, Place, PointerTy,
    ProjectionElem, Rvalue, Statement, StatementKind, Terminator, Ty, TypeLayout,
};
use crate::monomorphize::MonomorphizationSummary;
use crate::type_identity::type_identity_for_name;

/// Metadata describing a synthesised equality glue thunk.
#[derive(Debug, Clone)]
pub struct SynthesisedEqGlue {
    pub type_name: String,
    pub symbol: String,
    pub function_index: usize,
    pub type_identity: u64,
}

pub fn eq_glue_symbol_for(ty_name: &str) -> String {
    let mut symbol = String::from("__cl_eq__");
    for ch in ty_name.chars() {
        match ch {
            ':' | '<' | '>' | ',' | ' ' | '[' | ']' | '?' | '.' => symbol.push('_'),
            ch if ch.is_alphanumeric() || ch == '_' => symbol.push(ch),
            _ => symbol.push('_'),
        }
    }
    symbol
}

fn pointer_ty_for(ty: Ty) -> Ty {
    Ty::Pointer(Box::new(PointerTy::new(ty, true)))
}

fn parse_type_from_name(name: &str) -> Option<Ty> {
    if let Some(expr) = parse_type_expression_text(name) {
        return Some(Ty::from_type_expr(&expr));
    }
    if name.contains("::") {
        let substituted = name.replace("::", ".");
        if let Some(mut expr) = parse_type_expression_text(&substituted) {
            expr.name = name.to_string();
            return Some(Ty::from_type_expr(&expr));
        }
    }
    Some(Ty::named(name))
}

fn eq_method_symbol(type_name: &str) -> String {
    format!("{type_name}::op_Equality")
}

fn function_param_modes(function: &MirFunction) -> Vec<ParamMode> {
    let mut modes = vec![ParamMode::Value; function.body.arg_count];
    for local in &function.body.locals {
        if let LocalKind::Arg(index) = local.kind {
            if let Some(mode) = local.param_mode {
                if index < modes.len() {
                    modes[index] = mode;
                }
            }
        }
    }
    modes
}

pub fn synthesise_eq_glue(
    module: &mut MirModule,
    summary: &MonomorphizationSummary,
) -> Vec<SynthesisedEqGlue> {
    if summary.eq_candidates.is_empty() {
        return Vec::new();
    }

    let mut existing: HashSet<String> = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect();
    let mut param_modes = HashMap::new();
    for function in &module.functions {
        param_modes.insert(function.name.clone(), function_param_modes(function));
    }

    let mut synthesised = Vec::new();
    for ty_name in &summary.eq_candidates {
        let symbol = eq_glue_symbol_for(ty_name);
        if existing.contains(&symbol) {
            continue;
        }
        let method_symbol = eq_method_symbol(ty_name);
        let function = if let Some(modes) = param_modes.get(&method_symbol) {
            synthesize_eq_function(ty_name, &method_symbol, modes)
        } else if matches!(
            module.type_layouts.types.get(ty_name),
            Some(TypeLayout::Enum(_))
        ) {
            synthesize_enum_eq_function(ty_name)
        } else {
            None
        };
        let Some(function) = function else {
            continue;
        };
        let index = module.functions.len();
        existing.insert(symbol.clone());
        module.functions.push(function);
        synthesised.push(SynthesisedEqGlue {
            type_name: ty_name.clone(),
            symbol,
            function_index: index,
            type_identity: type_identity_for_name(&module.type_layouts, ty_name),
        });
    }

    synthesised
}

fn synthesize_eq_function(
    ty_name: &str,
    method_symbol: &str,
    modes: &[ParamMode],
) -> Option<MirFunction> {
    let name = eq_glue_symbol_for(ty_name);
    let ty = parse_type_from_name(ty_name)?;
    let typed_pointer_ty = pointer_ty_for(ty.clone());
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);

    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("left".into()),
        typed_pointer_ty.clone(),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("right".into()),
        typed_pointer_ty,
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        Some("eq".into()),
        Ty::named("bool"),
        false,
        None,
        LocalKind::Temp,
    ));

    let left_mode = modes.get(0).copied().unwrap_or(ParamMode::Value);
    let right_mode = modes.get(1).copied().unwrap_or(ParamMode::Value);

    let left_operand = Operand::Copy(Place {
        local: LocalId(1),
        projection: vec![ProjectionElem::Deref],
    });
    let right_operand = Operand::Copy(Place {
        local: LocalId(2),
        projection: vec![ProjectionElem::Deref],
    });

    let mut args = Vec::new();
    let mut arg_modes = Vec::new();
    if modes.len() > 2 {
        // Some operator overloads are lowered as instance-style methods and expect a
        // receiver pointer before the value operands. Reuse the left pointer to satisfy
        // that slot when present.
        args.push(Operand::Copy(Place::new(LocalId(1))));
        arg_modes.push(modes[0]);
        args.push(left_operand);
        arg_modes.push(modes.get(1).copied().unwrap_or(left_mode));
        args.push(right_operand);
        arg_modes.push(modes.get(2).copied().unwrap_or(right_mode));
    } else {
        args.push(left_operand);
        args.push(right_operand);
        arg_modes.push(left_mode);
        arg_modes.push(right_mode);
    }

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            method_symbol.to_string(),
        ))),
        args,
        arg_modes,
        destination: Some(Place::new(LocalId(3))),
        target: BlockId(1),
        unwind: None,
        dispatch: None,
    });
    body.blocks.push(entry);

    let mut switch_block = BasicBlock::new(BlockId(1), None);
    switch_block.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(Place::new(LocalId(3))),
        targets: vec![(0, BlockId(3)), (1, BlockId(2))],
        otherwise: BlockId(3),
    });
    body.blocks.push(switch_block);

    let mut true_block = BasicBlock::new(BlockId(2), None);
    true_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int32(1)))),
        },
    });
    true_block.terminator = Some(Terminator::Return);
    body.blocks.push(true_block);

    let mut false_block = BasicBlock::new(BlockId(3), None);
    false_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int32(0)))),
        },
    });
    false_block.terminator = Some(Terminator::Return);
    body.blocks.push(false_block);

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty.clone(), raw_pointer_ty],
            ret: Ty::named("int"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    })
}

fn synthesize_enum_eq_function(ty_name: &str) -> Option<MirFunction> {
    let name = eq_glue_symbol_for(ty_name);
    let ty = parse_type_from_name(ty_name)?;
    let typed_pointer_ty = pointer_ty_for(ty.clone());
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);

    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("left".into()),
        typed_pointer_ty.clone(),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("right".into()),
        typed_pointer_ty,
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        Some("eq".into()),
        Ty::named("bool"),
        false,
        None,
        LocalKind::Temp,
    ));

    let left_value = Operand::Copy(Place {
        local: LocalId(1),
        projection: vec![ProjectionElem::Deref],
    });
    let right_value = Operand::Copy(Place {
        local: LocalId(2),
        projection: vec![ProjectionElem::Deref],
    });

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Binary {
                op: crate::mir::BinOp::Eq,
                lhs: left_value,
                rhs: right_value,
                rounding: None,
            },
        },
    });
    entry.terminator = Some(Terminator::Goto { target: BlockId(1) });
    body.blocks.push(entry);

    let mut switch_block = BasicBlock::new(BlockId(1), None);
    switch_block.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(Place::new(LocalId(3))),
        targets: vec![(0, BlockId(3)), (1, BlockId(2))],
        otherwise: BlockId(3),
    });
    body.blocks.push(switch_block);

    let mut true_block = BasicBlock::new(BlockId(2), None);
    true_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int32(1)))),
        },
    });
    true_block.terminator = Some(Terminator::Return);
    body.blocks.push(true_block);

    let mut false_block = BasicBlock::new(BlockId(3), None);
    false_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int32(0)))),
        },
    });
    false_block.terminator = Some(Terminator::Return);
    body.blocks.push(false_block);

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty.clone(), raw_pointer_ty],
            ret: Ty::named("int"),
            abi: Abi::Extern("C".into()),
            effects: Vec::new(),
            lends_to_return: None,
            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    })
}
