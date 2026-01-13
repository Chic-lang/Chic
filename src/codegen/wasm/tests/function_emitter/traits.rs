#![cfg(test)]

use super::common::*;
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, CallDispatch, ConstOperand, ConstValue,
    FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand,
    ParamMode, Place, StructLayout, Terminator, TraitObjectDispatch, TraitObjectTy, TraitVTable,
    Ty, TypeLayout, TypeRepr, VTableSlot,
};
use std::collections::HashMap;

#[test]
fn emit_trait_object_call_uses_call_indirect() {
    let module = trait_dispatch_module();
    let harness = WasmFunctionHarness::from_module(module);
    let signature_map = trait_signature_map(&harness);
    let body = harness
        .emit_body_with("Demo::Render", |_| Some(signature_map.clone()))
        .expect("emit render");
    assert!(
        body.contains(&0x11),
        "expected call_indirect (0x11) opcode in dyn dispatch: {:?}",
        body
    );
}

fn trait_signature_map(harness: &WasmFunctionHarness) -> HashMap<FunctionSignature, u32> {
    harness
        .module()
        .functions
        .iter()
        .enumerate()
        .map(|(idx, func)| {
            (
                FunctionSignature::from_mir(func, &harness.module().type_layouts),
                idx as u32,
            )
        })
        .collect()
}

fn trait_dispatch_module() -> MirModule {
    let mut module = MirModule::default();
    register_class_layout(&mut module, "Demo::Widget");
    module.trait_vtables.push(TraitVTable {
        symbol: "__vtable_Demo__Formatter__Demo__Widget".into(),
        trait_name: "Demo::Formatter".into(),
        impl_type: "Demo::Widget".into(),
        slots: vec![VTableSlot {
            method: "Format".into(),
            symbol: "Demo::Widget::Formatter::Format".into(),
        }],
    });
    module.functions.push(simple_function(
        "Demo::Widget::Formatter::Format",
        FunctionKind::Function,
        Ty::Unit,
    ));
    module.functions.push(render_function());
    module
}

fn register_class_layout(module: &mut MirModule, name: &str) {
    module.type_layouts.types.insert(
        name.to_string(),
        TypeLayout::Class(StructLayout {
            name: name.to_string(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(0),
            align: Some(1),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

fn render_function() -> MirFunction {
    let trait_ty = Ty::TraitObject(TraitObjectTy::new(vec!["Demo::Formatter".into()]));
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("fmt".into()),
        trait_ty.clone(),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Unit)),
            args: vec![Operand::Copy(Place::new(LocalId(1)))],
            arg_modes: vec![ParamMode::Value],
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
    MirFunction {
        name: "Demo::Render".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![trait_ty],
            ret: Ty::Unit,
            abi: crate::mir::Abi::Chic,
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
