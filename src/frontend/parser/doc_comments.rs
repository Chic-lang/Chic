pub(super) fn normalise_doc_line(lexeme: &str) -> String {
    if let Some(stripped) = lexeme.strip_prefix("///") {
        let trimmed = stripped.strip_prefix(' ').unwrap_or(stripped);
        trimmed.trim_end().to_string()
    } else {
        lexeme.trim().to_string()
    }
}
