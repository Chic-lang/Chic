use super::super::common::FunctionFixture;
use crate::frontend::ast::{GotoTarget, StatementKind, SwitchLabel};
use crate::frontend::parser::tests::fixtures::parse_fail;

#[test]
fn parses_switch_with_multiple_sections() {
    let source = r"
public int Categorize(int value)
{
    switch (value)
    {
        case 0:
            return 0;
        case 1:
        case 2:
            return 1;
        default:
            return -1;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Switch(switch) => {
            assert_eq!(switch.sections.len(), 3, "expected three switch sections");
            assert_eq!(switch.sections[0].labels.len(), 1);
            assert_eq!(switch.sections[1].labels.len(), 2);
            assert!(matches!(switch.sections[2].labels[0], SwitchLabel::Default));
        }
        other => panic!("expected switch statement, found {other:?}"),
    }
}

#[test]
fn parses_switch_with_pattern_cases() {
    let source = r"
public int Match(object value)
{
    switch (value)
    {
        case int number when number > 0:
            return number;
        case string text:
            return text.Length;
        default:
            return -1;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Switch(switch) => {
            assert_eq!(switch.sections.len(), 3);
            assert!(matches!(switch.sections[2].labels[0], SwitchLabel::Default));

            let mut string_case = None;
            let mut guarded_case = None;
            for section in &switch.sections {
                for label in &section.labels {
                    if let SwitchLabel::Case(case_label) = label {
                        match case_label.pattern.raw.text.trim() {
                            "string text" => string_case = Some(case_label),
                            "int number" => guarded_case = Some(case_label),
                            _ => {}
                        }
                    }
                }
            }

            let string_case = string_case.expect("expected string case");
            assert!(
                string_case.guards.is_empty(),
                "string case should not have guard"
            );

            let guarded_case = guarded_case.expect("expected guarded case");
            let guard = guarded_case
                .guards
                .first()
                .expect("expected guard expression");
            assert_eq!(guard.expression.text.trim(), "number > 0");
        }
        other => panic!("expected switch statement, found {other:?}"),
    }
}

#[test]
fn switch_guard_missing_expression_reports_error() {
    let source = r"
public int Map(int value)
{
    switch (value)
    {
        case int number when :
            return number;
        default:
            return 0;
    }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`when` guard requires an expression")),
        "expected guard diagnostic, found {:#?}",
        diagnostics
    );
}

#[test]
fn parses_multiple_case_guards_in_order() {
    let source = r"
public int Map(int value)
{
    switch (value)
    {
        case var x when x > 0 when x < 10:
            return x;
        default:
            return value;
    }
}
";
    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let switch_stmt = match &fixture.statements()[0].kind {
        StatementKind::Switch(switch) => switch,
        other => panic!("expected switch, found {other:?}"),
    };
    if let SwitchLabel::Case(case_label) = &switch_stmt.sections[0].labels[0] {
        assert_eq!(case_label.guards.len(), 2);
        assert_eq!(case_label.guards[0].expression.text.trim(), "x > 0");
        assert_eq!(case_label.guards[1].expression.text.trim(), "x < 10");
        assert_eq!(case_label.guards[0].depth, 0);
        assert_eq!(case_label.guards[1].depth, 1);
    } else {
        panic!("expected case label");
    }
}

#[test]
fn parses_switch_with_complex_patterns() {
    let source = r"
public int Match(object value)
{
    switch (value)
    {
        case Point { X: > 0, Y: > 0 }:
            return 1;
        case Point { X: < 0, Y: < 0 }:
            return -1;
        default:
            return 0;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Switch(switch) => assert_eq!(switch.sections.len(), 3),
        other => panic!("expected switch statement, found {other:?}"),
    }
}

#[test]
fn parses_goto_case_with_when_guard() {
    let source = r#"public void Jump(int value)
{
    switch (value)
    {
        case 1:
            goto case 2 when (value > 2);
        case 2:
            break;
    }
}
"#;

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let switch_stmt = match &fixture.statements()[0].kind {
        StatementKind::Switch(switch) => switch,
        other => panic!("expected switch statement, found {other:?}"),
    };
    let first_section = &switch_stmt.sections[0];
    let goto_stmt = match &first_section.statements[0].kind {
        StatementKind::Goto(stmt) => stmt,
        other => panic!("expected goto statement, found {other:?}"),
    };
    let (pattern, guards) = match &goto_stmt.target {
        GotoTarget::Case { pattern, guards } => (pattern, guards),
        other => panic!("expected goto case target, found {other:?}"),
    };
    assert_eq!(pattern.raw.text.trim(), "2");
    let guard_expr = guards
        .first()
        .expect("expected goto case guard")
        .expression
        .text
        .trim()
        .to_string();
    assert_eq!(guard_expr, "(value > 2)");
}
