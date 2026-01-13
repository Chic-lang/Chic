use serde::Serialize;

use super::{Diagnostic, DiagnosticCode, FileCache, LineCol, Severity, Span};
use crate::unicode::grapheme;

pub const JSON_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorFormat {
    Human,
    Json,
    Toon,
    Short,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatOptions {
    pub format: ErrorFormat,
    pub color: ColorMode,
    pub is_terminal: bool,
}

impl FormatOptions {
    #[must_use]
    pub fn use_color(self) -> bool {
        match self.color {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => self.is_terminal,
        }
    }
}

/// Render a collection of diagnostics to a single string.
#[must_use]
pub fn format_diagnostics(
    diagnostics: &[Diagnostic],
    files: &FileCache,
    options: FormatOptions,
) -> String {
    let mut rendered = String::new();
    let use_color = options.use_color();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            rendered.push('\n');
        }
        let chunk = match options.format {
            ErrorFormat::Human => render_human(diagnostic, files, use_color),
            ErrorFormat::Toon => render_toon(diagnostic, files, use_color),
            ErrorFormat::Short => render_short(diagnostic, files),
            ErrorFormat::Json => render_json(diagnostic, files),
        };
        rendered.push_str(&chunk);
    }
    rendered
}

fn render_human(diagnostic: &Diagnostic, files: &FileCache, color: bool) -> String {
    let mut out = String::new();
    let (path, location) = locate_primary(diagnostic, files);
    let header = format_header(diagnostic, color);
    out.push_str(&header);
    out.push('\n');
    out.push_str(&format_location_arrow(&path, location.as_ref()));
    if let Some(label) = diagnostic.primary_label.as_ref() {
        out.push_str(&render_snippet(
            label.span,
            &label.message,
            diagnostic.severity,
            files,
            color,
        ));
    }
    for label in &diagnostic.secondary_labels {
        out.push_str(&render_snippet(
            label.span,
            &label.message,
            diagnostic.severity,
            files,
            color,
        ));
    }
    for note in &diagnostic.notes {
        out.push_str(&format!("\nnote: {note}"));
    }
    for suggestion in &diagnostic.suggestions {
        let mut line = format!("\nhelp: {}", suggestion.message);
        if let Some(span) = suggestion.span {
            if let Some(loc) = files
                .line_col(span.file_id, span.start)
                .or_else(|| files.line_col(span.file_id, span.end))
            {
                if let Some(path) = files.path(span.file_id) {
                    line.push_str(&format!(
                        " @ {}:{}:{}",
                        path.display(),
                        loc.line,
                        loc.column
                    ));
                }
            }
        }
        if let Some(replacement) = &suggestion.replacement {
            line.push_str(&format!(" replace with `{replacement}`"));
        }
        out.push_str(&line);
    }
    out
}

fn render_toon(diagnostic: &Diagnostic, files: &FileCache, color: bool) -> String {
    // Toon format is a playful variant of the human renderer.
    let mut out = String::new();
    let (path, location) = locate_primary(diagnostic, files);
    let header = format!(
        "{} {}",
        match diagnostic.severity {
            Severity::Error => "💥",
            Severity::Warning => "⚠️ ",
            Severity::Note => "📝",
            Severity::Help => "💡",
        },
        format_header(diagnostic, color)
    );
    out.push_str(&header);
    out.push('\n');
    out.push_str(&format_location_arrow(&path, location.as_ref()));
    if let Some(label) = diagnostic.primary_label.as_ref() {
        out.push_str(&render_snippet(
            label.span,
            &label.message,
            diagnostic.severity,
            files,
            color,
        ));
    }
    for label in &diagnostic.secondary_labels {
        out.push_str(&render_snippet(
            label.span,
            &label.message,
            diagnostic.severity,
            files,
            color,
        ));
    }
    if !diagnostic.notes.is_empty() {
        out.push_str("\ncurator says:");
        for note in &diagnostic.notes {
            out.push_str(&format!("\n  - {note}"));
        }
    }
    for suggestion in &diagnostic.suggestions {
        out.push_str(&format!("\n✨ try: {}", suggestion.message));
        if let Some(replacement) = &suggestion.replacement {
            out.push_str(&format!(" → `{replacement}`"));
        }
    }
    out
}

