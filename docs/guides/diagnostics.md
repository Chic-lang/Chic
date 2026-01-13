# Diagnostics authoring guide

- **Codes:** Use stable, namespaced codes (`PARSE`, `TYPE`, `LINT`, `MIRL`, `MIRV`, `WASM`, `RUNTIME`, etc.). Prefer `DiagnosticSink::new("PARSE")`/`DiagnosticCode::new("TYPE0007", Some("typeck"))` instead of hard-coding strings inside call sites.
- **Messages:** Short, actionable, and neutral. Prefer present-tense imperatives (“expected expression after `=`”, “annotate the type of `value`”) and avoid blaming the user. Keep the primary message focused on the root cause; relegate context to labels/notes.
- **Labels:** Always attach a primary label to the root span. Secondary labels highlight related sites (previous definition, conflicting borrow, etc.). Leave primary label text empty only when the header already states the failure crisply; otherwise, add a brief label (“defined here”, “first borrow occurs here”).
- **Suggestions:** Emit `help:` entries for mechanical fixes. Provide a replacement string when the fix is unambiguous, and attach a span so editors can surface code actions. Avoid suggestions for semantic errors where multiple fixes are viable.
- **Notes:** Use notes for stage breadcrumbs (`stage: parse`, `stage: typeck`, `stage: lint`) and clarifications that don’t belong in the primary message. Keep them terse.
- **Formatting modes:** The CLI/LSP respect `--error-format human|json|toon|short` (TTY defaults to `human`, non-TTY defaults to `short`, `NO_COLOR` disables ANSI). JSON output follows schema version `1.0.0` with stable field names (`severity`, `code`, `category`, `labels`, `suggestions`, `notes`).
- **Tests:** Add regression tests that render diagnostics via `format_diagnostics` with a `FileCache` populated for the spans in question. Cover at least one human/short/JSON example when introducing new diagnostics so formatting remains stable for CLI and machine consumers.

Example (parser error):

```rust
let mut sink = DiagnosticSink::new("PARSE");
sink.push(
    Diagnostic::error("unknown identifier `bad`", Some(span))
        .with_primary_label("not found"),
);
let rendered = format_diagnostics(
    &sink.into_vec(),
    &files,
    FormatOptions { format: ErrorFormat::Human, color: ColorMode::Never, is_terminal: false },
);
```

Sample outputs for the snippet above:
- `human` (TTY-coloured when allowed):
  - `error[PARSE00001]: unknown identifier 'bad'`  
    `  --> sample.cl:2:17`  
    `   |`  
    ` 2 |     let value = bad;`  
    `   |                 ^^^ not found`
- `short`: `sample.cl:2:17: error[PARSE00001]: unknown identifier 'bad'`
- `json`: Single-line payload with `version`, `severity`, `code`, `message`, `primary_span`, `labels`, `notes`, and `suggestions` fields (schema 1.0.0).
