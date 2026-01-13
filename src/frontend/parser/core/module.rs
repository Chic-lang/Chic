use super::*;

parser_impl! {
    pub(in crate::frontend::parser) fn new(source: &'a str, output: LexOutput) -> Self {
        let LexOutput {
            tokens: output_tokens,
            diagnostics: lex_diagnostics,
            file_id,
        } = output;
        let mut tokens = Vec::new();
        let mut leading_docs = Vec::new();
        let mut pending_lines: Vec<String> = Vec::new();
        let mut last_doc_span: Option<Span> = None;

        for token in output_tokens {
            match token.kind {
                TokenKind::Whitespace | TokenKind::Comment => {}
                TokenKind::DocComment => {
                    pending_lines.push(normalise_doc_line(&token.lexeme));
                    last_doc_span = Some(token.span);
                }
                _ => {
                    if pending_lines.is_empty() {
                        leading_docs.push(None);
                    } else {
                        let doc = DocComment::new(take(&mut pending_lines));
                        leading_docs.push(Some(doc));
                    }
                    tokens.push(token);
                }
            }
        }

        let mut diagnostics = DiagnosticSink::new("PARSE");
        for diag in lex_diagnostics {
            diagnostics.push(diag);
        }
        if !pending_lines.is_empty() {
            diagnostics.push_warning(
                "documentation comment has no following declaration",
                last_doc_span,
            );
        }

        Self {
            source,
            file_id,
            tokens,
            leading_docs,
            pending_doc: None,
            import_aliases: HashMap::new(),
            index: 0,
            last_span: None,
            namespace_stack: Vec::new(),
            file_namespace: Vec::new(),
            namespace_span: None,
            module_import_block_closed: false,
            diagnostics,
            recovery_telemetry: if recovery_telemetry_enabled() {
                Some(RecoveryTelemetryData::default())
            } else {
                None
            },
        }
    }

    pub(in crate::frontend::parser) fn parse_module(&mut self) -> Module {
        let crate_attributes = self.collect_crate_attributes();
        let leading_imports = self.collect_leading_imports();

        let mut module = Module::new(None);
        module.namespace_span = self.namespace_span;
        module.crate_attributes = crate_attributes;
        self.set_file_namespace(module.namespace.as_ref());
        for item in leading_imports {
            module.push_item(item);
        }

        while !self.is_at_end() {
            self.parse_module_item(&mut module);
        }

        module.rebuild_overloads();
        module
    }

    pub(in crate::frontend::parser) fn collect_crate_attributes(&mut self) -> CrateAttributes {
        let mut attrs = CrateAttributes::default();
        loop {
            self.stash_leading_doc();
            let Some(token) = self.peek().cloned() else {
                break;
            };
            if token.kind != TokenKind::Punctuation('#') {
                break;
            }

            // LL1_ALLOW: crate-level attributes need two-token lookahead for `#![` disambiguation (parser LL(1) guardrail).
            let bang = self.peek_n(1).cloned();
            let has_bang = matches!(bang, Some(Token { kind: TokenKind::Operator("!"), .. }));
            // LL1_ALLOW: confirm `#![` header with a second lookahead before parsing crate attribute payload.
            if !has_bang || !self.peek_punctuation_n(2, '[') {
                self.push_error(
                    "expected `#![` to start a crate-level attribute",
                    Some(token.span),
                );
                self.advance();
                continue;
            }

            let attr_start = token.span.start;
            self.advance(); // #
            self.advance(); // !
            self.advance(); // [

            let (name, _name_span) = match self.parse_attribute_name() {
                Some(pair) => pair,
                None => {
                    self.skip_balanced('[', ']');
                    continue;
                }
            };

            let mut has_arguments = false;
            if self.consume_punctuation('(') {
                has_arguments = true;
                self.skip_balanced('(', ')');
            }

            if !self.expect_punctuation(']') {
                continue;
            }

            let span = self
                .make_span(Some(attr_start))
                .or_else(|| self.peek().map(|t| t.span));
            if has_arguments {
                self.push_error(
                    "crate-level attributes do not accept arguments",
                    span,
                );
            }
            self.record_crate_attribute(&mut attrs, &name, span);
        }

        attrs
    }

    pub(in crate::frontend::parser) fn consume_misplaced_crate_attribute(&mut self) -> bool {
        let bang_is_op = {
            // LL1_ALLOW: crate attributes require two-token lookahead to confirm `#![` (parser LL(1) guardrail).
            self.peek_n(1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Operator("!")))
        };
        // LL1_ALLOW: confirm `[` after `#!` before consuming misplaced crate attribute.
        let has_bracket = self.peek_punctuation_n(2, '[');
        if !self.check_punctuation('#') || !bang_is_op || !has_bracket {
            return false;
        }

        let span = self.peek().map(|token| token.span);
        self.push_error(
            "crate-level attributes must appear before any items or import directives",
            span,
        );
        self.advance();
        self.advance();
        self.advance();
        self.skip_balanced('[', ']');
        true
    }

    fn record_crate_attribute(
        &mut self,
        attrs: &mut CrateAttributes,
        name: &str,
        span: Option<Span>,
    ) {
        let lowered = name.to_ascii_lowercase();
        match lowered.as_str() {
            "no_std" | "nostd" => match attrs.std_setting {
                CrateStdSetting::Unspecified => {
                    attrs.std_setting = CrateStdSetting::NoStd { span };
                }
                CrateStdSetting::Std { span: existing } => {
                    self.push_error(
                        "conflicting crate attributes: crate already marked `#![std]`",
                        span.or(existing),
                    );
                }
                CrateStdSetting::NoStd { .. } => {
                    self.push_error("duplicate `#![no_std]` crate attribute", span);
                }
            },
            "std" => match attrs.std_setting {
                CrateStdSetting::Unspecified => {
                    attrs.std_setting = CrateStdSetting::Std { span };
                }
                CrateStdSetting::Std { .. } => {
                    self.push_error("duplicate `#![std]` crate attribute", span);
                }
                CrateStdSetting::NoStd { span: existing } => {
                    self.push_error(
                        "conflicting crate attributes: crate already marked `#![no_std]`",
                        span.or(existing),
                    );
                }
            },
            "no_main" => match attrs.main_setting {
                crate::frontend::ast::CrateMainSetting::Unspecified => {
                    attrs.main_setting = crate::frontend::ast::CrateMainSetting::NoMain { span };
                }
                crate::frontend::ast::CrateMainSetting::NoMain { span: existing } => {
                    self.push_error("duplicate `#![no_main]` crate attribute", span.or(existing));
                }
            },
            other => {
                self.push_error(
                    format!("unsupported crate-level attribute `#![{other}]`"),
                    span,
                );
            }
        }
    }

    pub(in crate::frontend::parser) fn collect_leading_imports(&mut self) -> Vec<Item> {
        let mut leading_imports = Vec::new();

        loop {
            self.stash_leading_doc();
            if !self.import_directive_ahead() {
                break;
            }

            let mut attrs = self.collect_attributes();
            for (header, span) in attrs.take_c_imports() {
                if header.is_empty() {
                    self.push_error("`@cimport` attribute requires a header name", span);
                } else {
                    leading_imports.push(Item::Import(ImportDirective {
                        doc: None,
                        is_global: false,
                        span,
                        kind: ImportKind::CImport { header },
                    }));
                }
            }
            self.stash_leading_doc();

            let directive_start = self.index;
            let (is_global, keyword_span, using_keyword) = self.consume_import_keyword();
            let doc = self.take_pending_doc();
            if !attrs.is_empty() {
                self.report_attribute_misuse(
                    attrs,
                    "attributes are not supported on import directives",
                );
            }

            if using_keyword {
                self.emit_using_import_error(keyword_span);
            }

            match self.parse_import(doc, is_global, directive_start) {
                Some(item) => leading_imports.push(item),
                None => self.synchronize_item(),
            }
        }

        leading_imports
    }

    pub(in crate::frontend::parser) fn consume_import_keyword(
        &mut self,
    ) -> (bool, Option<Span>, bool) {
        let mut using_keyword = false;
        let mut keyword_span = None;
        let is_global = if self.match_keyword(Keyword::Global) {
            let global_span = self.last_span;
            if self.match_keyword(Keyword::Import) {
                keyword_span = self.last_span;
                true
            } else if self.match_keyword(Keyword::Using) {
                keyword_span = self.last_span;
                using_keyword = true;
                true
            } else {
                self.push_error(
                    "`global` keyword may only prefix an import directive",
                    global_span.or(self.last_span),
                );
                true
            }
        } else if self.match_keyword(Keyword::Import) {
            keyword_span = self.last_span;
            false
        } else {
            using_keyword = self.match_keyword(Keyword::Using);
            if using_keyword {
                keyword_span = self.last_span;
            }
            false
        };
        (is_global, keyword_span, using_keyword)
    }

    pub(in crate::frontend::parser) fn emit_using_import_error(
        &mut self,
        keyword_span: Option<Span>,
    ) {
        let mut diagnostic =
            Diagnostic::error("`using` directives are not supported; use `import`", keyword_span)
                .with_code(DiagnosticCode::new(
                    "IMPORT0001",
                    Some("import".to_string()),
                ));
        if let Some(span) = keyword_span {
            diagnostic = diagnostic.with_primary_label("replace `using` with `import`");
            diagnostic.add_suggestion(Suggestion::new(
                "replace `using` with `import`",
                Some(span),
                Some("import".to_string()),
            ));
        } else {
            diagnostic.add_suggestion(Suggestion::new(
                "replace `using` with `import`",
                None,
                Some("import".to_string()),
            ));
        }
        diagnostic.add_note("resource-management `using` statements are unchanged");
        self.diagnostics.push(diagnostic);
    }

    pub(in crate::frontend::parser) fn parse_block(&mut self) -> Option<Block> {
        let Some(open) = self.peek().cloned() else {
            self.push_error("expected '{' to start block", None);
            return None;
        };

        if open.kind != TokenKind::Punctuation('{') {
            self.push_error("expected '{' to start block", Some(open.span));
            return None;
        }

        let start_offset = open.span.start;
        self.advance();

        let mut statements = Vec::new();
        while !self.check_punctuation('}') && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else {
                self.synchronize_statement();
                if self.check_punctuation('}') {
                    break;
                }
            }
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        let end = self.last_span.map_or(open.span.end, |span| span.end);
        let span = Some(Span::in_file(self.file_id, start_offset, end));
        Some(Block { statements, span })
    }

    pub(in crate::frontend::parser) fn consume_file_scoped_namespace(&mut self) -> Option<(String, Span)> {
        if !self.check_keyword(Keyword::Namespace) {
            return None;
        }

        let start_index = self.index;
        let saved_span = self.last_span;

        // Consume 'namespace'
        self.advance();

        let Some((name, next_index)) = self.try_qualified_name_from(self.index) else {
            let _ = self.parse_qualified_name("expected namespace identifier");
            let _ = self.expect_punctuation(';');
            return None;
        };

        self.index = next_index;

        match self.peek() {
            Some(token) if token.kind == TokenKind::Punctuation(';') => {
                self.advance();
                let span = self
                    .span_from_indices(start_index, self.index)
                    .or(self.last_span)?;
                self.namespace_span = Some(span);
                Some((name, span))
            }
            Some(token) if token.kind == TokenKind::Punctuation('{') => {
                self.index = start_index;
                self.last_span = saved_span;
                None
            }
            Some(token) => {
                self.push_error(
                    "expected ';' for a file-scoped namespace or '{' to open a namespace block",
                    Some(token.span),
                );
                None
            }
            None => {
                self.push_error(
                    "expected ';' for a file-scoped namespace or '{' to open a namespace block",
                    None,
                );
                None
            }
        }
    }

    pub(in crate::frontend::parser) fn set_file_namespace<S: AsRef<str>>(&mut self, namespace: Option<S>) {
        self.file_namespace = namespace
            .map(|ns| {
                ns.as_ref()
                    .split('.')
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    }
}
