use super::*;

parser_impl! {
    pub(in crate::frontend::parser) fn try_qualified_name_from(&self, mut index: usize) -> Option<(String, usize)> {
        let mut parts = Vec::new();
        let first = self.tokens.get(index)?;
        if !matches!(first.kind, TokenKind::Identifier) {
            return None;
        }
        parts.push(first.lexeme.clone());
        index += 1;

        while let Some(dot) = self.tokens.get(index) {
            if dot.kind != TokenKind::Punctuation('.') {
                break;
            }
            let part = self.tokens.get(index + 1)?;
            if !matches!(part.kind, TokenKind::Identifier) {
                return None;
            }
            parts.push(part.lexeme.clone());
            index += 2;
        }

        Some((parts.join("."), index))
    }

    pub(in crate::frontend::parser) fn consume_identifier(&mut self, message: &str) -> Option<String> {
        if let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Identifier
                | TokenKind::Keyword(Keyword::Error | Keyword::Type) => {
                    let lexeme = token.lexeme.clone();
                    self.advance();
                    return Some(lexeme);
                }
                _ => {}
            }
            self.push_error(message, Some(token.span));
            self.advance();
            None
        } else {
            self.push_error(message, None);
            None
        }
    }

    pub(in crate::frontend::parser) fn expect_punctuation(&mut self, expected: char) -> bool {
        match self.peek() {
            Some(token) if token.kind == TokenKind::Punctuation(expected) => {
                self.advance();
                true
            }
            Some(token) => {
                self.push_error(format!("expected '{expected}'"), Some(token.span));
                false
            }
            None => {
                self.push_error(format!("expected '{expected}'"), None);
                false
            }
        }
    }

    pub(in crate::frontend::parser) fn consume_punctuation(&mut self, expected: char) -> bool {
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Punctuation(expected))
        {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(in crate::frontend::parser) fn check_punctuation(&self, expected: char) -> bool {
        self.peek()
            .is_some_and(|token| token.kind == TokenKind::Punctuation(expected))
    }

    pub(in crate::frontend::parser) fn check_keyword(&self, keyword: Keyword) -> bool {
        self.peek()
            .is_some_and(|token| token.kind == TokenKind::Keyword(keyword))
    }

    pub(in crate::frontend::parser) fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Keyword(keyword))
        {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(in crate::frontend::parser) fn peek_keyword_n(&self, offset: usize, keyword: Keyword) -> bool {
        self.peek_n(offset)
            .is_some_and(|token| token.kind == TokenKind::Keyword(keyword))
    }

    pub(in crate::frontend::parser) fn peek_punctuation_n(&self, offset: usize, punctuation: char) -> bool {
        self.peek_n(offset).is_some_and(|token| {
            matches!(token.kind, TokenKind::Punctuation(ch) if ch == punctuation)
        })
    }

    pub(in crate::frontend::parser) fn peek_identifier(&self, expected: &str) -> bool {
        self.peek().is_some_and(|token| {
            matches!(token.kind, TokenKind::Identifier) && token.lexeme == expected
        })
    }

    pub(in crate::frontend::parser) fn peek_n(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.index + offset)
    }

    pub(in crate::frontend::parser) fn consume_modifiers(&mut self) -> Vec<Modifier> {
        const KNOWN_MODIFIERS: &[&str] = &[
            "static", "noreturn", "extern", "async", "constexpr", "virtual", "override", "sealed",
            "abstract", "partial", "required", "unsafe", "readonly", "new",
        ];

        let mut modifiers = Vec::new();
        while let Some(token) = self.peek() {
            if KNOWN_MODIFIERS.contains(&token.lexeme.as_str()) {
                modifiers.push(Modifier {
                    name: token.lexeme.clone(),
                    span: token.span,
                                    });
                self.advance();
            } else {
                break;
            }
        }
        modifiers
    }

    pub(in crate::frontend::parser) fn take_modifier(
        modifiers: &mut Vec<Modifier>,
        name: &str,
    ) -> Option<Modifier> {
        if let Some(pos) = modifiers
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
        {
            Some(modifiers.remove(pos))
        } else {
            None
        }
    }

    pub(in crate::frontend::parser) fn span_from_indices(&self, start: usize, end: usize) -> Option<Span> {
        if start >= end {
            return None;
        }
        let first = self.tokens.get(start)?;
        let last = self.tokens.get(end - 1)?;
        Some(Span::in_file(
            first.span.file_id,
            first.span.start,
            last.span.end,
        ))
    }

    pub(in crate::frontend::parser) fn text_from_span(&self, span: Span) -> String {
        self.source
            .get(span.start..span.end)
            .unwrap_or_default()
            .to_string()
    }

    pub(in crate::frontend::parser) fn make_span(&self, start: Option<usize>) -> Option<Span> {
        match (start, self.last_span) {
            (Some(begin), Some(end_span)) if end_span.end >= begin => {
                Some(Span::in_file(end_span.file_id, begin, end_span.end))
            }
            _ => None,
        }
    }

    pub(in crate::frontend::parser) fn check_operator(&self, symbol: &str) -> bool {
        self.peek()
            .is_some_and(|token| matches!(token.kind, TokenKind::Operator(op) if op == symbol))
    }

    pub(in crate::frontend::parser) fn consume_operator(&mut self, symbol: &str) -> bool {
        if self
            .peek()
            .is_some_and(|token| matches!(token.kind, TokenKind::Operator(op) if op == symbol))
        {
            self.advance();
            return true;
        }
        false
    }

    pub(in crate::frontend::parser) fn push_error(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.diagnostics.push_error(message, span);
    }

    pub(in crate::frontend::parser) fn push_warning(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.diagnostics.push_warning(message, span);
    }

    pub(in crate::frontend::parser) fn finish(self) -> (Vec<Diagnostic>, Option<RecoveryTelemetryData>) {
        (self.diagnostics.into_vec(), self.recovery_telemetry)
    }

    pub(in crate::frontend::parser) fn stash_leading_doc(&mut self) {
        if self.index >= self.leading_docs.len() {
            return;
        }
        if let Some(doc) = self.leading_docs[self.index].take() {
            self.append_doc(doc);
        }
    }

    pub(in crate::frontend::parser) fn append_doc(&mut self, doc: DocComment) {
        if let Some(existing) = &mut self.pending_doc {
            existing.extend(doc);
        } else {
            self.pending_doc = Some(doc);
        }
    }

    pub(in crate::frontend::parser) fn take_pending_doc(&mut self) -> Option<DocComment> {
        self.pending_doc.take()
    }

    pub(in crate::frontend::parser) fn check_import_cycle(&mut self, alias: &str, target: &str, span: Option<Span>) {
        if alias == target {
            self.push_error("import alias cannot reference itself", span);
            return;
        }

        let mut seen = HashSet::new();
        let mut current = target;
        while let Some(next) = self.import_aliases.get(current) {
            if next == alias {
                self.push_error(format!("import alias '{alias}' forms a cycle"), span);
                break;
            }
            if !seen.insert(current.to_string()) {
                break;
            }
            current = next;
        }
    }

    pub(in crate::frontend::parser) fn is_at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }

    pub(in crate::frontend::parser) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    pub(in crate::frontend::parser) fn advance(&mut self) -> Option<Token> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            self.last_span = Some(token.span);
            Some(token)
        } else {
            None
        }
    }
}
