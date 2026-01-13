use super::*;

pub(super) fn parse_module_allowing_errors(source: &str) -> (Module, Vec<Diagnostic>) {
    let lex_output = lex_tokens(source);
    let mut parser = Parser::new(source, lex_output);
    let module = parser.parse_module();
    let (diagnostics, _) = parser.finish();
    (module, diagnostics)
}
