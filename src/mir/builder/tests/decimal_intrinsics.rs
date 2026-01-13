use super::super::static_registry::StaticRegistry;
use super::common::RequireExt;
use super::module_lowering::driver::TypeDeclInfo;
use super::module_lowering::traits::TraitLoweringInfo;
use super::*;
use crate::decimal::Decimal128;
use crate::frontend::import_resolver::ImportResolver;
use crate::mir::SymbolIndex;
use crate::mir::builder::FunctionSpecialization;
use crate::mir::builder::body_builder::BodyBuilder;
use crate::mir::builder::default_arguments::{DefaultArgumentMap, DefaultArgumentStore};
use crate::mir::builder::string_interner::StringInterner;
use crate::mir::data::{
    Abi, ConstOperand, ConstValue, DecimalIntrinsic, DecimalIntrinsicKind, FnSig, Operand,
    PendingOperand, Place, Rvalue, StatementKind, Ty, ValueCategory,
};
use crate::mir::operators::OperatorRegistry;
use crate::primitives::PrimitiveRegistry;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const DECIMAL_INTRINSIC_RESULT_TY: &str = "Std::Numeric::Decimal::DecimalIntrinsicResult";

fn default_argument_store() -> DefaultArgumentStore {
    Rc::new(RefCell::new(DefaultArgumentMap::default()))
}

fn empty_specializations() -> Rc<RefCell<Vec<FunctionSpecialization>>> {
    Rc::new(RefCell::new(Vec::new()))
}

fn collect_decimal_intrinsic(body: &MirBody) -> &DecimalIntrinsic {
    for block in &body.blocks {
        for statement in &block.statements {
            if let StatementKind::Assign { value, .. } = &statement.kind {
                if let Rvalue::DecimalIntrinsic(intrinsic) = value {
                    return intrinsic;
                }
            }
        }
    }
    panic!("decimal intrinsic rvalue not emitted");
}

#[test]
fn decimal_intrinsic_rvalue_formatting() {
    let rvalue = Rvalue::DecimalIntrinsic(DecimalIntrinsic {
        kind: DecimalIntrinsicKind::Add,
        lhs: Operand::Copy(Place::new(LocalId(1))),
        rhs: Operand::Copy(Place::new(LocalId(2))),
        addend: None,
        rounding: Operand::Const(ConstOperand::new(ConstValue::Enum {
            type_name: "Std::Numeric::Decimal::DecimalRoundingMode".into(),
            variant: "TiesToEven".into(),
            discriminant: 0,
        })),
        vectorize: Operand::Const(ConstOperand::new(ConstValue::Enum {
            type_name: "Std::Numeric::Decimal::DecimalVectorizeHint".into(),
            variant: "None".into(),
            discriminant: 0,
        })),
    });
    let display = format!("{rvalue:?}");
    assert!(
        display.contains("kind: Add"),
        "unexpected debug output: {display}"
    );
}

#[test]
fn lower_decimal_add_intrinsic() {
    let mut type_layouts = TypeLayoutTable::default();
    let mut interner = StringInterner::new();
    let symbol_index = SymbolIndex::default();
    let operator_registry = OperatorRegistry::default();
    let trait_registry: HashMap<String, TraitLoweringInfo> = HashMap::new();
    let static_registry = StaticRegistry::new();
    let class_bases: HashMap<String, Vec<String>> = HashMap::new();
    let class_virtual_slots: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let import_resolver = ImportResolver::default();
    let default_arguments = default_argument_store();
    let type_visibilities: HashMap<String, TypeDeclInfo> = HashMap::new();
    let generic_specializations = empty_specializations();
    let primitive_registry = PrimitiveRegistry::with_builtins();
    let function_packages: HashMap<String, String> = HashMap::new();

    let signature = FnSig {
        params: vec![Ty::named("decimal"), Ty::named("decimal")],
        ret: Ty::named(DECIMAL_INTRINSIC_RESULT_TY),
        abi: Abi::Chic,
        effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };

    let mut builder = BodyBuilder::new(
        &signature,
        None,
        "Sample::Compute",
        false,
        false,
        Vec::new(),
        &mut type_layouts,
        &type_visibilities,
        &primitive_registry,
        default_arguments.clone(),
        None,
        None,
        &function_packages,
        &operator_registry,
        &mut interner,
        &symbol_index,
        &import_resolver,
        &static_registry,
        &class_bases,
        &class_virtual_slots,
        &trait_registry,
        FunctionKind::Function,
        false,
        crate::threading::thread_runtime_mode(),
        None,
        None,
        generic_specializations.clone(),
    );

    let lowered_operand = builder
        .test_lower_decimal_intrinsic(
            "Std::Numeric::Decimal::Intrinsics::Add",
            vec![
                Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
                Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
            ],
            Some(Place::new(LocalId(0))),
        )
        .expect("lowering should succeed");
    match lowered_operand {
        Operand::Copy(place) => {
            assert_eq!(place.local, LocalId(0));
            assert!(place.projection.is_empty());
        }
        other => panic!(
            "decimal intrinsic should return destination operand, found {:?}",
            other
        ),
    }

    let (body, diagnostics, _, _) = builder.finish();
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let intrinsic = collect_decimal_intrinsic(&body);
    assert_eq!(intrinsic.kind, DecimalIntrinsicKind::Add);
    assert!(intrinsic.addend.is_none());
    match &intrinsic.rounding {
        Operand::Const(constant) => {
            if let ConstValue::Enum {
                variant,
                discriminant,
                ..
            } = &constant.value
            {
                assert_eq!(variant, "TiesToEven");
                assert_eq!(*discriminant, 0);
            } else {
                panic!(
                    "expected constant rounding operand, found {:?}",
                    constant.value
                );
            }
        }
        other => panic!("expected constant rounding operand, found {other:?}"),
    }
    match &intrinsic.vectorize {
        Operand::Const(constant) => {
            if let ConstValue::Enum {
                variant,
                discriminant,
                ..
            } = &constant.value
            {
                assert_eq!(variant, "None");
                assert_eq!(*discriminant, 0);
            } else {
                panic!(
                    "expected constant vectorize operand, found {:?}",
                    constant.value
                );
            }
        }
        other => panic!("expected constant vectorize operand, found {other:?}"),
    }
}

