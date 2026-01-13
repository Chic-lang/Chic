use super::common::RequireExt;
use super::*;
use crate::mir::data::StatementKind as MirStatementKind;

fn resolve_interned<'a>(module: &'a MirModule, id: StrId) -> &'a str {
    module
        .interned_strs
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value.as_str())
        .expect("interned string missing")
}

#[test]
fn lowers_interpolated_string_expression() {
    let source = r#"
namespace Sample;

public string Format(int value, string name)
{
    return $"Hello {name}! Answer = {value,4:X}";
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let module = &lowering.module;
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "Sample::Format")
        .expect("function `Sample::Format`");
    let block = func
        .body
        .blocks
        .iter()
        .find(|block| !block.statements.is_empty())
        .expect("entry block");

    let (dest_place, segments) = block
        .statements
        .iter()
        .find_map(|statement| {
            if let MirStatementKind::Assign { place, value } = &statement.kind {
                if let Rvalue::StringInterpolate { segments } = value {
                    return Some((place, segments));
                }
            }
            None
        })
        .expect("interpolated string assignment");

    // Destination local is the temporary created for the interpolated value.
    assert_ne!(dest_place.local.0, 0, "expected expression temp");

    assert_eq!(segments.len(), 4);

    match &segments[0] {
        InterpolatedStringSegment::Text { id } => {
            assert_eq!(resolve_interned(module, *id), "Hello ");
        }
        other => panic!("expected leading text segment, found {other:?}"),
    }

    match &segments[1] {
        InterpolatedStringSegment::Expr {
            operand,
            alignment,
            format,
            ..
        } => {
            assert!(alignment.is_none());
            assert!(format.is_none());
            match operand {
                Operand::Copy(place) => assert_eq!(place.local.0, 2, "expected `name` local"),
                other => panic!("expected copy operand for `name`, found {other:?}"),
            }
        }
        other => panic!("expected expression segment for `name`, found {other:?}"),
    }

    match &segments[2] {
        InterpolatedStringSegment::Text { id } => {
            assert_eq!(resolve_interned(module, *id), "! Answer = ");
        }
        other => panic!("expected middle text segment, found {other:?}"),
    }

    match &segments[3] {
        InterpolatedStringSegment::Expr {
            operand,
            alignment,
            format,
            ..
        } => {
            assert_eq!(alignment, &Some(4));
            let format_id = format.expect("format specifier should be interned");
            assert_eq!(resolve_interned(module, format_id), "X");
            match operand {
                Operand::Copy(place) => assert_eq!(place.local.0, 1, "expected `value` local"),
                other => panic!("expected copy operand for `value`, found {other:?}"),
            }
        }
        other => panic!("expected expression segment for `value`, found {other:?}"),
    }

    // Ensure the temporary result flows into the return slot.
    let return_assign = block
        .statements
        .iter()
        .rev()
        .find(|statement| {
            if let MirStatementKind::Assign { place, value } = &statement.kind {
                matches!(value, Rvalue::Use(Operand::Copy(_))) && place.local.0 == 0
            } else {
                false
            }
        })
        .expect("return assignment");
    if let MirStatementKind::Assign { value, .. } = &return_assign.kind {
        match value {
            Rvalue::Use(Operand::Copy(place)) => {
                assert_eq!(
                    place.local, dest_place.local,
                    "return should copy interpolation result"
                );
            }
            other => panic!("expected copy into return slot, found {other:?}"),
        }
    }
}
