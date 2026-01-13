use std::collections::HashSet;

use blake3::hash;

use crate::frontend::parser::parse_type_expression_text;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, ParamMode, Place, PointerTy,
    ProjectionElem, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use crate::monomorphize::MonomorphizationSummary;

#[derive(Debug, Clone)]
pub struct SynthesisedCloneGlue {
    pub type_name: String,
    pub symbol: String,
    pub function_index: usize,
    pub type_identity: u64,
}

pub fn clone_glue_symbol_for(ty_name: &str) -> String {
    let mut symbol = String::from("__cl_clone__");
    for ch in ty_name.chars() {
        match ch {
            ':' => symbol.push('_'),
            '<' | '>' | ',' | ' ' | '[' | ']' => symbol.push('_'),
            _ => symbol.push(ch),
        }
    }
    symbol
}

#[must_use]
pub fn clone_type_identity(name: &str) -> u64 {
    let digest = hash(name.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
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

fn pointer_ty_for(ty: Ty) -> Ty {
    Ty::Pointer(Box::new(PointerTy::new(ty, true)))
}

fn clone_method_symbol(type_name: &str) -> String {
    format!("{type_name}::Clone::Clone")
}

pub fn synthesise_clone_glue(
    module: &mut MirModule,
    summary: &MonomorphizationSummary,
) -> Vec<SynthesisedCloneGlue> {
    if summary.clone_candidates.is_empty() {
        return Vec::new();
    }

    let mut existing: HashSet<String> = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect();
    let mut synthesised = Vec::new();

    for ty_name in &summary.clone_candidates {
        let symbol = clone_glue_symbol_for(ty_name);
        if existing.contains(&symbol) {
            continue;
        }
        let method_symbol = clone_method_symbol(ty_name);
        if !module.functions.iter().any(|f| f.name == method_symbol) {
            continue;
        }
        let Some(function) = synthesize_clone_function(ty_name, &method_symbol) else {
            continue;
        };
        let index = module.functions.len();
        existing.insert(symbol.clone());
        module.functions.push(function);
        synthesised.push(SynthesisedCloneGlue {
            type_name: ty_name.clone(),
            symbol,
            function_index: index,
            type_identity: clone_type_identity(ty_name),
        });
    }

    synthesised
}

fn synthesize_clone_function(ty_name: &str, method_symbol: &str) -> Option<MirFunction> {
    let name = clone_glue_symbol_for(ty_name);
    let ty = parse_type_from_name(ty_name)?;
    let dest_pointer_ty = pointer_ty_for(ty.clone());
    let src_pointer_ty = pointer_ty_for(ty.clone());
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);

    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("dest".into()),
        dest_pointer_ty,
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("src".into()),
        src_pointer_ty,
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        Some("clone_result".into()),
        ty.clone(),
        false,
        None,
        LocalKind::Temp,
    ));

    let dest_place = Place {
        local: LocalId(1),
        projection: vec![ProjectionElem::Deref],
    };
    let src_operand = Operand::Copy(Place {
        local: LocalId(2),
        projection: vec![ProjectionElem::Deref],
    });

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(3)),
    });
    entry.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            method_symbol.to_string(),
        ))),
        args: vec![src_operand],
        arg_modes: vec![ParamMode::In],
        destination: Some(Place {
            local: LocalId(3),
            projection: Vec::new(),
        }),
        target: BlockId(1),
        unwind: None,
        dispatch: None,
    });
    body.blocks.push(entry);

    let mut exit = BasicBlock::new(BlockId(1), None);
    exit.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: dest_place,
            value: Rvalue::Use(Operand::Move(Place {
                local: LocalId(3),
                projection: Vec::new(),
            })),
        },
    });
    exit.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(LocalId(3)),
    });
    exit.terminator = Some(Terminator::Return);
    body.blocks.push(exit);

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty.clone(), raw_pointer_ty],
            ret: Ty::Unit,
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
