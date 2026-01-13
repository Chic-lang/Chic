use super::fixtures::emit_result;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FnTy, FunctionKind, LocalDecl,
    LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Rvalue, Statement,
    StatementKind, Terminator, Ty,
};

#[test]
fn function_pointer_call_bitcasts_and_branches() {
    let module = function_pointer_module();
    let body = emit_result(&module, "Demo::CallFnPointer").expect("emit fn pointer caller");
    assert!(
        body.contains("getelementptr"),
        "indirect call should project invoke/context: {body}"
    );
    assert!(
        body.contains("call i64"),
        "indirect call should invoke the typed function pointer: {body}"
    );
    assert!(
        body.contains("br label %bb1"),
        "indirect call should branch to the provided target block: {body}"
    );
}

#[test]
fn direct_call_argument_mismatch_reports_context() {
    let module = direct_call_mismatch_module();
    let err = emit_result(&module, "Demo::Caller").expect_err("call with missing args should fail");
    assert!(
        err.contains("direct call expects 1 arguments but 0 were provided"),
        "expected helper-driven argument count error, got: {err}"
    );
}

#[test]
fn direct_call_emits_assignment_and_branch() {
    let module = direct_call_success_module();
    let body = emit_result(&module, "Demo::CallerOk").expect("emit direct caller");
    assert!(
        body.contains("@Demo__Target_ok("),
        "direct call should invoke the callee symbol: {body}"
    );
    assert!(
        body.contains("br label %bb1"),
        "direct call should branch to the target block: {body}"
    );
}

#[test]
fn direct_call_rejects_void_destination() {
    let module = direct_call_void_dest_module();
    let err = emit_result(&module, "Demo::CallerVoidDest")
        .expect_err("void destination should be rejected");
    assert!(
        err.contains("void call cannot have an assignment destination"),
        "expected void destination error, got {err:?}"
    );
}

#[test]
fn indirect_call_rejects_unit_parameters() {
    let module = function_pointer_bad_args_module();
    let err = emit_result(&module, "Demo::CallFnPointerBadArgs")
        .expect_err("fn pointer call with wrong arity should fail");
    assert!(
        err.contains("function pointer call expects 2 arguments"),
        "expected argument count error, got {err:?}"
    );
}

#[test]
fn indirect_call_allows_void_return_without_destination() {
    let module = function_pointer_void_module();
    let body = emit_result(&module, "Demo::FnPointer::VoidCaller")
        .expect("void fn pointer should lower without destination");
    assert!(
        body.contains("call void"),
        "void fn pointer should emit void call: {body}"
    );
    assert!(
        body.contains("br label %bb1"),
        "void fn pointer should branch to target: {body}"
    );
}

#[test]
fn function_pointer_call_argument_mismatch_errors() {
    let module = function_pointer_bad_args_module();
    let err = emit_result(&module, "Demo::CallFnPointerBadArgs")
        .expect_err("fn pointer call with wrong arity should fail");
    assert!(
        err.contains("function pointer call expects 2 arguments"),
        "expected argument count error, got {err:?}"
    );
}

fn function_pointer_module() -> MirModule {
    let mut module = MirModule::default();
    let fn_ty = FnTy::new(vec![Ty::named("int")], Ty::named("long"), Abi::Chic);
    module.type_layouts.ensure_fn_layout(&fn_ty);
    module.type_layouts.finalize_auto_traits();
    module.functions.push(function_pointer_caller());
    module
}

fn function_pointer_caller() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("long"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("fun".into()),
        Ty::Fn(FnTy::new(
            vec![Ty::named("int")],
            Ty::named("long"),
            Abi::Chic,
        )),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("dest".into()),
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
            func: Operand::Copy(Place::new(LocalId(1))),
            args: vec![Operand::Const(ConstOperand::new(ConstValue::Int32(7)))],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(2))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(2)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::CallFnPointer".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Fn(FnTy::new(
                vec![Ty::named("int")],
                Ty::named("long"),
                Abi::Chic,
            ))],
            ret: Ty::named("long"),
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

fn direct_call_mismatch_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(direct_target_function());
    module.functions.push(direct_caller_function());
    module
}

fn direct_target_function() -> MirFunction {
    let mut body = MirBody::new(1, None);
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
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Target".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("int")],
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

fn direct_caller_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let entry = body.entry();
    body.blocks.push(BasicBlock {
        id: entry,
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol("Demo::Target".into()))),
            args: Vec::new(),
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
        name: "Demo::Caller".into(),
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

fn direct_call_success_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(direct_target_ok_function());
    module.functions.push(direct_caller_ok_function());
    module
}

