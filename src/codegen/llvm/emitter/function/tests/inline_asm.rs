use super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, InlineAsm,
    InlineAsmOperand, InlineAsmOperandKind, InlineAsmOptions, InlineAsmRegister,
    InlineAsmRegisterClass, InlineAsmTemplatePiece, LocalDecl, LocalId, LocalKind, MirBody,
    MirFunction, MirModule, Operand, Place, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn inline_asm_lowers_to_llvm_ir() {
    let module = inline_asm_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("llvm signatures");
    let function = &module.functions[0];
    let sig = signatures
        .get(&function.name)
        .unwrap_or_else(|| panic!("missing signature for {}", function.name));
    let mut externals = BTreeSet::new();
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut externals,
        &HashSet::new(),
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit inline asm function");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("asm sideeffect inteldialect \"xor $0, $0\""),
        "expected inline asm template in IR: {body}"
    );
    assert!(
        body.contains("+r"),
        "expected register constraint in inline asm: {body}"
    );
    assert!(
        body.contains("~{xmm0}"),
        "expected clobber to surface in constraints: {body}"
    );
}

fn inline_asm_module() -> MirModule {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("dst".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    let asm = InlineAsm {
        template: vec![
            InlineAsmTemplatePiece::Literal("xor ".into()),
            InlineAsmTemplatePiece::Placeholder {
                operand_idx: 0,
                modifier: None,
                span: None,
            },
            InlineAsmTemplatePiece::Literal(", ".into()),
            InlineAsmTemplatePiece::Placeholder {
                operand_idx: 0,
                modifier: None,
                span: None,
            },
        ],
        operands: vec![InlineAsmOperand {
            name: Some("dst".into()),
            reg: InlineAsmRegister::Class(InlineAsmRegisterClass::Reg),
            kind: InlineAsmOperandKind::InOut {
                input: Operand::Copy(Place::new(LocalId(1))),
                output: Place::new(LocalId(1)),
                late: true,
            },
            span: None,
        }],
        clobbers: vec![InlineAsmRegister::Explicit("xmm0".into())],
        options: InlineAsmOptions {
            volatile: true,
            intel_syntax: true,
            ..InlineAsmOptions::default()
        },
        span: None,
    };
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::InlineAsm(asm),
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Sample::Asm".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    let mut module = MirModule::default();
    module.functions.push(function);
    module
}
