use super::fixtures::emit_result;
use crate::mir::{
    Abi, BasicBlock, BlockId, BorrowKind, BorrowOperand, CallDispatch, ClassVTable,
    ClassVTableSlot, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind,
    MirBody, MirFunction, MirModule, Operand, Place, PointerTy, RegionVar, Terminator,
    TraitObjectDispatch, Ty, VirtualDispatch,
};

#[test]
fn virtual_dispatch_rejects_out_of_range_slot() {
    let module = virtual_out_of_range_module();
    let err = emit_result(&module, "Demo::Animal::InvokeBase")
        .expect_err("virtual dispatch with bad slot should error");
    assert!(
        err.contains("vtable slot 3 is out of range"),
        "expected slot-out-of-range error, got {err:?}"
    );
}

#[test]
fn virtual_dispatch_requires_vtable_metadata() {
    let mut module = virtual_out_of_range_module();
    module.class_vtables.clear();
    let err = emit_result(&module, "Demo::Animal::InvokeBase")
        .expect_err("virtual dispatch without vtable metadata should fail");
    assert!(
        err.contains("does not have vtable metadata"),
        "expected metadata error, got {err:?}"
    );
}

#[test]
fn trait_dispatch_missing_slot_errors() {
    let module = trait_missing_slot_module();
    let err = emit_result(&module, "Demo::Item::Render")
        .expect_err("trait vtable slot error should surface");
    assert!(
        err.contains("vtable is missing slot"),
        "expected missing slot error, got {err:?}"
    );
}

#[test]
fn virtual_dispatch_missing_vtable_metadata_errors() {
    let module = virtual_missing_vtable_module();
    let err = emit_result(&module, "Demo::Animal::InvokeBase")
        .expect_err("missing class vtable metadata should error");
    assert!(
        err.contains("does not have vtable metadata"),
        "expected missing metadata error, got {err:?}"
    );
}

#[test]
fn virtual_dispatch_unknown_target_errors() {
    let module = virtual_unknown_target_module();
    let err = emit_result(&module, "Demo::Animal::InvokeMissing")
        .expect_err("unknown virtual target should error");
    assert!(
        err.contains("unknown call target"),
        "expected unknown target error, got {err:?}"
    );
}

#[test]
fn trait_dispatch_missing_signature_errors() {
    let module = trait_missing_signature_module();
    let err = emit_result(&module, "Demo::Item::RenderMissing")
        .expect_err("trait dispatch should error when method signature missing");
    assert!(
        err.contains("missing LLVM signature for trait method"),
        "expected missing signature error, got {err:?}"
    );
}

#[test]
fn trait_dispatch_without_destination_branches() {
    let module = trait_no_destination_module();
    let body = emit_result(&module, "Demo::Item::RenderNoDest")
        .expect("trait dispatch without destination should lower");
    assert!(
        body.contains("call void"),
        "expected trait call without destination to emit void call: {body}"
    );
    assert!(
        body.contains("br label %bb1"),
        "trait call should branch to target: {body}"
    );
}

#[test]
fn virtual_dispatch_without_destination_emits_branch() {
    let module = virtual_no_destination_module();
    let body = emit_result(&module, "Demo::Tests::InvokeNoDest")
        .expect("virtual dispatch without destination should lower");
    assert!(
        body.contains("call void"),
        "virtual call without destination should emit void call: {body}"
    );
    assert!(
        body.contains("br label %bb1"),
        "virtual call should branch to the next block: {body}"
    );
}

#[test]
fn trait_dispatch_requires_receiver_operand() {
    let module = trait_missing_receiver_module();
    let err = emit_result(&module, "Demo::TraitMissing")
        .expect_err("trait dispatch without receiver should error");
    assert!(
        err.contains("trait object call missing receiver argument"),
        "expected receiver error, got {err:?}"
    );
}

