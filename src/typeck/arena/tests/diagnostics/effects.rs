use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::ast::{ClassDecl, ClassKind, Item, TypeExpr, Visibility};
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, ClassLayoutInfo, ClassLayoutKind, StructLayout, TypeLayout,
    TypeLayoutTable, TypeRepr,
};
use crate::typeck::arena::TypeCheckResult;

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::custom(
        "error_class_requires_error_base",
        error_class_requires_error_base,
        Expectation::contains(&[
            "error type `Recovery::TransientError` cannot inherit from non-error type",
        ]),
    ),
    ArenaDiagnosticCase::lowered(
        "missing_throws_clause_reports_effect_error",
        r#"
namespace Effects;

public error IoError { }

public void Raise(IoError err)
{
    throw err;
}
"#,
        Expectation::contains(&["[TCK100]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "declared_throws_clause_covers_effect",
        r#"
namespace Effects;

public error IoError { }

public void Raise(IoError err) throws IoError
{
    throw err;
}
"#,
        Expectation::lacks(&["[TCK100]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "throws_clause_accepts_base_exception",
        r#"
namespace Effects;

public error FormatError { }
public error InvalidFormatError : FormatError { }

public void Parse(InvalidFormatError err) throws FormatError
{
    throw err;
}
"#,
        Expectation::lacks(&["[TCK100]"]),
    ),
];

fn error_class_requires_error_base(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut module = crate::frontend::ast::Module::new(Some("Recovery".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Error,
        name: "TransientError".into(),
        bases: vec![TypeExpr::simple("NotError")],
        members: Vec::new(),
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "NotError".into(),
        bases: Vec::new(),
        members: Vec::new(),
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Recovery::TransientError".into(),
        TypeLayout::Class(StructLayout {
            name: "Recovery::TransientError".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: None,
            align: None,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_yes(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: Some(ClassLayoutInfo {
                kind: ClassLayoutKind::Error,
                bases: vec!["Recovery::NotError".into()],
                vtable_offset: Some(0),
            }),
        }),
    );
    layouts.types.insert(
        "Recovery::NotError".into(),
        TypeLayout::Class(StructLayout {
            name: "Recovery::NotError".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: None,
            align: None,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_yes(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: Some(ClassLayoutInfo {
                kind: ClassLayoutKind::Class,
                bases: Vec::new(),
                vtable_offset: Some(0),
            }),
        }),
    );

    fixture.check_module(&module, &[], &layouts)
}

#[test]
fn effect_diagnostics() {
    run_cases("effects", CASES);
}