fn direct_call_void_dest_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(MirFunction {
        name: "Demo::NoRet".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("int")],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: MirBody {
            arg_count: 1,
            locals: vec![
                LocalDecl::new(
                    Some("_ret".into()),
                    Ty::Unit,
                    false,
                    None,
                    LocalKind::Return,
                ),
                LocalDecl::new(
                    Some("val".into()),
                    Ty::named("int"),
                    false,
                    None,
                    LocalKind::Arg(0),
                ),
            ],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                statements: Vec::new(),
                terminator: Some(Terminator::Return),
                span: None,
            }],
            span: None,
            async_machine: None,
            generator: None,
            exception_regions: Vec::new(),
            vectorize_decimal: false,
            effects: Vec::new(),
            stream_metadata: Vec::new(),
            debug_notes: Vec::new(),
        },
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    });
    module.functions.push(MirFunction {
        name: "Demo::CallerVoidDest".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: MirBody {
            arg_count: 0,
            locals: vec![
                LocalDecl::new(
                    Some("_ret".into()),
                    Ty::Unit,
                    false,
                    None,
                    LocalKind::Return,
                ),
                LocalDecl::new(Some("tmp".into()), Ty::Unit, false, None, LocalKind::Temp),
            ],
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    statements: Vec::new(),
                    terminator: Some(Terminator::Call {
                        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                            "Demo::NoRet".into(),
                        ))),
                        args: vec![Operand::Const(ConstOperand::new(ConstValue::Int32(3)))],
                        arg_modes: Vec::new(),
                        destination: Some(Place::new(LocalId(1))),
                        target: BlockId(1),
                        unwind: None,
                        dispatch: None,
                    }),
                    span: None,
                },
                BasicBlock {
                    id: BlockId(1),
                    statements: Vec::new(),
                    terminator: Some(Terminator::Return),
                    span: None,
                },
            ],
            span: None,
            async_machine: None,
            generator: None,
            exception_regions: Vec::new(),
            vectorize_decimal: false,
            effects: Vec::new(),
            stream_metadata: Vec::new(),
            debug_notes: Vec::new(),
        },
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

fn direct_target_ok_function() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("val".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::Target_ok".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("int")],
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

fn direct_caller_ok_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("dest".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Target_ok".into(),
            ))),
            args: vec![Operand::Const(ConstOperand::new(ConstValue::Int32(5)))],
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
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(1)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });

    MirFunction {
        name: "Demo::CallerOk".into(),
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

fn function_pointer_bad_args_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(function_pointer_bad_args_caller());
    module
}

fn function_pointer_bad_args_caller() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("long"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("fun".into()),
        Ty::Fn(FnTy::new(
            vec![Ty::named("int"), Ty::named("int")],
            Ty::named("long"),
            Abi::Chic,
        )),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("dest".into()),
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
            func: Operand::Copy(Place::new(LocalId(1))),
            args: vec![Operand::Const(ConstOperand::new(ConstValue::Int32(7)))],
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(2))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Move(Place::new(LocalId(2)))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::CallFnPointerBadArgs".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Fn(FnTy::new(
                vec![Ty::named("int"), Ty::named("int")],
                Ty::named("long"),
                Abi::Chic,
            ))],
            ret: Ty::named("long"),
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

fn function_pointer_void_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(MirFunction {
        name: "Demo::FnPointer::VoidCaller".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Fn(FnTy::new(
                vec![Ty::named("int")],
                Ty::Unit,
                Abi::Chic,
            ))],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: MirBody {
            arg_count: 1,
            locals: vec![
                LocalDecl::new(
                    Some("_ret".into()),
                    Ty::Unit,
                    false,
                    None,
                    LocalKind::Return,
                ),
                LocalDecl::new(
                    Some("fun".into()),
                    Ty::Fn(FnTy::new(vec![Ty::named("int")], Ty::Unit, Abi::Chic)),
                    false,
                    None,
                    LocalKind::Arg(0),
                ),
            ],
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    statements: Vec::new(),
                    terminator: Some(Terminator::Call {
                        func: Operand::Copy(Place::new(LocalId(1))),
                        args: vec![Operand::Const(ConstOperand::new(ConstValue::Int32(42)))],
                        arg_modes: Vec::new(),
                        destination: None,
                        target: BlockId(1),
                        unwind: None,
                        dispatch: None,
                    }),
                    span: None,
                },
                BasicBlock {
                    id: BlockId(1),
                    statements: Vec::new(),
                    terminator: Some(Terminator::Return),
                    span: None,
                },
            ],
            span: None,
            async_machine: None,
            generator: None,
            exception_regions: Vec::new(),
            vectorize_decimal: false,
            effects: Vec::new(),
            stream_metadata: Vec::new(),
            debug_notes: Vec::new(),
        },
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