#[test]
fn virtual_dispatch_rejects_non_place_receiver() {
    let module = virtual_non_place_receiver_module();
    let err = emit_result(&module, "Demo::Animal::InvokeConst")
        .expect_err("virtual dispatch should reject const receiver");
    assert!(
        err.contains("virtual dispatch receiver must be addressable"),
        "expected receiver addressability error, got {err:?}"
    );
}

fn virtual_out_of_range_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.class_vtables.push(ClassVTable {
        type_name: "Demo::Animal".into(),
        symbol: "__class_vtable_Demo__Animal".into(),
        version: 0,
        slots: vec![ClassVTableSlot {
            slot_index: 0,
            member: "Speak".into(),
            accessor: None,
            symbol: "Demo::Animal::Speak".into(),
        }],
    });
    module.functions.push(simple_virtual_callee());
    module.functions.push(virtual_invoke_function());
    module
}

fn simple_virtual_callee() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("Demo::Animal"), true))),
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
        name: "Demo::Animal::Speak".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Demo::Animal"),
                true,
            )))],
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

fn virtual_invoke_function() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("Demo::Animal"), true))),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Animal::Speak".into(),
            ))),
            args: vec![Operand::Move(Place::new(LocalId(1)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Virtual(VirtualDispatch {
                slot_index: 3,
                receiver_index: 0,
                base_owner: Some("Demo::Animal".into()),
            })),
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
        name: "Demo::Animal::InvokeBase".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Demo::Animal"),
                true,
            )))],
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

fn virtual_missing_vtable_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(simple_virtual_callee());
    module.functions.push(virtual_invoke_function());
    module
}

fn virtual_unknown_target_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.class_vtables.push(ClassVTable {
        type_name: "Demo::Animal".into(),
        symbol: "__class_vtable_Demo__Animal".into(),
        version: 0,
        slots: vec![ClassVTableSlot {
            slot_index: 0,
            member: "Speak".into(),
            accessor: None,
            symbol: "Demo::Animal::Speak".into(),
        }],
    });
    module.functions.push(simple_virtual_callee());
    // Caller references a different symbol than provided by signatures to trigger unknown target.
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("Demo::Animal"), true))),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Animal::Unknown".into(),
            ))),
            args: vec![Operand::Move(Place::new(LocalId(1)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Virtual(VirtualDispatch {
                slot_index: 0,
                receiver_index: 0,
                base_owner: Some("Demo::Animal".into()),
            })),
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
        name: "Demo::Animal::InvokeMissing".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Demo::Animal"),
                true,
            )))],
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

fn trait_missing_slot_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.trait_vtables.push(crate::mir::TraitVTable {
        symbol: "__vtable_Demo__Formatter__Item".into(),
        trait_name: "Demo::Formatter".into(),
        impl_type: "Demo::Item".into(),
        slots: Vec::new(),
    });
    module
        .functions
        .push(simple_trait_method("Demo::Formatter::Format"));
    module.functions.push(MirFunction {
        name: "Demo::Item::Render".into(),
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
                LocalDecl::new(
                    Some("self".into()),
                    Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
                    false,
                    None,
                    LocalKind::Temp,
                ),
            ],
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    statements: Vec::new(),
                    terminator: Some(Terminator::Call {
                        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                            "Demo::Formatter::Format".into(),
                        ))),
                        args: vec![Operand::Move(Place::new(LocalId(1)))],
                        arg_modes: Vec::new(),
                        destination: None,
                        target: BlockId(1),
                        unwind: None,
                        dispatch: Some(CallDispatch::Trait(TraitObjectDispatch {
                            trait_name: "Demo::Formatter".into(),
                            method: "Format".into(),
                            slot_index: 0,
                            slot_count: 1,
                            receiver_index: 0,
                            impl_type: None,
                        })),
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

fn simple_trait_method(name: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
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
        name: name.into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("byte"),
                true,
            )))],
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

