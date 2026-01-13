use super::common::{RequireExt, assert_no_pending};
use super::*;

#[test]
fn array_literal_allocates_and_sets_length() {
    let source = r#"
namespace Demo;

public static class Harness
{
    public static int[] Make()
    {
        return [1, 2, 3];
    }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let make_fn = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Harness::Make"))
        .require("missing Make lowering");
    assert_no_pending(&make_fn.body);

    let mut saw_alloc = false;
    let mut saw_zero_init = false;
    let mut saw_len_set = false;

    for block in &make_fn.body.blocks {
        if let Some(Terminator::Call { func, .. }) = &block.terminator {
            if let Operand::Const(ConstOperand {
                value: ConstValue::Symbol(sym),
                ..
            }) = func
            {
                if sym == "chic_rt_vec_with_capacity" {
                    saw_alloc = true;
                }
            }
        }
        for stmt in &block.statements {
            match &stmt.kind {
                StatementKind::ZeroInitRaw { .. } => {
                    saw_zero_init = true;
                }
                StatementKind::Assign { place, value } => {
                    let writes_len = place.projection.iter().any(
                        |proj| matches!(proj, ProjectionElem::FieldNamed(name) if name == "len"),
                    );
                    if writes_len {
                        if let Rvalue::Use(Operand::Const(ConstOperand {
                            value: ConstValue::UInt(val),
                            ..
                        })) = value
                        {
                            if *val == 3 {
                                saw_len_set = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    assert!(saw_alloc, "array literal should allocate backing storage");
    assert!(
        saw_zero_init,
        "array literal should zero-initialize backing data"
    );
    assert!(
        saw_len_set,
        "array literal should set length to element count"
    );
}
