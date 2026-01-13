#![cfg(test)]

use super::common::*;
use super::helpers::*;
use crate::codegen::wasm::{STACK_POINTER_GLOBAL_INDEX, push_i32_const};
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, Operand, Place, Rvalue, SpanTy, Statement, StatementKind,
    Terminator, Ty, TypeLayoutTable,
};

fn span_stack_alloc_fixture(element: Ty) -> (TypeLayoutTable, MirFunction) {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    let len_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("len".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    let span_ty = Ty::Span(SpanTy::new(Box::new(element.clone())));
    let span_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("span".into()),
        span_ty,
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(len_local),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(4)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(span_local),
            value: Rvalue::SpanStackAlloc {
                element,
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Demo::SpanAlloc".into(),
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
    };

    (wasm_layouts(), function)
}

fn double_span_stack_alloc_fixture(element: Ty) -> (TypeLayoutTable, MirFunction) {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    let len_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("len".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    let span_ty = Ty::Span(SpanTy::new(Box::new(element.clone())));
    let first_span = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("first".into()),
        span_ty.clone(),
        true,
        None,
        LocalKind::Local,
    ));
    let second_span = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("second".into()),
        span_ty,
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(len_local),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(2)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(first_span),
            value: Rvalue::SpanStackAlloc {
                element: element.clone(),
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(second_span),
            value: Rvalue::SpanStackAlloc {
                element,
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Demo::SpanAllocTwice".into(),
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
    };

    (wasm_layouts(), function)
}

fn span_stack_alloc_with_source_fixture(element: Ty) -> (TypeLayoutTable, MirFunction) {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    let len_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("len".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    let span_ty = Ty::Span(SpanTy::new(Box::new(element.clone())));
    let source_span = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("source".into()),
        span_ty.clone(),
        true,
        None,
        LocalKind::Local,
    ));
    let dest_span = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("dest".into()),
        span_ty,
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(len_local),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(3)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(source_span),
            value: Rvalue::SpanStackAlloc {
                element: element.clone(),
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(dest_span),
            value: Rvalue::SpanStackAlloc {
                element,
                length: Operand::Copy(Place::new(len_local)),
                source: Some(Operand::Copy(Place::new(source_span))),
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Demo::SpanAllocWithSource".into(),
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
    };

    (wasm_layouts(), function)
}

fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[test]
fn span_stack_alloc_updates_stack_pointer_and_struct_fields() {
    let (layouts, function) = span_stack_alloc_fixture(Ty::named("int"));
    let body = emit_body_using_layouts(layouts.clone(), function.clone(), |_| None);
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut global_get = vec![0x23];
            global_get.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
            assert!(
                contains_bytes(&body, &global_get),
                "span stack allocation should read the stack pointer"
            );
            let mut adjust_get = vec![0x20];
            adjust_get.extend(leb_u32(emitter.stack_adjust_local));
            assert!(
                contains_bytes(&body, &adjust_get),
                "span stack allocation should load the adjustment local"
            );
            assert!(
                count_occurrences(&body, &adjust_get) >= 2,
                "stack allocations should load the adjustment local outside teardown"
            );
            let mut adjust_set = vec![0x21];
            adjust_set.extend(leb_u32(emitter.stack_adjust_local));
            assert!(
                contains_bytes(&body, &adjust_set),
                "span stack allocation should update the adjustment local"
            );
            assert!(
                count_occurrences(&body, &adjust_set) >= 2,
                "stack allocations should write to the adjustment local in addition to the prelude reset"
            );
            let mut global_set = vec![0x24];
            global_set.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
            assert!(
                contains_bytes(&body, &global_set),
                "span stack allocation should write the stack pointer global"
            );
            let mut size_const = Vec::new();
            push_i32_const(&mut size_const, 4);
            assert!(
                contains_bytes(&body, &size_const),
                "span stack allocation should store the element size"
            );
            let mut align_offset = Vec::new();
            push_i32_const(&mut align_offset, 12);
            assert!(
                contains_bytes(&body, &align_offset),
                "span stack allocation should write the element alignment field"
            );
        },
    );
}

#[test]
fn span_stack_alloc_skips_stack_adjustment_for_zero_sized_elements() {
    let (layouts, function) = span_stack_alloc_fixture(Ty::Unit);
    let body = emit_body_using_layouts(layouts.clone(), function.clone(), |_| None);
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut adjust_get = vec![0x20];
            adjust_get.extend(leb_u32(emitter.stack_adjust_local));
            assert_eq!(
                count_occurrences(&body, &adjust_get),
                1,
                "only teardown should load the adjustment local for zero-sized spans"
            );
            let mut adjust_set = vec![0x21];
            adjust_set.extend(leb_u32(emitter.stack_adjust_local));
            assert_eq!(
                count_occurrences(&body, &adjust_set),
                1,
                "zero-sized spans should only see the prelude reset of the adjustment local"
            );
            let mut sentinel = Vec::new();
            push_i32_const(&mut sentinel, 1);
            assert!(
                contains_bytes(&body, &sentinel),
                "zero-sized spans should still populate the sentinel pointer"
            );
        },
    );
}

#[test]
fn span_stack_alloc_tracks_multiple_allocations() {
    let (layouts, function) = double_span_stack_alloc_fixture(Ty::named("int"));
    let body = emit_body_using_layouts(layouts.clone(), function.clone(), |_| None);
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut adjust_set = vec![0x21];
            adjust_set.extend(leb_u32(emitter.stack_adjust_local));
            assert!(
                count_occurrences(&body, &adjust_set) >= 3,
                "two stack allocations should update the adjustment local twice in addition to the prelude reset"
            );
            let mut global_set = vec![0x24];
            global_set.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
            assert!(
                count_occurrences(&body, &global_set) >= 2,
                "stack pointer global should be updated for each stack allocation"
            );
        },
    );
}

#[test]
fn span_stack_alloc_with_source_invokes_runtime_copy() {
    let (layouts, function) = span_stack_alloc_with_source_fixture(Ty::named("int"));
    let body = emit_body_using_layouts(layouts.clone(), function.clone(), |_| None);
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let hook = emitter
                .lookup_function_index("chic_rt_span_copy_to")
                .expect("runtime copy hook available for spans");
            let mut call = vec![0x10];
            call.extend(leb_u32(hook));
            assert!(
                contains_bytes(&body, &call),
                "stackalloc with source should call runtime copy helper"
            );
        },
    );
}
