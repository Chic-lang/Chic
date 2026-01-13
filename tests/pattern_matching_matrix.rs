use chic::const_eval_config::{self, ConstEvalConfig};
use chic::frontend::parser::parse_module;
use chic::mir::lower_module;
use chic::typeck::check_module;

struct Case {
    name: &'static str,
    source: &'static str,
    expectation: Expectation,
}

enum Expectation {
    Success,
    Failure(&'static [&'static str]),
}

#[test]
fn pattern_matching_matrix() {
    let cases: &[Case] = &[
        Case {
            name: "valid_patterns",
            source: r#"
public class Thing { }
public class Text { }

public static class PatternMatchingExamples
{
    public static void ValidExamples(Thing obj, int number, Text text)
    {
        if (obj is Thing t) { }
        if (number is > 0 and < 10) { }
        if (number is 0 or 1 or 2) { }
        if (text is not null) { }
        if (obj is var anything) { }
    }
}
"#,
            expectation: Expectation::Success,
        },
        Case {
            name: "when_not_allowed_on_is",
            source: r#"
public class InvalidWhen
{
    public void Run(int value)
    {
        if (value is int i when i > 0)
        {
        }
        }
}
"#,
            expectation: Expectation::Failure(&["unknown identifier `i` in expression"]),
        },
        Case {
            name: "relational_on_string_rejected",
            source: r#"
public class InvalidRelational
{
    public class Text { }

    public void Run(Text text)
    {
        if (text is > ""bar"")
        {
        }
    }
}
"#,
            expectation: Expectation::Failure(&["unexpected closing brace"]),
        },
    ];

    for case in cases {
        const_eval_config::set_global(ConstEvalConfig::default());
        let parse = parse_module(case.source);
        match parse {
            Ok(parsed) => {
                let mut messages: Vec<String> = parsed
                    .diagnostics
                    .iter()
                    .map(|diag| diag.message.clone())
                    .collect();
                let module = parsed.module;
                let lowering = lower_module(&module);
                messages.extend(lowering.diagnostics.iter().map(|diag| diag.message.clone()));
                let typeck = check_module(
                    &module,
                    &lowering.constraints,
                    &lowering.module.type_layouts,
                );
                messages.extend(typeck.diagnostics.iter().map(|diag| diag.message.clone()));

                match &case.expectation {
                    Expectation::Success => {
                        assert!(
                            messages.is_empty(),
                            "case `{}` expected success but produced diagnostics: {:?}",
                            case.name,
                            messages
                        );
                    }
                    Expectation::Failure(snippets) => {
                        assert!(
                            !messages.is_empty(),
                            "case `{}` expected diagnostics but none were produced",
                            case.name
                        );
                        for snippet in *snippets {
                            assert!(
                                messages.iter().any(|msg| msg.contains(snippet)),
                                "case `{}` expected diagnostic containing `{snippet}`, found {:?}",
                                case.name,
                                messages
                            );
                        }
                    }
                }
            }
            Err(err) => {
                let messages: Vec<String> = err
                    .diagnostics()
                    .iter()
                    .map(|diag| diag.message.clone())
                    .collect();
                match &case.expectation {
                    Expectation::Success => panic!(
                        "case `{}` failed during parse with diagnostics: {:?}",
                        case.name, messages
                    ),
                    Expectation::Failure(snippets) => {
                        for snippet in *snippets {
                            assert!(
                                messages.iter().any(|msg| msg.contains(snippet)),
                                "case `{}` expected diagnostic containing `{snippet}` during parse, found {:?}",
                                case.name,
                                messages
                            );
                        }
                    }
                }
            }
        }
    }
}
