use std::collections::{BTreeMap, BTreeSet};

use crate::chic_kind::ChicKind;
use crate::mir::{
    Abi, BasicBlock, BinOp, ConstValue, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
    MirModule, Operand, Rvalue, StatementKind, Terminator, Ty,
};

#[derive(Debug, Clone)]
pub struct Cc1Module {
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct Cc1Error {
    pub message: String,
}

pub fn translate(mir: &MirModule, _kind: ChicKind) -> Result<Cc1Module, Cc1Error> {
    let mut out = String::new();
    out.push_str("/* chic cc1 preprocessed output */\n");
    out.push_str("typedef signed int int32_t;\n");
    out.push_str("typedef unsigned int uint32_t;\n");
    out.push_str("typedef _Bool bool;\n");
    out.push_str("#ifndef true\n#define true ((_Bool)1)\n#endif\n");
    out.push_str("#ifndef false\n#define false ((_Bool)0)\n#endif\n\n");

    for func in &mir.functions {
        if func.kind != FunctionKind::Function {
            continue;
        }
        if func.signature.abi != Abi::Chic {
            if is_compiler_generated_extern(&func.name) {
                continue;
            }
            return Err(error(format!(
                "cc1 backend does not support extern ABI for `{}`",
                func.name
            )));
        }
        if func.is_async || func.is_generator {
            return Err(error(format!(
                "cc1 backend does not yet support async or generator functions ({})",
                func.name
            )));
        }
        let body = translate_function(func)?;
        out.push_str(&body);
        out.push('\n');
    }

    if out.trim().is_empty() || mir.functions.is_empty() {
        return Err(error(
            "cc1 backend requires at least one non-async Chic function",
        ));
    }

    Ok(Cc1Module { source: out })
}

fn is_compiler_generated_extern(name: &str) -> bool {
    name.starts_with("__cl_")
}

fn translate_function(func: &MirFunction) -> Result<String, Cc1Error> {
    let locals: BTreeMap<usize, LocalDecl> = func
        .body
        .locals
        .iter()
        .enumerate()
        .map(|(idx, decl)| (idx, decl.clone()))
        .collect();

    let signature = &func.signature;
    debug_assert_eq!(signature.abi, Abi::Chic);

    let ret_ty = map_type(&signature.ret)?;
    let params = translate_params(signature)?;
    let mut body = String::new();
    body.push_str(ret_ty);
    body.push(' ');
    body.push_str(&sanitize_name(&func.name));
    body.push('(');
    body.push_str(&params.join(", "));
    body.push_str(")\n{\n");

    let block_order = linearize_blocks(&func.body)?;
    let mut assignments = Vec::new();
    let mut declared: BTreeSet<usize> = BTreeSet::new();
    for block in &block_order {
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign { place, value } => {
                    if !place.projection.is_empty() {
                        return Err(error("cc1 backend does not support projections"));
                    }
                    let local = place.local.0;
                    let decl = locals.get(&local).ok_or_else(|| {
                        error(format!("unknown local _{} used in assignment", local))
                    })?;
                    let expr = translate_rvalue(value, &locals)?;
                    body.push_str("    ");
                    if !declared.contains(&local) && !matches!(decl.kind, LocalKind::Arg(_)) {
                        let ty = map_type(&decl.ty)?;
                        body.push_str(ty);
                        body.push(' ');
                    }
                    body.push_str(&local_name(local));
                    body.push_str(" = ");
                    body.push_str(&expr);
                    body.push_str(";\n");
                    assignments.push(local);
                    declared.insert(local);
                }
                StatementKind::StorageLive(_) | StatementKind::StorageDead(_) => {}
                StatementKind::Nop | StatementKind::MarkFallibleHandled { .. } => {}
                other => {
                    return Err(error(format!(
                        "cc1 backend encountered unsupported statement {:?} in `{}`",
                        other, func.name
                    )));
                }
            }
        }
    }

    let last_block = block_order.last().ok_or_else(|| {
        error(format!(
            "function `{}` did not contain any blocks",
            func.name
        ))
    })?;
    match &last_block.terminator {
        Some(Terminator::Return) => {
            let ret_local = func
                .body
                .locals
                .iter()
                .enumerate()
                .find(|(_, decl)| matches!(decl.kind, LocalKind::Return))
                .map(|(idx, _)| idx);
            if ret_ty != "void" {
                let ret_local = ret_local.ok_or_else(|| {
                    error(format!(
                        "function `{}` missing return slot for non-void return",
                        func.name
                    ))
                })?;
                if !assignments.contains(&ret_local) {
                    return Err(error(format!(
                        "return slot _{} was not assigned before return in `{}`",
                        ret_local, func.name
                    )));
                }
                body.push_str("    return ");
                body.push_str(&local_name(ret_local));
                body.push_str(";\n");
            } else {
                body.push_str("    return;\n");
            }
        }
        other => {
            return Err(error(format!(
                "cc1 backend requires return terminator; found {:?} in `{}`",
                other, func.name
            )));
        }
    }

    body.push_str("}\n");
    Ok(body)
}