fn trait_missing_signature_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.trait_vtables.push(crate::mir::TraitVTable {
        symbol: "__vtable_Demo__Formatter__Missing".into(),
        trait_name: "Demo::Formatter".into(),
        impl_type: "Demo::Item".into(),
        slots: vec![crate::mir::VTableSlot {
            method: "FormatMissing".into(),
            symbol: "Demo::Formatter::FormatMissing".into(),
        }],
    });
    module
        .functions
        .push(simple_trait_method("Demo::Formatter::Existing"));
    module.functions.push(build_trait_call(
        "Demo::Item::RenderMissing",
        "Demo::Formatter::FormatMissing",
        None,
    ));
    module
}

fn trait_no_destination_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.trait_vtables.push(crate::mir::TraitVTable {
        symbol: "__vtable_Demo__Formatter__ItemNoDest".into(),
        trait_name: "Demo::Formatter".into(),
        impl_type: "Demo::Item".into(),
        slots: vec![crate::mir::VTableSlot {
            method: "Format".into(),
            symbol: "Demo::Formatter::Format".into(),
        }],
    });
    module
        .functions
        .push(simple_trait_method("Demo::Formatter::Format"));
    module.functions.push(build_trait_call(
        "Demo::Item::RenderNoDest",
        "Demo::Formatter::Format",
        None,
    ));
    module
}

fn build_trait_call(name: &str, callee: &str, destination: Option<LocalId>) -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
        false,
        None,
        LocalKind::Temp,
    ));
    if destination.is_some() {
        body.locals.push(LocalDecl::new(
            Some("out".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Temp,
        ));
    }
    let dest_place = destination.map(Place::new);
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(callee.into()))),
            args: vec![Operand::Borrow(BorrowOperand {
                kind: BorrowKind::Shared,
                place: Place::new(LocalId(1)),
                region: RegionVar(0),
                span: None,
            })],
            arg_modes: Vec::new(),
            destination: dest_place,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Trait(TraitObjectDispatch {
                trait_name: "Demo::Formatter".into(),
                method: callee.into(),
                slot_index: 0,
                slot_count: 1,
                receiver_index: 0,
                impl_type: None,
            })),
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
        name: name.into(),
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

fn virtual_no_destination_module() -> MirModule {
    let mut module = MirModule::default();
    module.type_layouts.finalize_auto_traits();
    module.functions.push(simple_virtual_callee());
    module.functions.push(invoke_function_no_destination());
    module
}

fn invoke_function_no_destination() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("Demo::Animal"), true))),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Animal::Speak".into(),
            ))),
            args: vec![Operand::Copy(Place::new(LocalId(1)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Virtual(VirtualDispatch {
                slot_index: 0,
                receiver_index: 0,
                base_owner: None,
            })),
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
        name: "Demo::Tests::InvokeNoDest".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Demo::Animal"),
                true,
            )))],
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

fn trait_missing_receiver_module() -> MirModule {
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
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Formatter::Format".into(),
            ))),
            args: Vec::new(),
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Trait(TraitObjectDispatch {
                trait_name: "Demo::Formatter".into(),
                method: "Format".into(),
                slot_index: 0,
                slot_count: 1,
                receiver_index: 0,
                impl_type: None,
            })),
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
        name: "Demo::TraitMissing".into(),
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

fn virtual_non_place_receiver_module() -> MirModule {
    let mut module = virtual_out_of_range_module();
    let mut bad_body = MirBody::new(0, None);
    bad_body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    bad_body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Animal::Speak".into(),
            ))),
            args: vec![Operand::Const(ConstOperand::new(ConstValue::Int(1)))],
            arg_modes: Vec::new(),
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Virtual(VirtualDispatch {
                slot_index: 0,
                receiver_index: 0,
                base_owner: Some("Demo::Animal".into()),
            })),
        }),
        span: None,
    });
    bad_body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    module.functions.push(MirFunction {
        name: "Demo::Animal::InvokeConst".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: bad_body,
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