fn render_short(diagnostic: &Diagnostic, files: &FileCache) -> String {
    let (path, location) = locate_primary(diagnostic, files);
    let severity = diagnostic.severity.as_str();
    let code = diagnostic
        .code
        .as_ref()
        .map(|c| c.code.as_str())
        .unwrap_or("UNKNOWN");
    let (line, column) = location
        .map(|loc| (loc.line.to_string(), loc.column.to_string()))
        .unwrap_or_else(|| ("?".into(), "?".into()));
    let mut out = format!(
        "{}:{}:{}: {}[{code}]: {}",
        path, line, column, severity, diagnostic.message
    );
    if !diagnostic.notes.is_empty() {
        out.push_str(&format!(" (notes: {})", diagnostic.notes.len()));
    }
    for suggestion in &diagnostic.suggestions {
        out.push_str(&format!("; suggestion: {}", suggestion.message));
    }
    out
}

fn render_json(diagnostic: &Diagnostic, files: &FileCache) -> String {
    let primary_span = diagnostic
        .primary_label
        .as_ref()
        .and_then(|label| JsonSpan::from_span(label.span, files));
    let mut labels = Vec::new();
    if let Some(span) = diagnostic.primary_label.as_ref() {
        labels.push(JsonLabel::from_label(span, files));
    }
    for label in &diagnostic.secondary_labels {
        labels.push(JsonLabel::from_label(label, files));
    }
    let suggestions: Vec<JsonSuggestion> = diagnostic
        .suggestions
        .iter()
        .map(|s| JsonSuggestion::from_suggestion(s, files))
        .collect();

    let payload = JsonDiagnostic {
        version: JSON_SCHEMA_VERSION.to_string(),
        severity: diagnostic.severity.as_str().to_string(),
        code: diagnostic.code.clone(),
        message: diagnostic.message.clone(),
        primary_span,
        labels,
        notes: diagnostic.notes.clone(),
        suggestions,
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
}

fn format_header(diagnostic: &Diagnostic, color: bool) -> String {
    let severity = diagnostic.severity.as_str();
    let code = diagnostic
        .code
        .as_ref()
        .map(|c| c.code.as_str())
        .unwrap_or("UNKNOWN");
    let prefix = if color {
        colorize(severity, severity_color(diagnostic.severity))
    } else {
        severity.to_string()
    };
    format!("{prefix}[{code}]: {}", diagnostic.message)
}

fn format_location_arrow(path: &str, loc: Option<&LineCol>) -> String {
    match loc {
        Some(loc) => format!("  --> {path}:{}:{}\n   |\n", loc.line, loc.column),
        None => format!("  --> {path}:?:?\n   |\n"),
    }
}

fn render_snippet(
    span: Span,
    message: &str,
    severity: Severity,
    files: &FileCache,
    color: bool,
) -> String {
    let mut out = String::new();
    let Some(file) = files.get(span.file_id) else {
        return out;
    };
    let Some(loc) = file.line_col(span.start) else {
        return out;
    };
    if let Some(line) = file.line(loc.line) {
        let (line_start, line_end) = file
            .line_bounds(loc.line)
            .unwrap_or((span.start.saturating_sub(loc.column), span.end));
        let display_line = line.trim_end_matches('\n');
        let rel_start = span
            .start
            .saturating_sub(line_start)
            .min(display_line.len());
        let rel_end = span
            .end
            .min(line_end)
            .saturating_sub(line_start)
            .min(display_line.len());
        let column = grapheme::grapheme_column(display_line, rel_start);
        let caret_count = grapheme::grapheme_span_len(display_line, rel_start, rel_end);
        let grapheme_loc = LineCol {
            line: loc.line,
            column,
        };
        out.push_str(&format!("{:>4} | {}\n", loc.line, display_line));
        let caret_line = format!(
            "{:>4} | {}{} {}",
            "",
            " ".repeat(grapheme_loc.column.saturating_sub(1)),
            "^".repeat(caret_count),
            message
        );
        if color {
            out.push_str(&format!(
                "{}\n",
                caret_line.replace('^', &colorize("^", severity_color(severity)))
            ));
        } else {
            out.push('\n');
            out.push_str(&caret_line);
        }
    }
    out
}

fn locate_primary<'a>(
    diagnostic: &'a Diagnostic,
    files: &'a FileCache,
) -> (String, Option<LineCol>) {
    if let Some(label) = diagnostic.primary_label.as_ref() {
        if let Some(file) = files.get(label.span.file_id) {
            if let Some(path) = files.path(label.span.file_id) {
                let loc = file.line_col(label.span.start);
                let grapheme_loc = loc.and_then(|lc| {
                    let (line_start, _) = file
                        .line_bounds(lc.line)
                        .unwrap_or((label.span.start, label.span.end));
                    let line = file.line(lc.line)?;
                    let display_line = line.trim_end_matches('\n');
                    let rel_start = label
                        .span
                        .start
                        .saturating_sub(line_start)
                        .min(display_line.len());
                    Some(LineCol {
                        line: lc.line,
                        column: grapheme::grapheme_column(display_line, rel_start),
                    })
                });
                return (path.display().to_string(), grapheme_loc);
            }
        }
    }
    ("<unknown>".into(), None)
}