#[test]
fn lower_decimal_add_with_options_uses_dynamic_operands() {
    let mut type_layouts = TypeLayoutTable::default();
    let mut interner = StringInterner::new();
    let symbol_index = SymbolIndex::default();
    let operator_registry = OperatorRegistry::default();
    let trait_registry: HashMap<String, TraitLoweringInfo> = HashMap::new();
    let static_registry = StaticRegistry::new();
    let class_bases: HashMap<String, Vec<String>> = HashMap::new();
    let class_virtual_slots: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let import_resolver = ImportResolver::default();
    let default_arguments = default_argument_store();
    let type_visibilities: HashMap<String, TypeDeclInfo> = HashMap::new();
    let generic_specializations = empty_specializations();
    let primitive_registry = PrimitiveRegistry::with_builtins();
    let function_packages: HashMap<String, String> = HashMap::new();

    let signature = FnSig {
        params: vec![Ty::named("decimal"), Ty::named("decimal")],
        ret: Ty::named(DECIMAL_INTRINSIC_RESULT_TY),
        abi: Abi::Chic,
        effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };

    let mut builder = BodyBuilder::new(
        &signature,
        None,
        "Sample::Compute",
        false,
        false,
        Vec::new(),
        &mut type_layouts,
        &type_visibilities,
        &primitive_registry,
        default_arguments.clone(),
        None,
        None,
        &function_packages,
        &operator_registry,
        &mut interner,
        &symbol_index,
        &import_resolver,
        &static_registry,
        &class_bases,
        &class_virtual_slots,
        &trait_registry,
        FunctionKind::Function,
        false,
        crate::threading::thread_runtime_mode(),
        None,
        None,
        generic_specializations.clone(),
    );

    let rounding_operand = Operand::Pending(PendingOperand {
        category: ValueCategory::Pending,
        repr: "rounding".into(),
        span: None,
        info: None,
    });
    let vectorize_operand = Operand::Pending(PendingOperand {
        category: ValueCategory::Pending,
        repr: "vectorize".into(),
        span: None,
        info: None,
    });
    let lowered_operand = builder.test_lower_decimal_intrinsic(
        "Std::Numeric::Decimal::Intrinsics::AddWithOptions",
        vec![
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
            rounding_operand.clone(),
            vectorize_operand.clone(),
        ],
        Some(Place::new(LocalId(0))),
    );
    assert!(lowered_operand.is_some());

    let (body, diagnostics, _, _) = builder.finish();
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let intrinsic = collect_decimal_intrinsic(&body);
    assert_eq!(intrinsic.kind, DecimalIntrinsicKind::Add);
    match &intrinsic.rounding {
        Operand::Pending(pending) => assert_eq!(pending.repr, "rounding"),
        other => panic!("expected dynamic rounding operand, found {other:?}"),
    }
    match &intrinsic.vectorize {
        Operand::Pending(pending) => assert_eq!(pending.repr, "vectorize"),
        other => panic!("expected dynamic vectorize operand, found {other:?}"),
    }
}

