use crate::frontend::diagnostics::Diagnostic;
use crate::frontend::parser::attributes::CollectedAttributes;
use crate::frontend::parser::tests::fixtures::lex_tokens;
use crate::frontend::parser::{ParseResult, Parser, parse_module};

pub(super) fn parse_with_diagnostics(source: &str) -> (Option<ParseResult>, Vec<Diagnostic>) {
    match parse_module(source) {
        Ok(result) => {
            let diagnostics = result.diagnostics.clone();
            (Some(result), diagnostics)
        }
        Err(err) => (None, err.diagnostics().to_vec()),
    }
}

pub(super) fn messages<'a>(diagnostics: &'a [Diagnostic]) -> impl Iterator<Item = &'a str> {
    diagnostics.iter().map(|diag| diag.message.as_str())
}

pub(super) fn collect_attributes_from_source(
    source: &str,
) -> (CollectedAttributes, Vec<Diagnostic>) {
    let lex = lex_tokens(source);
    let mut parser = Parser::new(source, lex);
    let attrs = parser.collect_attributes();
    let (diagnostics, _) = parser.finish();
    (attrs, diagnostics)
}

pub(super) fn layout_fixture(args: &str) -> String {
    format!(
        "@StructLayout({args})
public struct LayoutTarget {{ public int Value; }}
"
    )
}

pub(super) fn codegen_fixture(spec: &str) -> String {
    format!(
        "@{spec}
public extern void Native();
"
    )
}

pub(super) fn diagnostic_fixture(lines: &[&str]) -> String {
    format!("{}\npublic class Service {{}}\n", lines.join("\n"))
}

pub(super) fn parser_fixture(source: &str) -> Parser<'_> {
    let lex = lex_tokens(source);
    Parser::new(source, lex)
}
