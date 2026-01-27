use chic::diagnostics::{ColorMode, ErrorFormat, FileCache, FormatOptions, format_diagnostics};
use chic::frontend::parser::parse_module_in_file;

#[test]
fn parser_errors_render_with_file_locations() {
    let source = "namespace Sample { fn main() { let =; } }";
    let mut files = FileCache::default();
    let file_id = files.add_file("module.ch", source);
    let err = parse_module_in_file(source, file_id).expect_err("expected parse failure");
    let rendered = format_diagnostics(
        err.diagnostics(),
        &files,
        FormatOptions {
            format: ErrorFormat::Short,
            color: ColorMode::Never,
            is_terminal: false,
        },
    );
    assert!(
        rendered.contains("module.ch"),
        "formatted diagnostics should include the file path: {rendered}"
    );
    assert!(
        rendered.contains("error["),
        "formatted diagnostics should include severity and code: {rendered}"
    );
}
