use super::*;
use crate::frontend::diagnostics::{Diagnostic, DiagnosticCode};

parser_impl! {
    pub(super) fn parse_namespace(
        &mut self,
        visibility: Visibility,
        modifiers: &[Modifier],
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
        start_index: usize,
    ) -> Option<Item> {
        self.warn_namespace_visibility(visibility);
        self.warn_namespace_modifiers(modifiers);

        let name = self.parse_qualified_name("expected namespace identifier")?;
        let full_name = self.compose_namespace_name(&name);
        let components: Vec<String> = name.split('.').map(str::to_owned).collect();

        let attributes = attrs.take_list();
        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "unsupported attributes applied to namespace declarations",
            );
        }

        if self.consume_punctuation(';') {
            let span = self
                .span_from_indices(start_index, self.index)
                .or(self.last_span);
            self.report_file_scoped_namespace_error(span);
            return Some(Item::Namespace(NamespaceDecl {
                name: full_name,
                items: Vec::new(),
                doc,
                attributes: attributes.clone(),
                span,
            }));
        }

        if !self.expect_punctuation('{') {
            return None;
        }

        let items = self.parse_namespace_block(&components)?;

        let span = self
            .span_from_indices(start_index, self.index)
            .or(self.last_span);

        Some(Item::Namespace(NamespaceDecl {
            name: full_name,
            items,
            doc,
            attributes,
            span,
        }))
    }

    fn warn_namespace_visibility(&mut self, visibility: Visibility) {
        if matches!(visibility, Visibility::Internal) {
            return;
        }
        let span = self.last_span;
        self.push_warning("namespace declarations ignore visibility modifiers", span);
    }

    fn warn_namespace_modifiers(&mut self, modifiers: &[Modifier]) {
        if modifiers.is_empty() {
            return;
        }
        let span = self.last_span;
        self.push_warning(
            format!(
                "namespace declarations do not support modifiers: {}",
                modifiers
                    .iter()
                    .map(|modifier| modifier.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            span,
                    );
    }

    fn compose_namespace_name(&self, name: &str) -> String {
        let mut segments = Vec::new();
        segments.extend(self.file_namespace.iter().cloned());
        segments.extend(self.namespace_stack.iter().cloned());
        segments.extend(
            name.split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_owned),
        );
        segments.join(".")
    }

    fn report_file_scoped_namespace_error(&mut self, span: Option<Span>) {
        self.push_error(
            "file-scoped namespace can only appear at the compilation unit root",
            span,
                    );
    }

    fn parse_namespace_block(&mut self, components: &[String]) -> Option<Vec<Item>> {
        let previous_depth = self.namespace_stack.len();
        self.namespace_stack.extend(components.iter().cloned());

        let items = self.collect_namespace_items();

        if !self.expect_punctuation('}') {
            self.namespace_stack.truncate(previous_depth);
            return None;
        }

        self.namespace_stack.truncate(previous_depth);
        Some(items)
    }

    pub(super) fn collect_namespace_items(&mut self) -> Vec<Item> {
        let mut items = Vec::new();
        let mut import_block_closed = false;
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            let attrs = self.collect_attributes();
            self.stash_leading_doc();
            if self.is_at_end() || self.check_punctuation('}') {
                if !attrs.is_empty() {
                    self.report_attribute_misuse(attrs, "dangling attribute at end of namespace");
                }
                break;
            }
            if import_block_closed && self.import_directive_ahead() {
                let span = self.peek().map(|token| token.span);
                self.push_error(
                    "import directives must appear before other declarations within a namespace",
                    span,
                );
            }
            match self.parse_item(attrs) {
                Some(Item::Import(import)) if import.is_global => {
                    let span = import.span.or_else(|| self.last_span);
                    self.diagnostics.push(
                        Diagnostic::error(
                            "global import directives are not allowed inside namespaces or types",
                            span,
                        )
                        .with_code(DiagnosticCode::new("E0G02", Some("import".to_string()))),
                    );
                    import_block_closed = true;
                }
                Some(item) => {
                    import_block_closed |= !matches!(item, Item::Import(_));
                    items.push(item);
                }
                None => {
                    import_block_closed = true;
                    self.synchronize_item();
                }
            }
        }
        items
    }
}
