use std::collections::{HashMap, HashSet};

use crate::frontend::parser::parse_type_expression_text;
use crate::mir::{
    Abi, BasicBlock, BlockId, CastKind, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl,
    LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, ParamMode, Place, PointerTy,
    ProjectionElem, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use crate::monomorphize::MonomorphizationSummary;
use crate::primitives::PrimitiveKind;
use crate::type_identity::type_identity_for_name;

/// Metadata describing a synthesised hash glue thunk.
#[derive(Debug, Clone)]
pub struct SynthesisedHashGlue {
    pub type_name: String,
    pub symbol: String,
    pub function_index: usize,
    pub type_identity: u64,
}

pub fn hash_glue_symbol_for(ty_name: &str) -> String {
    let mut symbol = String::from("__cl_hash__");
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

fn hash_method_symbol(type_name: &str) -> String {
    format!("{type_name}::GetHashCode")
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

pub fn synthesise_hash_glue(
    module: &mut MirModule,
    summary: &MonomorphizationSummary,
) -> Vec<SynthesisedHashGlue> {
    if summary.hash_candidates.is_empty() {
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
    for ty_name in &summary.hash_candidates {
        let symbol = hash_glue_symbol_for(ty_name);
        if existing.contains(&symbol) {
            continue;
        }
        let method_symbol = hash_method_symbol(ty_name);
        let function = if let Some(modes) = param_modes.get(&method_symbol) {
            let arg_mode = modes.get(0).copied().unwrap_or(ParamMode::Value);
            synthesize_hash_function(ty_name, &method_symbol, arg_mode)
        } else {
            let desc = module
                .type_layouts
                .primitive_registry
                .descriptor_for_name(ty_name);
            desc.and_then(|desc| {
                synthesize_primitive_hash_function(ty_name, &desc.primitive_name, &desc.kind)
            })
        };
        let Some(function) = function else {
            continue;
        };
        let index = module.functions.len();
        existing.insert(symbol.clone());
        module.functions.push(function);
        synthesised.push(SynthesisedHashGlue {
            type_name: ty_name.clone(),
            symbol,
            function_index: index,
            type_identity: type_identity_for_name(&module.type_layouts, ty_name),
        });
    }

    synthesised
}

fn synthesize_primitive_hash_function(
    ty_name: &str,
    primitive_name: &str,
    kind: &PrimitiveKind,
) -> Option<MirFunction> {
    let name = hash_glue_symbol_for(ty_name);
    let primitive_ty = Ty::named(primitive_name.to_string());
    let typed_pointer_ty = pointer_ty_for(primitive_ty.clone());
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("ulong"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        typed_pointer_ty,
        false,
        None,
        LocalKind::Arg(0),
    ));

    let value_operand = Operand::Copy(Place {
        local: LocalId(1),
        projection: vec![ProjectionElem::Deref],
    });

    match kind {
        PrimitiveKind::Bool => {
            body.locals.push(LocalDecl::new(
                Some("raw".into()),
                Ty::named("bool"),
                false,
                None,
                LocalKind::Temp,
            ));
            let mut switch = BasicBlock::new(BlockId(0), None);
            switch.statements.push(Statement {
                span: None,
                kind: StatementKind::Assign {
                    place: Place::new(LocalId(2)),
                    value: Rvalue::Use(value_operand),
                },
            });
            switch.terminator = Some(Terminator::SwitchInt {
                discr: Operand::Copy(Place::new(LocalId(2))),
                targets: vec![(0, BlockId(2)), (1, BlockId(1))],
                otherwise: BlockId(2),
            });
            body.blocks.push(switch);

            let mut true_block = BasicBlock::new(BlockId(1), None);
            true_block.statements.push(Statement {
                span: None,
                kind: StatementKind::Assign {
                    place: Place::new(LocalId(0)),
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::UInt(1)))),
                },
            });
            true_block.terminator = Some(Terminator::Return);
            body.blocks.push(true_block);

            let mut false_block = BasicBlock::new(BlockId(2), None);
            false_block.statements.push(Statement {
                span: None,
                kind: StatementKind::Assign {
                    place: Place::new(LocalId(0)),
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::UInt(0)))),
                },
            });
            false_block.terminator = Some(Terminator::Return);
            body.blocks.push(false_block);
        }
        PrimitiveKind::Char { .. } => {
            let mut entry = BasicBlock::new(BlockId(0), None);
            entry.statements.push(Statement {
                span: None,
                kind: StatementKind::Assign {
                    place: Place::new(LocalId(0)),
                    value: Rvalue::Cast {
                        kind: CastKind::IntToInt,
                        operand: value_operand,
                        source: primitive_ty,
                        target: Ty::named("ulong"),
                        rounding: None,
                    },
                },
            });
            entry.terminator = Some(Terminator::Return);
            body.blocks.push(entry);
        }
        PrimitiveKind::Int {
            bits,
            signed,
            pointer_sized,
        } => {
            let unsigned_name = if *pointer_sized {
                "nuint"
            } else {
                match bits {
                    8 => "byte",
                    16 => "ushort",
                    32 => "uint",
                    64 => "ulong",
                    _ => return None,
                }
            };
            let mut entry = BasicBlock::new(BlockId(0), None);
            if *signed && unsigned_name != "ulong" {
                body.locals.push(LocalDecl::new(
                    Some("unsigned".into()),
                    Ty::named(unsigned_name),
                    false,
                    None,
                    LocalKind::Temp,
                ));
                entry.statements.push(Statement {
                    span: None,
                    kind: StatementKind::Assign {
                        place: Place::new(LocalId(2)),
                        value: Rvalue::Cast {
                            kind: CastKind::IntToInt,
                            operand: value_operand,
                            source: primitive_ty.clone(),
                            target: Ty::named(unsigned_name),
                            rounding: None,
                        },
                    },
                });
                entry.statements.push(Statement {
                    span: None,
                    kind: StatementKind::Assign {
                        place: Place::new(LocalId(0)),
                        value: Rvalue::Cast {
                            kind: CastKind::IntToInt,
                            operand: Operand::Copy(Place::new(LocalId(2))),
                            source: Ty::named(unsigned_name),
                            target: Ty::named("ulong"),
                            rounding: None,
                        },
                    },
                });
            } else {
                entry.statements.push(Statement {
                    span: None,
                    kind: StatementKind::Assign {
                        place: Place::new(LocalId(0)),
                        value: Rvalue::Cast {
                            kind: CastKind::IntToInt,
                            operand: value_operand,
                            source: primitive_ty,
                            target: Ty::named("ulong"),
                            rounding: None,
                        },
                    },
                });
            }
            entry.terminator = Some(Terminator::Return);
            body.blocks.push(entry);
        }
        PrimitiveKind::Float { .. }
        | PrimitiveKind::Decimal
        | PrimitiveKind::String
        | PrimitiveKind::Str
        | PrimitiveKind::Void => return None,
    }

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty],
            ret: Ty::named("ulong"),
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