fn colorize(value: &str, code: &str) -> String {
    format!("\u{1b}[{code}m{value}\u{1b}[0m")
}

fn severity_color(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "1;31",
        Severity::Warning => "1;33",
        Severity::Note => "1;34",
        Severity::Help => "1;32",
    }
}

#[derive(Serialize)]
struct JsonDiagnostic {
    version: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<DiagnosticCode>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary_span: Option<JsonSpan>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    labels: Vec<JsonLabel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    notes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    suggestions: Vec<JsonSuggestion>,
}

#[derive(Serialize)]
struct JsonSpan {
    file: String,
    start: usize,
    end: usize,
    line_start: usize,
    column_start: usize,
}

impl JsonSpan {
    fn from_span(span: Span, files: &FileCache) -> Option<Self> {
        let file = files.get(span.file_id)?;
        let line_col = file.line_col(span.start)?;
        Some(Self {
            file: file.path.display().to_string(),
            start: span.start,
            end: span.end,
            line_start: line_col.line,
            column_start: line_col.column,
        })
    }
}

#[derive(Serialize)]
struct JsonLabel {
    message: String,
    span: JsonSpan,
    is_primary: bool,
}

impl JsonLabel {
    fn from_label(label: &super::Label, files: &FileCache) -> JsonLabel {
        JsonLabel {
            message: label.message.clone(),
            span: JsonSpan::from_span(label.span, files).unwrap_or(JsonSpan {
                file: "<unknown>".into(),
                start: label.span.start,
                end: label.span.end,
                line_start: 0,
                column_start: 0,
            }),
            is_primary: label.is_primary,
        }
    }
}

#[derive(Serialize)]
struct JsonSuggestion {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<JsonSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<String>,
}