fn translate_params(sig: &crate::mir::FnSig) -> Result<Vec<String>, Cc1Error> {
    let mut params = Vec::new();
    for (index, ty) in sig.params.iter().enumerate() {
        let mapped = map_type(ty)?;
        params.push(format!("{mapped} {}", local_name(index)));
    }
    if params.is_empty() {
        Ok(vec!["void".into()])
    } else {
        Ok(params)
    }
}

fn translate_rvalue(
    value: &Rvalue,
    locals: &BTreeMap<usize, LocalDecl>,
) -> Result<String, Cc1Error> {
    match value {
        Rvalue::Use(Operand::Const(constant)) => translate_const(&constant.value),
        Rvalue::Use(Operand::Copy(place) | Operand::Move(place)) => {
            if !place.projection.is_empty() {
                return Err(error("cc1 backend does not support place projections"));
            }
            Ok(local_name(place.local.0))
        }
        Rvalue::Binary { op, lhs, rhs, .. } => {
            let lhs = translate_operand(lhs, locals)?;
            let rhs = translate_operand(rhs, locals)?;
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Rem => "%",
                BinOp::BitAnd => "&",
                BinOp::BitOr => "|",
                BinOp::BitXor => "^",
                BinOp::Shl => "<<",
                BinOp::Shr => ">>",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::NullCoalesce => {
                    return Err(error(
                        "cc1 backend cannot translate null-coalescing expressions",
                    ));
                }
            };
            Ok(format!("({lhs} {op_str} {rhs})"))
        }
        other => Err(error(format!(
            "cc1 backend cannot translate rvalue {:?}",
            other
        ))),
    }
}

fn translate_operand(
    operand: &Operand,
    locals: &BTreeMap<usize, LocalDecl>,
) -> Result<String, Cc1Error> {
    match operand {
        Operand::Const(constant) => translate_const(&constant.value),
        Operand::Copy(place) | Operand::Move(place) => {
            if !place.projection.is_empty() {
                return Err(error("cc1 backend does not support projections"));
            }
            if !locals.contains_key(&place.local.0) {
                return Err(error(format!(
                    "cc1 backend encountered reference to unknown local _{}",
                    place.local.0
                )));
            }
            Ok(local_name(place.local.0))
        }
        other => Err(error(format!(
            "cc1 backend encountered unsupported operand {:?}",
            other
        ))),
    }
}

fn translate_const(value: &ConstValue) -> Result<String, Cc1Error> {
    match value {
        ConstValue::Int(i) => Ok(format!("{i}")),
        ConstValue::UInt(u) => Ok(format!("{u}u")),
        ConstValue::Bool(true) => Ok("true".into()),
        ConstValue::Bool(false) => Ok("false".into()),
        ConstValue::Float(f) => Ok(f.display()),
        other => Err(error(format!(
            "cc1 backend does not yet support constant value {:?}",
            other
        ))),
    }
}

