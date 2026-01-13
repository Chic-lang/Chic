use super::fixtures::emit_result;
use crate::codegen::llvm::emitter::function::tests::helpers::flag_layouts;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, Place, PointerTy, Terminator, Ty,
};

#[test]
fn startup_async_call_emits_marshalled_arguments() {
    let module = startup_async_module();
    let body = emit_result(&module, "Demo::Startup::AsyncCall")
        .expect("emit async startup call should succeed");
    assert!(
        body.contains(
            "@chic_rt_startup_call_entry_async(ptr null, i32 1, i32 2, ptr null, ptr null)"
        ),
        "async startup call should marshal all five arguments with pointer width ints: {body}"
    );
    assert!(
        body.contains("br label %bb1"),
        "call terminator should branch to the target label: {body}"
    );
}

#[test]
fn startup_runtime_paths_cover_main_intrinsics() {
    let module = runtime_coverage_module();
    let body =
        emit_result(&module, "Demo::Runtime::Coverage").expect("emit runtime coverage function");
    assert!(
        body.contains("@chic_rt_startup_descriptor_snapshot()"),
        "descriptor snapshot should be emitted: {body}"
    );
    assert!(
        body.contains("@chic_rt_startup_call_entry_async"),
        "async entry call should be emitted: {body}"
    );
    assert!(
        body.contains("@chic_rt_startup_complete_entry_async"),
        "completion helpers should be emitted: {body}"
    );
    assert!(
        body.contains("@chic_rt_stderr_flush"),
        "IO flush paths should be emitted: {body}"
    );
    assert!(
        body.contains("@chic_rt_object_new"),
        "object_new runtime call should be covered: {body}"
    );
}

#[test]
fn object_new_rejects_multiple_arguments() {
    let module = object_new_bad_module();
    let err = emit_result(&module, "Demo::Runtime::BadObjectNew")
        .expect_err("invalid object_new args should fail lowering");
    assert!(
        err.contains("expects a single type-id argument"),
        "expected object_new arg count error, got {err:?}"
    );
}

#[test]
fn startup_exit_rejects_destination() {
    let module = startup_exit_destination_module();
    let err = emit_result(&module, "Demo::Startup::ExitWithDest")
        .expect_err("startup exit should reject destinations");
    assert!(
        err.contains("startup exit call cannot assign"),
        "expected startup-exit destination error, got {err:?}"
    );
}

fn startup_async_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts = flag_layouts();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(startup_async_function());
    module
}

fn startup_async_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("result".into()),
        Ty::named("long"),
        false,
        None,
        LocalKind::Temp,
    ));

    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_startup_call_entry_async".into(),
            ))),
            args: vec![
                Operand::Const(ConstOperand::new(ConstValue::Null)),
                Operand::Const(ConstOperand::new(ConstValue::Int32(1))),
                Operand::Const(ConstOperand::new(ConstValue::Int32(2))),
                Operand::Const(ConstOperand::new(ConstValue::Null)),
                Operand::Const(ConstOperand::new(ConstValue::Null)),
            ],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Startup::AsyncCall".into(),
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
    }
}

fn runtime_coverage_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(runtime_coverage_function());
    module
}

