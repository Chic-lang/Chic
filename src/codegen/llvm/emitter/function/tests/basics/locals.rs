use super::super::helpers::test_target;
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody,
    MirFunction, MirModule, Mutability, Place, Rvalue, Statement, StatementKind, StrId, Terminator,
    Ty, TypeLayoutTable,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn address_of_temporaries_keep_pointer_types() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp_ptr".into()),
        Ty::Unknown,
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::AddressOf {
                mutability: Mutability::Immutable,
                place: Place::new(LocalId(2)),
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let mut module = MirModule::default();
    module.type_layouts = TypeLayoutTable::default();
    module.type_layouts.finalize_auto_traits();
    let function = MirFunction {
        name: "Demo::AddrOfTemp".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
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
    };
    module.functions.push(function);

    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut externals = BTreeSet::new();
    let str_literals: HashMap<StrId, StrLiteralInfo> = HashMap::new();
    let symbol_set = HashSet::new();
    let mut metadata = MetadataRegistry::new();
    let emitter = FunctionEmitter::new(
        &module.functions[0],
        &signatures,
        &mut externals,
        &symbol_set,
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &str_literals,
        &module.type_layouts,
        &mut metadata,
        None,
    );

    let locals = emitter.compute_local_types().expect("local types");
    assert_eq!(
        locals.get(1).and_then(|ty| ty.as_deref()),
        Some("ptr"),
        "address-of temporaries should retain pointer representation"
    );
}
