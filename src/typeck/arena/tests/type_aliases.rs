#![cfg(test)]

use super::fixtures::parse_and_check;
use crate::const_eval_config::{self, ConstEvalConfig};
use crate::frontend::parser::parse_module;
use crate::mir::lower_module;
use crate::mir::{Ty, TypeLayout, TypeLayoutTable};
use crate::typeck::diagnostics::codes;

#[test]
fn type_alias_resolves_in_fields_and_signatures() {
    let source = r#"
typealias AudioSample = ushort;
typealias Boxed<T> = Holder<T>;

public struct Holder<T> { public T Value; }

public struct Mixer
{
    public AudioSample Sample;
    public Boxed<int> Buffered;
}

public AudioSample Convert(AudioSample input) { return input; }
"#;

    let (_, result) = parse_and_check(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn type_alias_cycle_reports_error() {
    let source = r#"
typealias A = B;
typealias B = A;
"#;

    let (_, result) = parse_and_check(source);
    assert!(
        result.diagnostics.iter().any(|diag| diag
            .code
            .as_ref()
            .is_some_and(|code| code.code == codes::TYPE_ALIAS_CYCLE)),
        "expected cycle diagnostic, found {:?}",
        result.diagnostics
    );
}

#[test]
fn type_alias_rejects_const_generics() {
    let source = r#"
typealias Sized<const N: int> = int;
"#;

    let (_, result) = parse_and_check(source);
    assert!(
        result.diagnostics.iter().any(|diag| diag
            .code
            .as_ref()
            .is_some_and(|code| code.code == codes::TYPE_ALIAS_CONST_PARAM)),
        "expected const generic diagnostic, found {:?}",
        result.diagnostics
    );
}

#[test]
fn type_alias_preserves_layout_and_abi() {
    const_eval_config::set_global(ConstEvalConfig::default());
    let source = r#"
typealias AudioSample = ushort;

public struct WithAlias { public AudioSample Value; }
public struct WithPrimitive { public ushort Value; }
"#;

    let parsed = parse_module(source).expect("parse module");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.module;
    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );
    let layouts: &TypeLayoutTable = &lowering.module.type_layouts;
    let alias_layout = layouts
        .layout_for_name("WithAlias")
        .or_else(|| layouts.layout_for_name("::WithAlias"))
        .expect("WithAlias layout");
    let prim_layout = layouts
        .layout_for_name("WithPrimitive")
        .or_else(|| layouts.layout_for_name("::WithPrimitive"))
        .expect("WithPrimitive layout");

    let alias_size_align = match alias_layout {
        TypeLayout::Struct(layout) | TypeLayout::Class(layout) => {
            (layout.size.expect("size"), layout.align.expect("align"))
        }
        other => panic!("unexpected layout for WithAlias: {other:?}"),
    };
    let prim_size_align = match prim_layout {
        TypeLayout::Struct(layout) | TypeLayout::Class(layout) => {
            (layout.size.expect("size"), layout.align.expect("align"))
        }
        other => panic!("unexpected layout for WithPrimitive: {other:?}"),
    };
    assert_eq!(
        alias_size_align, prim_size_align,
        "alias should not change ABI"
    );

    if let TypeLayout::Struct(layout) | TypeLayout::Class(layout) = alias_layout {
        let field_ty = &layout
            .fields
            .first()
            .expect("alias struct should have a field")
            .ty;
        assert_eq!(
            field_ty.canonical_name(),
            Ty::named("ushort").canonical_name(),
            "alias should expand to the canonical underlying type"
        );
    }

    assert!(
        layouts.layout_for_name("AudioSample").is_none(),
        "type aliases should not create new runtime types"
    );
}