fn runtime_coverage_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("typed_ptr".into()),
        Ty::named("byte**"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("wide".into()),
        Ty::named("long"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("narrow".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("str".into()),
        Ty::String,
        false,
        None,
        LocalKind::Temp,
    ));

    body.blocks.push(block(
        0,
        call_term(
            "chic_rt_object_new",
            vec![const_int(1)],
            Some(Place::new(LocalId(1))),
            1,
        ),
    ));
    body.blocks.push(block(
        1,
        call_term(
            "chic_rt_object_new",
            vec![const_int(2)],
            Some(Place::new(LocalId(2))),
            2,
        ),
    ));
    body.blocks.push(block(
        2,
        call_term(
            "chic_rt_startup_descriptor_snapshot",
            Vec::new(),
            Some(Place::new(LocalId(3))),
            3,
        ),
    ));
    body.blocks.push(block(
        3,
        call_term(
            "chic_rt_startup_test_descriptor",
            vec![Operand::Copy(Place::new(LocalId(3))), const_int(9)],
            None,
            4,
        ),
    ));
    body.blocks.push(block(
        4,
        call_term(
            "chic_rt_startup_store_state",
            vec![const_i32(1), const_null_ptr(), const_null_ptr()],
            None,
            5,
        ),
    ));
    body.blocks.push(block(
        5,
        call_term(
            "chic_rt_startup_raw_argc",
            Vec::new(),
            Some(Place::new(LocalId(4))),
            6,
        ),
    ));
    body.blocks.push(block(
        6,
        call_term(
            "chic_rt_startup_raw_argv",
            Vec::new(),
            Some(Place::new(LocalId(3))),
            7,
        ),
    ));
    body.blocks.push(block(
        7,
        call_term(
            "chic_rt_startup_raw_envp",
            Vec::new(),
            Some(Place::new(LocalId(3))),
            8,
        ),
    ));
    body.blocks.push(block(
        8,
        call_term(
            "chic_rt_startup_ptr_at",
            vec![const_int(11), const_i32(1), const_i32(2)],
            Some(Place::new(LocalId(3))),
            9,
        ),
    ));
    body.blocks.push(block(
        9,
        call_term(
            "chic_rt_startup_call_entry",
            vec![
                const_int(12),
                const_i32(1),
                const_i32(2),
                const_int(3),
                const_int(4),
            ],
            Some(Place::new(LocalId(4))),
            10,
        ),
    ));
    body.blocks.push(block(
        10,
        call_term(
            "chic_rt_startup_call_entry_async",
            vec![
                const_int(13),
                const_i32(3),
                const_i32(4),
                const_int(5),
                const_int(6),
            ],
            Some(Place::new(LocalId(3))),
            11,
        ),
    ));
    body.blocks.push(block(
        11,
        call_term(
            "chic_rt_startup_complete_entry_async",
            vec![const_int(7), const_i32(8)],
            Some(Place::new(LocalId(4))),
            12,
        ),
    ));
    body.blocks.push(block(
        12,
        call_term(
            "chic_rt_startup_call_testcase",
            vec![const_int(9)],
            Some(Place::new(LocalId(4))),
            13,
        ),
    ));
    body.blocks.push(block(
        13,
        call_term(
            "chic_rt_startup_call_testcase_async",
            vec![const_int(10)],
            Some(Place::new(LocalId(3))),
            14,
        ),
    ));
    body.blocks.push(block(
        14,
        call_term(
            "chic_rt_startup_complete_testcase_async",
            vec![const_int(11)],
            Some(Place::new(LocalId(4))),
            15,
        ),
    ));
    body.blocks.push(block(
        15,
        call_term(
            "chic_rt_startup_cstr_to_string",
            vec![const_int(12)],
            Some(Place::new(LocalId(5))),
            16,
        ),
    ));
    body.blocks.push(block(
        16,
        call_term(
            "chic_rt_startup_slice_to_string",
            vec![const_int(13), const_int(14)],
            Some(Place::new(LocalId(5))),
            17,
        ),
    ));
    body.blocks.push(block(
        17,
        call_term(
            "chic_rt_startup_i32_to_string",
            vec![const_i32(15)],
            Some(Place::new(LocalId(5))),
            18,
        ),
    ));
    body.blocks.push(block(
        18,
        call_term(
            "chic_rt_startup_usize_to_string",
            vec![const_int(16)],
            Some(Place::new(LocalId(5))),
            19,
        ),
    ));
    body.blocks.push(block(
        19,
        Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_stdout_write_string".into(),
            ))),
            args: vec![Operand::Move(Place::new(LocalId(5)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(20),
            unwind: None,
            dispatch: None,
        },
    ));
    body.blocks.push(block(
        20,
        Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_stdout_write_line_string".into(),
            ))),
            args: vec![Operand::Move(Place::new(LocalId(5)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(21),
            unwind: None,
            dispatch: None,
        },
    ));
    body.blocks.push(block(
        21,
        call_term("chic_rt_stdout_flush", Vec::new(), None, 22),
    ));
    body.blocks.push(block(
        22,
        Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_stderr_write_string".into(),
            ))),
            args: vec![Operand::Move(Place::new(LocalId(5)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(23),
            unwind: None,
            dispatch: None,
        },
    ));
    body.blocks.push(block(
        23,
        Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_stderr_write_line_string".into(),
            ))),
            args: vec![Operand::Move(Place::new(LocalId(5)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(24),
            unwind: None,
            dispatch: None,
        },
    ));
    body.blocks.push(block(
        24,
        call_term("chic_rt_stderr_flush", Vec::new(), None, 25),
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(25),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Runtime::Coverage".into(),
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
    }
}

fn block(id: u32, term: Terminator) -> BasicBlock {
    BasicBlock {
        id: BlockId(id as usize),
        statements: Vec::new(),
        terminator: Some(term),
        span: None,
    }
}

fn call_term(
    symbol: &str,
    args: Vec<Operand>,
    destination: Option<Place>,
    target: u32,
) -> Terminator {
    Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.into()))),
        args,
        arg_modes: Vec::new(),
        destination,
        target: BlockId(target as usize),
        unwind: None,
        dispatch: None,
    }
}

fn const_int(value: i128) -> Operand {
    Operand::Const(ConstOperand::new(ConstValue::Int(value)))
}

fn const_null_ptr() -> Operand {
    Operand::Const(ConstOperand::new(ConstValue::Null))
}

fn const_i32(value: i128) -> Operand {
    Operand::Const(ConstOperand::new(ConstValue::Int32(value)))
}

fn object_new_bad_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(object_new_bad_function());
    module
}

fn object_new_bad_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_object_new".into(),
            ))),
            args: vec![
                Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                Operand::Const(ConstOperand::new(ConstValue::Int(2))),
            ],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Runtime::BadObjectNew".into(),
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
    }
}

fn startup_exit_destination_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("res".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Temp,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "chic_rt_startup_exit".into(),
            ))),
            args: vec![Operand::Const(ConstOperand::new(ConstValue::Int32(0)))],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    module.functions.push(MirFunction {
        name: "Demo::Startup::ExitWithDest".into(),
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
    });
    module
}
