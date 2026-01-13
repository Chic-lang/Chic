use super::type_fixtures::sample_pair_layout;
use crate::codegen::wasm::write_u32;
use crate::mir::*;

pub(crate) fn scalar_local_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block0 = BasicBlock::new(BlockId(0), None);
    block0.terminator = Some(Terminator::Return);
    body.blocks.push(block0);

    MirFunction {
        name: "Demo::ScalarLocal".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
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
    }
}

pub(crate) fn function_with_call_destination() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("result".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
        func: Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: "Demo.Callee".into(),
            span: None,
            info: None,
        }),
        args: Vec::new(),
        arg_modes: Vec::new(),
        destination: Some(Place::new(LocalId(1))),
        target: BlockId(1),
        unwind: None,

        dispatch: None,
    });
    body.blocks.push(entry);

    let mut target = BasicBlock::new(BlockId(1), None);
    target.terminator = Some(Terminator::Return);
    body.blocks.push(target);

    MirFunction {
        name: "Demo::CallWithDestination".into(),
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
    }
}

pub(crate) fn struct_assign_via_projection_function() -> (TypeLayoutTable, MirFunction) {
    let layouts = sample_pair_layout();
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("pair".into()),
        Ty::named("Demo::Pair"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut projection = Place::new(LocalId(1));
    projection.projection.push(ProjectionElem::Field(1));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: projection,
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(42)))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Demo::StructAssign".into(),
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

    (layouts, function)
}

pub(crate) fn nested_block_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Goto { target: BlockId(1) });
    body.blocks.push(entry);

    let mut next = BasicBlock::new(BlockId(1), None);
    next.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    next.terminator = Some(Terminator::Return);
    body.blocks.push(next);

    MirFunction {
        name: "Demo::NestedBlocks".into(),
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
    }
}

pub(crate) fn function_with_unreachable() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Unreachable);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Unreachable".into(),
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
    }
}

pub(crate) fn function_with_assert() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assert {
            cond: Operand::Const(ConstOperand::new(ConstValue::Bool(true))),
            expected: true,
            message: "should never fail".into(),
            target: BlockId(1),
            cleanup: None,
        },
    });
    entry.terminator = Some(Terminator::Goto { target: BlockId(1) });
    body.blocks.push(entry);

    let mut success = BasicBlock::new(BlockId(1), None);
    success.terminator = Some(Terminator::Return);
    body.blocks.push(success);

    MirFunction {
        name: "Demo::Assert".into(),
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
    }
}

pub(crate) fn function_with_missing_terminator() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    body.blocks.push(entry);

    let mut exit = BasicBlock::new(BlockId(1), None);
    exit.terminator = Some(Terminator::Return);
    body.blocks.push(exit);

    MirFunction {
        name: "Demo::MissingTerminator".into(),
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
    }
}

pub(crate) fn function_with_panic() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Panic);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Panic".into(),
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
    }
}

pub(crate) fn function_with_throw() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("payload".into()),
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
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0x1234)))),
        },
    });
    entry.terminator = Some(Terminator::Throw {
        exception: Some(Operand::Copy(Place::new(LocalId(1)))),
        ty: Some(Ty::named("Demo::Failure")),
    });
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Throw".into(),
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
    }
}

pub(crate) fn function_with_yield() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Yield {
        value: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
        resume: BlockId(1),
        drop: BlockId(2),
    });
    body.blocks.push(entry);

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.terminator = Some(Terminator::Return);
    body.blocks.push(resume);

    let mut drop = BasicBlock::new(BlockId(2), None);
    drop.terminator = Some(Terminator::Return);
    body.blocks.push(drop);

    MirFunction {
        name: "Demo::Yielding".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: false,
        async_result: None,
        is_generator: true,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

pub(crate) fn function_with_pending() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Pending(PendingTerminator {
        kind: PendingTerminatorKind::Await,
        detail: Some("await".into()),
    }));
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Pending".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: true,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

