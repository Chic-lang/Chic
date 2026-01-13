//! Local function parsing helpers.

use super::*;

const LOCAL_FUNCTION_MODIFIERS: &[&str] = &[
    "static",
    "noreturn",
    "extern",
    "async",
    "constexpr",
    "virtual",
    "override",
    "sealed",
    "abstract",
    "partial",
    "required",
    "unsafe",
    "readonly",
];

fn is_local_function_modifier(token: &Token) -> bool {
    LOCAL_FUNCTION_MODIFIERS
        .iter()
        .any(|modifier| token.lexeme.eq_ignore_ascii_case(modifier))
}

parser_impl! {
    pub(super) fn peek_local_function_declaration(&self) -> bool {
        let mut offset = 0;
        loop {
            // LL1_ALLOW: Local functions permit modifier prefixes, so we scan ahead until we find the `function` keyword to keep the entry grammar LL(1) (docs/compiler/parser.md#ll1-allowances).
            let Some(token) = self.peek_n(offset) else {
                return false;
            };

            if matches!(token.kind, TokenKind::Keyword(Keyword::Function)) {
                return true;
            }

            if is_local_function_modifier(token) {
                offset += 1;
                continue;
            }

            return false;
        }
    }

    pub(super) fn parse_local_function_statement(
        &mut self,
        start_pos: Option<usize>,
        mut attrs: CollectedAttributes,
    ) -> Option<Statement> {
        let mut modifiers = self.consume_modifiers();
        if !self.match_keyword(Keyword::Function) {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected `function` keyword", span);
            return None;
        }

        let is_async = Self::take_modifier(&mut modifiers, "async").is_some();
        let is_constexpr = Self::take_modifier(&mut modifiers, "constexpr").is_some();
        let is_unsafe = Self::take_modifier(&mut modifiers, "unsafe").is_some();

        let function_attrs = attrs.take_function_attributes();
        let surface_attributes = attrs.take_list();
        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "attributes are not supported on local function declarations",
            );
        }

        let mut function =
            self.parse_function(Visibility::Private, is_async, is_constexpr, None)?;
        function.modifiers = modifiers
            .into_iter()
            .map(|modifier| modifier.name)
            .collect();
        function.is_unsafe = is_unsafe;
        function.attributes = surface_attributes;
        self.apply_function_attributes(&mut function, false, function_attrs);

        let span = self.make_span(start_pos);
        if function.body.is_none() {
            self.push_error("local functions must provide a body", span);
        }
        Some(Statement::new(
            span,
            StatementKind::LocalFunction(function),
        ))
    }
}