#[test]
fn lower_decimal_fma_intrinsic_tracks_addend() {
    let mut type_layouts = TypeLayoutTable::default();
    let mut interner = StringInterner::new();
    let symbol_index = SymbolIndex::default();
    let operator_registry = OperatorRegistry::default();
    let trait_registry: HashMap<String, TraitLoweringInfo> = HashMap::new();
    let static_registry = StaticRegistry::new();
    let class_bases: HashMap<String, Vec<String>> = HashMap::new();
    let class_virtual_slots: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let import_resolver = ImportResolver::default();
    let default_arguments = default_argument_store();
    let type_visibilities: HashMap<String, TypeDeclInfo> = HashMap::new();
    let generic_specializations = empty_specializations();
    let primitive_registry = PrimitiveRegistry::with_builtins();
    let function_packages: HashMap<String, String> = HashMap::new();

    let signature = FnSig {
        params: vec![
            Ty::named("decimal"),
            Ty::named("decimal"),
            Ty::named("decimal"),
        ],
        ret: Ty::named(DECIMAL_INTRINSIC_RESULT_TY),
        abi: Abi::Chic,
        effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };

    let mut builder = BodyBuilder::new(
        &signature,
        None,
        "Sample::Compute",
        false,
        false,
        Vec::new(),
        &mut type_layouts,
        &type_visibilities,
        &primitive_registry,
        default_arguments.clone(),
        None,
        None,
        &function_packages,
        &operator_registry,
        &mut interner,
        &symbol_index,
        &import_resolver,
        &static_registry,
        &class_bases,
        &class_virtual_slots,
        &trait_registry,
        FunctionKind::Function,
        false,
        crate::threading::thread_runtime_mode(),
        None,
        None,
        generic_specializations.clone(),
    );

    let lowered_operand = builder.test_lower_decimal_intrinsic(
        "Std::Numeric::Decimal::Intrinsics::Fma",
        vec![
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
        ],
        Some(Place::new(LocalId(0))),
    );
    assert!(lowered_operand.is_some());

    let (body, diagnostics, _, _) = builder.finish();
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let intrinsic = collect_decimal_intrinsic(&body);
    assert_eq!(intrinsic.kind, DecimalIntrinsicKind::Fma);
    assert!(
        intrinsic.addend.is_some(),
        "fma expected to capture addend operand"
    );
}

#[test]
fn decimal_intrinsic_without_destination_returns_unit() {
    let mut type_layouts = TypeLayoutTable::default();
    let mut interner = StringInterner::new();
    let symbol_index = SymbolIndex::default();
    let operator_registry = OperatorRegistry::default();
    let trait_registry: HashMap<String, TraitLoweringInfo> = HashMap::new();
    let static_registry = StaticRegistry::new();
    let class_bases: HashMap<String, Vec<String>> = HashMap::new();
    let class_virtual_slots: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let import_resolver = ImportResolver::default();
    let default_arguments = default_argument_store();
    let type_visibilities: HashMap<String, TypeDeclInfo> = HashMap::new();
    let generic_specializations = empty_specializations();
    let primitive_registry = PrimitiveRegistry::with_builtins();
    let function_packages: HashMap<String, String> = HashMap::new();

    let signature = FnSig {
        params: vec![Ty::named("decimal"), Ty::named("decimal")],
        ret: Ty::named(DECIMAL_INTRINSIC_RESULT_TY),
        abi: Abi::Chic,
        effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };

    let mut builder = BodyBuilder::new(
        &signature,
        None,
        "Sample::Compute",
        false,
        false,
        Vec::new(),
        &mut type_layouts,
        &type_visibilities,
        &primitive_registry,
        default_arguments.clone(),
        None,
        None,
        &function_packages,
        &operator_registry,
        &mut interner,
        &symbol_index,
        &import_resolver,
        &static_registry,
        &class_bases,
        &class_virtual_slots,
        &trait_registry,
        FunctionKind::Function,
        false,
        crate::threading::thread_runtime_mode(),
        None,
        None,
        generic_specializations.clone(),
    );

    let result = builder.test_lower_decimal_intrinsic(
        "Std::Numeric::Decimal::Intrinsics::Add",
        vec![
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
            Operand::Const(ConstOperand::new(ConstValue::Decimal(Decimal128::zero()))),
        ],
        None,
    );
    match result {
        Some(Operand::Const(constant)) => match constant.value() {
            ConstValue::Unit => {}
            other => panic!("expected unit operand when destination absent, found {other:?}"),
        },
        other => panic!("expected unit operand, found {other:?}"),
    }

    let (body, diagnostics, _, _) = builder.finish();
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    assert!(
        body.blocks.iter().any(|block| block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, StatementKind::Assign { .. }))),
        "intrinsic lowering should emit assign statement even without destination",
    );
}

#[test]
fn vectorize_attribute_without_intrinsics_emits_dm0001() {
    let source = r#"
import Std.Numeric.Decimal;

namespace Sample;

@vectorize(decimal)
public decimal PassThrough(decimal value)
{
    return value;
}
"#;

    let parsed = parse_module(source).require("parse vectorize module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("DM0001")),
        "expected DM0001 diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn decimal_intrinsic_without_vectorize_emits_dm0002() {
    let source = r#"
import Std.Numeric.Decimal;

namespace Sample;

public decimal Sum(decimal lhs, decimal rhs)
{
    var result = Std.Numeric.Decimal.Intrinsics.Add(lhs, rhs);
    if (result.Status != Std.Numeric.Decimal.DecimalStatus.Success)
    {
        return 0m;
    }
    return result.Value;
}
"#;

    let parsed = parse_module(source).require("parse decimal intrinsic module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("DM0002")),
        "expected DM0002 diagnostic, found {:?}",
        lowering.diagnostics
    );
}
