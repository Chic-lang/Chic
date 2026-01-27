use std::io::{self, Write};

use crate::diagnostics::{
    ColorMode, Diagnostic, DiagnosticCode, ErrorFormat, FormatOptions, Severity, Suggestion,
    format_diagnostics,
};
use crate::driver::{FrontendReport, RunResult};
use crate::error::Error;

pub(super) fn report_error(err: &Error) {
    let mut out = io::stderr();
    if let Err(io_err) = report_error_to(err, &mut out) {
        let _ = writeln!(io::stderr(), "failed to report error: {io_err}");
    }
}

pub(super) fn print_report_diagnostics(report: &FrontendReport, options: FormatOptions) {
    let mut out: Box<dyn Write> = match options.format {
        ErrorFormat::Json => Box::new(io::stdout()),
        _ if report.has_errors() => Box::new(io::stderr()),
        _ => Box::new(io::stdout()),
    };
    if let Err(err) = print_report_diagnostics_to(report, options, &mut out) {
        let _ = writeln!(io::stderr(), "failed to write diagnostics: {err}");
    }
}

pub(super) fn print_report_diagnostics_to(
    report: &FrontendReport,
    options: FormatOptions,
    out: &mut dyn Write,
) -> io::Result<()> {
    let diagnostics = collect_diagnostics(report);
    if diagnostics.is_empty() {
        return Ok(());
    }
    if !matches!(options.format, crate::diagnostics::ErrorFormat::Json) {
        if !report.format_diagnostics.is_empty() {
            writeln!(out, "format diagnostics:")?;
        }
        if !report.type_diagnostics.is_empty() {
            writeln!(out, "type diagnostics:")?;
        }
        if !report.mir_lowering_diagnostics.is_empty() || !report.mir_verification.is_empty() {
            writeln!(out, "mir lowering diagnostics:")?;
        }
        if !report.reachability_diagnostics.is_empty() {
            writeln!(out, "reachability diagnostics:")?;
        }
        if !report.borrow_diagnostics.is_empty() {
            writeln!(out, "borrow checker diagnostics:")?;
        }
        if !report.doc_diagnostics.is_empty() {
            writeln!(out, "documentation diagnostics:")?;
        }
    }
    let rendered = format_diagnostics(&diagnostics, &report.files, options);
    writeln!(out, "{rendered}")?;
    Ok(())
}

pub(super) fn report_error_to(err: &Error, out: &mut dyn Write) -> io::Result<()> {
    match err {
        Error::Parse(parse_err) => {
            writeln!(out, "error: {parse_err}")?;
            let rendered = format_diagnostics(
                parse_err.diagnostics(),
                parse_err.files(),
                FormatOptions {
                    format: ErrorFormat::Human,
                    color: ColorMode::Never,
                    is_terminal: false,
                },
            );
            writeln!(out, "{rendered}")?;
        }
        _ => {
            writeln!(out, "{err}")?;
            if cfg!(debug_assertions) {
                if let Some(backtrace) = err.backtrace() {
                    writeln!(out, "stack trace:")?;
                    writeln!(out, "{backtrace}")?;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn relay_run_output(result: &RunResult) -> io::Result<()> {
    let mut out = io::stdout();
    let mut err = io::stderr();
    relay_run_output_to(result, &mut out, &mut err)
}

pub(super) fn relay_run_output_to(
    result: &RunResult,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> io::Result<()> {
    stdout.write_all(&result.stdout)?;
    stderr.write_all(&result.stderr)
}

fn collect_diagnostics(report: &FrontendReport) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for diagnostic in &report.format_diagnostics {
        diagnostics.push(tag_stage(diagnostic.clone(), "format"));
    }
    for module in &report.modules {
        for diagnostic in &module.parse.diagnostics {
            diagnostics.push(tag_stage(diagnostic.clone(), "parse"));
        }
    }
    for diagnostic in &report.type_diagnostics {
        diagnostics.push(tag_stage(diagnostic.clone(), "typeck"));
    }
    for diagnostic in &report.reachability_diagnostics {
        diagnostics.push(tag_stage(diagnostic.clone(), "reachability"));
    }
    for diagnostic in &report.borrow_diagnostics {
        diagnostics.push(tag_stage(diagnostic.clone(), "borrowck"));
    }
    for diagnostic in &report.fallible_diagnostics {
        diagnostics.push(tag_stage(diagnostic.clone(), "fallibility"));
    }
    for (index, diagnostic) in report.mir_lowering_diagnostics.iter().enumerate() {
        let (is_warning, message) = diagnostic
            .message
            .strip_prefix("warning:")
            .map(|message| (true, message.trim_start()))
            .unwrap_or((false, diagnostic.message.as_str()));
        let mut diag = if is_warning {
            Diagnostic::warning(message, diagnostic.span)
        } else {
            Diagnostic::error(message, diagnostic.span)
        };
        diag.code = Some(DiagnosticCode::new(
            format!("MIRLOW{:04}", index),
            Some("mir-lowering".into()),
        ));
        diag.notes.push("stage: mir-lowering".into());
        diagnostics.push(diag);
    }
    for (index, issue) in report.mir_verification.iter().enumerate() {
        for error in &issue.errors {
            let mut diag = Diagnostic::error(
                format!("verification failure in {}: {:?}", issue.function, error),
                None,
            );
            diag.code = Some(DiagnosticCode::new(
                format!("MIRVER{:04}", index),
                Some("mir-verify".into()),
            ));
            diag.notes
                .push(format!("stage: mir-verify for {}", issue.function));
            diagnostics.push(diag);
        }
    }
    for lint in &report.lint_diagnostics {
        diagnostics.push(convert_lint(lint, report));
    }
    for diagnostic in &report.doc_diagnostics {
        diagnostics.push(tag_stage(diagnostic.clone(), "docs"));
    }
    diagnostics
}

fn tag_stage(mut diagnostic: Diagnostic, stage: &str) -> Diagnostic {
    diagnostic.notes.insert(0, format!("stage: {stage}"));
    diagnostic
}

fn convert_lint(lint: &crate::lint::LintDiagnostic, report: &FrontendReport) -> Diagnostic {
    let severity = if lint.level.is_error() {
        Severity::Error
    } else {
        Severity::Warning
    };
    let file_id = report
        .files
        .find_id_by_path(&lint.file)
        .unwrap_or(lint.span.map(|sp| sp.file_id).unwrap_or_default());
    let span = lint.span.map(|span| span.with_file(file_id));
    let mut diagnostic = match severity {
        Severity::Error => Diagnostic::error(lint.message.clone(), span),
        _ => Diagnostic::warning(lint.message.clone(), span),
    };
    diagnostic.code = Some(DiagnosticCode::new(
        lint.code().to_string(),
        Some(lint.descriptor.category.as_str().to_string()),
    ));
    diagnostic
        .notes
        .push(format!("stage: lint ({})", lint.descriptor.name));
    diagnostic
        .notes
        .push(lint.descriptor.description.to_string());
    for suggestion in &lint.suggestions {
        let suggestion_span = suggestion.span.map(|span| span.with_file(file_id));
        diagnostic.add_suggestion(Suggestion::new(
            suggestion.message.clone(),
            suggestion_span,
            suggestion.replacement.clone(),
        ));
    }
    diagnostic
}