impl JsonSuggestion {
    fn from_suggestion(suggestion: &super::Suggestion, files: &FileCache) -> Self {
        Self {
            message: suggestion.message.clone(),
            span: suggestion
                .span
                .and_then(|span| JsonSpan::from_span(span, files)),
            replacement: suggestion.replacement.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticCode, Suggestion};
    use serde_json::Value;

    fn sample_file() -> (FileCache, Span) {
        let mut files = FileCache::default();
        let source = "fn main() {\n    let value = bad;\n}\n";
        let file_id = files.add_file("sample.cl", source);
        let start = source.find("bad").expect("sample contains bad identifier");
        let span = Span::in_file(file_id, start, start + 3);
        (files, span)
    }

    fn base_diagnostic(span: Span) -> Diagnostic {
        let mut diagnostic =
            Diagnostic::error("unknown name `bad`", Some(span)).with_primary_label("not found");
        diagnostic.code = Some(DiagnosticCode::new("PARSE0001", Some("parse".into())));
        diagnostic
    }

    fn options(format: ErrorFormat) -> FormatOptions {
        FormatOptions {
            format,
            color: ColorMode::Never,
            is_terminal: false,
        }
    }

    #[test]
    fn human_format_includes_snippet_and_metadata() {
        let (files, span) = sample_file();
        let mut diagnostic = base_diagnostic(span);
        diagnostic.add_note("stage: parse");
        diagnostic.add_suggestion(Suggestion::new(
            "replace with `good`",
            Some(span),
            Some("good".into()),
        ));
        let loc = files.line_col(span.file_id, span.start).expect("line/col");
        let file = files.get(span.file_id).expect("file present");
        let (line_start, _) = file.line_bounds(loc.line).expect("line bounds");
        let line = file
            .line(loc.line)
            .expect("source line")
            .trim_end_matches('\n');
        let column = grapheme::grapheme_column(line, span.start - line_start);

        let rendered = format_diagnostics(&[diagnostic], &files, options(ErrorFormat::Human));
        assert!(
            rendered.contains("error[PARSE0001]: unknown name `bad`"),
            "header should contain severity and code: {rendered}"
        );
        assert!(
            rendered.contains(&format!("--> sample.cl:{}:{}", loc.line, column)),
            "location arrow should include path and line/col: {rendered}"
        );
        assert!(
            rendered.contains("not found"),
            "primary label message should be rendered: {rendered}"
        );
        assert!(
            rendered.contains("note: stage: parse"),
            "notes should render after snippets: {rendered}"
        );
        assert!(
            rendered.contains("help: replace with `good`"),
            "suggestions should be formatted with replacement text: {rendered}"
        );
    }

    #[test]
    fn toon_format_uses_playful_header() {
        let (files, span) = sample_file();
        let diagnostic = base_diagnostic(span);
        let rendered = format_diagnostics(&[diagnostic], &files, options(ErrorFormat::Toon));
        assert!(
            rendered.contains("💥 error[PARSE0001]:"),
            "toon format should prefix emoji: {rendered}"
        );
    }

    #[test]
    fn short_format_is_single_line() {
        let (files, span) = sample_file();
        let mut diagnostic = base_diagnostic(span);
        diagnostic.add_note("stage: parse");
        let loc = files.line_col(span.file_id, span.start).expect("line/col");
        let file = files.get(span.file_id).expect("file present");
        let (line_start, _) = file.line_bounds(loc.line).expect("line bounds");
        let line = file
            .line(loc.line)
            .expect("source line")
            .trim_end_matches('\n');
        let column = grapheme::grapheme_column(line, span.start - line_start);
        let rendered = format_diagnostics(&[diagnostic], &files, options(ErrorFormat::Short));
        let expected_prefix = format!("sample.cl:{}:{}: error[PARSE0001]:", loc.line, column);
        assert!(
            rendered.starts_with(&expected_prefix),
            "short format should start with path/line/col: {rendered}"
        );
        assert!(
            rendered.contains("(notes: 1)"),
            "short format should include note count when present: {rendered}"
        );
    }

    #[test]
    fn json_format_emits_schema_versioned_payload() {
        let (files, span) = sample_file();
        let mut diagnostic = base_diagnostic(span);
        diagnostic.add_suggestion(Suggestion::new(
            "rename to good",
            Some(span),
            Some("good".into()),
        ));
        let rendered = format_diagnostics(&[diagnostic], &files, options(ErrorFormat::Json));
        let value: Value = serde_json::from_str(&rendered).expect("valid json diagnostic");
        assert_eq!(value["version"], JSON_SCHEMA_VERSION, "schema version");
        assert_eq!(value["severity"], "error", "severity field");
        assert_eq!(value["code"]["code"], "PARSE0001", "diagnostic code");
        assert_eq!(value["code"]["category"], "parse", "diagnostic category");
        assert!(
            value["primary_span"].is_object(),
            "primary span should be included: {value}"
        );
        assert!(
            value["labels"].is_array() && !value["labels"].as_array().unwrap().is_empty(),
            "labels array should be present: {value}"
        );
        assert!(
            value["suggestions"]
                .as_array()
                .is_some_and(|list| !list.is_empty()),
            "suggestions should serialize with spans: {value}"
        );
    }

    #[test]
    fn grapheme_columns_align_caret_ranges() {
        let mut files = FileCache::default();
        let source = "let flag = 🇺🇳;\n";
        let file_id = files.add_file("unicode.cl", source);
        let start = source.find('🇺').expect("flag present");
        let span = Span::in_file(file_id, start, start + "🇺🇳".len());
        let diagnostic =
            Diagnostic::error("invalid flag literal", Some(span)).with_primary_label("invalid");
        let rendered = format_diagnostics(&[diagnostic], &files, options(ErrorFormat::Human));
        let caret_line = rendered
            .lines()
            .find(|line| line.contains('^'))
            .expect("caret line rendered");
        let caret_count = caret_line.chars().filter(|ch| *ch == '^').count();
        assert_eq!(
            caret_count, 1,
            "grapheme cluster should be underlined once: {rendered}"
        );
    }
}
