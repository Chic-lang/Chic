use super::*;

parser_impl! {
    pub(super) fn parse_qualified_name(&mut self, message: &str) -> Option<String> {
        let mut parts = Vec::new();
        let first = self.consume_identifier(message)?;
        parts.push(first);

        while self.consume_punctuation('.') {
            let part = self.consume_identifier("expected identifier after '.'")?;
            parts.push(part);
        }

        Some(parts.join("."))
    }
}
