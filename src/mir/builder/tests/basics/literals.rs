use super::helper::*;
use super::*;

fn return_assignment<'a>(func: &'a MirFunction) -> &'a Rvalue {
    func.body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign { place, value } if place.local.0 == 0 => Some(value),
            _ => None,
        })
        .require("missing return assignment")
}

#[test]
fn lowers_bool_literal_into_const_operand() {
    let source = r#"
namespace Literals;

public bool Truth() { return true; }
"#;
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Truth");
    let value = return_assignment(func);
    match value {
        Rvalue::Use(Operand::Const(ConstOperand {
            value: ConstValue::Bool(true),
            ..
        })) => {}
        other => panic!("expected bool const, found {other:?}"),
    }
}

#[test]
fn lowers_char_literal_into_const_operand() {
    let source = r#"
namespace Literals;

public char Greek() { return 'α'; }
"#;
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Greek");
    let value = return_assignment(func);
    match value {
        Rvalue::Use(Operand::Const(ConstOperand {
            value: ConstValue::Char(c),
            ..
        })) => assert_eq!(*c, 'α' as u16),
        other => panic!("expected char const, found {other:?}"),
    }
}