fn map_type(ty: &Ty) -> Result<&'static str, Cc1Error> {
    match ty {
        Ty::Unit => Ok("void"),
        Ty::Named(name) => match name.as_str() {
            "int" | "System::Int32" | "Std::Int32" => Ok("int32_t"),
            "uint" | "System::UInt32" | "Std::UInt32" => Ok("uint32_t"),
            "bool" | "System::Bool" | "Std::Bool" => Ok("bool"),
            other => Err(error(format!(
                "cc1 backend does not support Chic type `{other}`"
            ))),
        },
        Ty::Nullable(_) => Err(error(
            "cc1 backend does not support nullable types in signatures",
        )),
        Ty::Unknown => Ok("int32_t"),
        _ => Err(error(format!(
            "cc1 backend does not support composite type {:?}",
            ty
        ))),
    }
}

fn local_name(index: usize) -> String {
    format!("_{}", index)
}

fn sanitize_name(name: &str) -> String {
    name.replace("::", "_")
}

fn error(message: impl Into<String>) -> Cc1Error {
    Cc1Error {
        message: message.into(),
    }
}

fn linearize_blocks(body: &MirBody) -> Result<Vec<&BasicBlock>, Cc1Error> {
    let mut order = Vec::new();
    let mut visited = BTreeSet::new();
    let mut current = body.entry();

    loop {
        if !visited.insert(current.0) {
            return Err(error("cc1 backend requires acyclic linear control flow"));
        }
        let block = body
            .blocks
            .get(current.0)
            .ok_or_else(|| error(format!("missing basic block {}", current.0)))?;
        order.push(block);
        match block.terminator.as_ref() {
            Some(Terminator::Goto { target }) => {
                current = *target;
            }
            Some(Terminator::Return) => break,
            other => {
                return Err(error(format!(
                    "cc1 backend requires linear goto/return terminators; found {:?}",
                    other
                )));
            }
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        Abi, BasicBlock, ConstOperand, ConstValue, FunctionKind, LocalDecl, LocalKind, MirBody,
        MirFunction, MirModule, Operand, Place, Rvalue, Statement, StatementKind, Terminator, Ty,
    };

    #[test]
    fn translator_emits_constant_return_function() {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            None,
            Ty::named("int"),
            false,
            None,
            LocalKind::Return,
        ));
        let mut block = BasicBlock::new(body.entry(), None);
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(crate::mir::LocalId(0)),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(7)))),
            },
        });
        block.terminator = Some(Terminator::Return);
        body.blocks.push(block);

        let mut module = MirModule::default();
        module.functions.push(MirFunction {
            name: "Example::Main".into(),
            kind: FunctionKind::Function,
            signature: crate::mir::FnSig {
                params: Vec::new(),
                ret: Ty::named("int"),
                abi: Abi::Chic,
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
        });

        let cc1 = translate(&module, ChicKind::Executable).expect("translate cc1");
        assert!(cc1.source.contains("int32_t Example_Main(void)"));
        assert!(cc1.source.contains("return _0;"));
    }

    #[test]
    fn translator_rejects_multiple_blocks() {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            None,
            Ty::named("int"),
            false,
            None,
            LocalKind::Return,
        ));
        body.blocks
            .push(BasicBlock::new(crate::mir::BlockId(0), None));
        body.blocks
            .push(BasicBlock::new(crate::mir::BlockId(1), None));
        let mut module = MirModule::default();
        module.functions.push(MirFunction {
            name: "Example::Main".into(),
            kind: FunctionKind::Function,
            signature: crate::mir::FnSig {
                params: Vec::new(),
                ret: Ty::named("int"),
                abi: Abi::Chic,
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
        });

        let err = translate(&module, ChicKind::Executable).expect_err("translation should fail");
        assert!(
            err.message.contains("linear goto/return terminators"),
            "unexpected error message: {}",
            err.message
        );
    }
}
