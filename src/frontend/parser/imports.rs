use super::*;

parser_impl! {
    pub(super) fn parse_import(
        &mut self,
        doc: Option<DocComment>,
        is_global: bool,
        start_index: usize,
    ) -> Option<Item> {
        if self.check_keyword(Keyword::Static) {
            self.advance();
            return self.parse_import_static(doc, is_global, start_index);
        }

        if self.import_alias_ahead() {
            return self.parse_import_alias(doc, is_global, start_index);
        }

        self.parse_import_namespace(doc, is_global, start_index)
    }

    fn import_alias_ahead(&self) -> bool {
        // LL1_ALLOW: Import aliases peek for `=` to differentiate `import Alias = Path;` from namespace imports without sacrificing LL(1) parsing elsewhere (docs/compiler/parser.md#ll1-allowances).
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Identifier,
                ..
            })
        ) && {
            matches!(
                // LL1_ALLOW: Import aliases peek for `=` to differentiate `import Alias = Path;` from namespace imports without sacrificing LL(1) parsing elsewhere (docs/compiler/parser.md#ll1-allowances).
                self.peek_n(1),
                Some(Token {
                    kind: TokenKind::Operator(op),
                    ..
                }) if *op == "="
            )
        }
    }

    fn parse_import_static(
        &mut self,
        doc: Option<DocComment>,
        is_global: bool,
        start_index: usize,
    ) -> Option<Item> {
        let target = self.parse_qualified_name("expected type after 'import static'")?;
        if !self.expect_punctuation(';') {
            return None;
        }

        let span = self.span_from_indices(start_index, self.index);
        Some(Item::Import(ImportDirective {
            doc,
            is_global,
            span,
            kind: ImportKind::Static { target },
        }))
    }

    fn parse_import_alias(
        &mut self,
        doc: Option<DocComment>,
        is_global: bool,
        start_index: usize,
    ) -> Option<Item> {
        let alias = self.consume_identifier("expected import alias name")?;
        let alias_span = self.last_span;

        if !self.consume_operator("=") {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected '=' after import alias", span);
            return None;
        }

        let target = self.parse_qualified_name("expected namespace or type after '='")?;
        if alias.eq_ignore_ascii_case("std") && !target.eq_ignore_ascii_case("std") {
            let mut diagnostic = Diagnostic::error(
                "`Std` is implicitly imported and cannot be aliased to a different namespace",
                alias_span,
            )
            .with_code(DiagnosticCode::new(
                "IMPORT0002",
                Some("import".to_string()),
            ));
            diagnostic.add_note(
                "remove the alias or choose a different name to avoid shadowing the implicit \
                 standard prelude",
            );
            self.diagnostics.push(diagnostic);
        }
        if !self.expect_punctuation(';') {
            return None;
        }

        self.check_import_cycle(&alias, &target, alias_span);
        self.import_aliases.insert(alias.clone(), target.clone());

        let span = self.span_from_indices(start_index, self.index);
        Some(Item::Import(ImportDirective {
            doc,
            is_global,
            span,
            kind: ImportKind::Alias { alias, target },
        }))
    }

    fn parse_import_namespace(
        &mut self,
        doc: Option<DocComment>,
        is_global: bool,
        start_index: usize,
    ) -> Option<Item> {
        let path = self.parse_qualified_name("expected namespace or type after import")?;
        if !self.expect_punctuation(';') {
            return None;
        }

        let span = self.span_from_indices(start_index, self.index);
        Some(Item::Import(ImportDirective {
            doc,
            is_global,
            span,
            kind: ImportKind::Namespace { path },
        }))
    }
}