fn synthesize_hash_function(
    ty_name: &str,
    method_symbol: &str,
    arg_mode: ParamMode,
) -> Option<MirFunction> {
    let name = hash_glue_symbol_for(ty_name);
    let ty = parse_type_from_name(ty_name)?;
    let typed_pointer_ty = pointer_ty_for(ty.clone());
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("ulong"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        typed_pointer_ty,
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("hash_i32".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("hash_u32".into()),
        Ty::named("uint"),
        false,
        None,
        LocalKind::Temp,
    ));

    let value_operand = Operand::Copy(Place {
        local: LocalId(1),
        projection: vec![ProjectionElem::Deref],
    });

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            method_symbol.to_string(),
        ))),
        args: vec![value_operand],
        arg_modes: vec![arg_mode],
        destination: Some(Place::new(LocalId(2))),
        target: BlockId(1),
        unwind: None,
        dispatch: None,
    });
    body.blocks.push(entry);

    let mut exit = BasicBlock::new(BlockId(1), None);
    exit.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Cast {
                kind: CastKind::IntToInt,
                operand: Operand::Copy(Place::new(LocalId(2))),
                source: Ty::named("int"),
                target: Ty::named("uint"),
                rounding: None,
            },
        },
    });
    exit.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Cast {
                kind: CastKind::IntToInt,
                operand: Operand::Copy(Place::new(LocalId(3))),
                source: Ty::named("uint"),
                target: Ty::named("ulong"),
                rounding: None,
            },
        },
    });
    exit.terminator = Some(Terminator::Return);
    body.blocks.push(exit);

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty],
            ret: Ty::named("ulong"),
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