pub(crate) fn async_single_await_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("future".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("result".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Await {
        future: Place::new(LocalId(1)),
        destination: Some(Place::new(LocalId(2))),
        resume: BlockId(1),
        drop: BlockId(2),
    });
    body.blocks.push(entry);

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.statements.push(Statement {
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(2)))),
        },
        span: None,
    });
    resume.terminator = Some(Terminator::Return);
    body.blocks.push(resume);

    let mut drop_block = BasicBlock::new(BlockId(2), None);
    drop_block.terminator = Some(Terminator::Return);
    body.blocks.push(drop_block);

    MirFunction {
        name: "Demo::AsyncAwait".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: true,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

pub(crate) fn function_with_missing_target() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Goto {
        target: BlockId(42),
    });
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::MissingTarget".into(),
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
    }
}

pub(crate) fn function_with_switch() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("discr".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(Place::new(LocalId(1))),
        targets: vec![(0, BlockId(1)), (1, BlockId(2))],
        otherwise: BlockId(3),
    });
    body.blocks.push(entry);

    for id in 1..=3 {
        let mut block = BasicBlock::new(BlockId(id), None);
        block.terminator = Some(Terminator::Return);
        body.blocks.push(block);
    }

    MirFunction {
        name: "Demo::Switch".into(),
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
    }
}

pub(crate) fn function_with_return_arg_call() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    let args = vec![Operand::Copy(Place::new(LocalId(0)))];
    let arg_modes = vec![ParamMode::Value; args.len()];
    entry.terminator = Some(Terminator::Call {
        func: Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: "Demo.Callee".into(),
            span: None,
            info: None,
        }),
        args,
        arg_modes,
        destination: None,
        target: BlockId(1),
        unwind: None,

        dispatch: None,
    });
    body.blocks.push(entry);

    let mut target = BasicBlock::new(BlockId(1), None);
    target.terminator = Some(Terminator::Return);
    body.blocks.push(target);

    MirFunction {
        name: "Demo::ReturnArgCall".into(),
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
    }
}

pub(crate) fn simple_return_block(id: usize) -> BasicBlock {
    let mut block = BasicBlock::new(BlockId(id), None);
    block.terminator = Some(Terminator::Return);
    block
}

pub(crate) fn simple_function(name: &str, kind: FunctionKind, ret: Ty) -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        ret.clone(),
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(simple_return_block(0));
    MirFunction {
        name: name.into(),
        kind,
        signature: FnSig {
            params: Vec::new(),
            ret,
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
    }
}

pub(crate) fn module_with_functions(functions: Vec<MirFunction>) -> MirModule {
    let type_layouts = super::wasm_layouts();
    let primitive_registry = type_layouts.primitive_registry.clone();
    MirModule {
        functions,
        test_cases: Vec::new(),
        default_arguments: Vec::new(),
        statics: Vec::new(),
        type_layouts,
        primitive_registry,
        interned_strs: Vec::new(),
        exports: Vec::new(),
        attributes: crate::mir::module_metadata::ModuleAttributes::default(),
        trait_vtables: Vec::new(),
        class_vtables: Vec::new(),
        interface_defaults: Vec::new(),
        type_variance: std::collections::HashMap::new(),
        async_plans: Vec::new(),
    }
}

pub(crate) fn decode_exports(payload: &[u8]) -> Vec<(String, u8, u32)> {
    fn read_u32(bytes: &[u8], index: &mut usize) -> u32 {
        let mut result: u32 = 0;
        let mut shift = 0;
        loop {
            let byte = bytes[*index];
            *index += 1;
            result |= u32::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        result
    }

    let mut cursor = 0;
    let export_count = read_u32(payload, &mut cursor);
    let mut exports = Vec::new();
    for _ in 0..export_count {
        let name_len = read_u32(payload, &mut cursor) as usize;
        let name_bytes = &payload[cursor..cursor + name_len];
        let name = String::from_utf8(name_bytes.to_vec()).expect("utf8 export name");
        cursor += name_len;
        let kind = payload[cursor];
        cursor += 1;
        let index = read_u32(payload, &mut cursor);
        exports.push((name, kind, index));
    }
    exports
}

pub(crate) fn leb_u32(value: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    write_u32(&mut buf, value);
    buf
}

pub(crate) fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
