use super::members::DispatchModifiers;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PropertyContext {
    Class,
    Struct,
    Interface,
}

impl PropertyContext {
    fn allows_bodies(self) -> bool {
        matches!(self, PropertyContext::Class | PropertyContext::Struct)
    }
}

parser_impl! {
    pub(super) fn parse_indexer_parameters(&mut self) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        if !self.expect_punctuation('[') {
            return parameters;
        }
        if self.check_punctuation(']') {
            self.push_error(
                "indexer must declare at least one parameter",
                self.peek().map(|token| token.span),
            );
            self.advance();
            return parameters;
        }

        loop {
            self.stash_leading_doc();
            let mut attrs = self.collect_attributes();
            self.stash_leading_doc();
            if !attrs.is_empty() && !attrs.builtin.is_empty() {
                self.report_attribute_misuse(
                    attrs.clone(),
                    "unsupported built-in attribute on parameters",
                );
            }
            let attributes = attrs.take_list();
            let (binding, binding_nullable) = self.parse_binding_modifier();

            let Some(ty) = self.parse_type_expr() else {
                self.synchronize_parameter();
                break;
            };

            if let Some(token) = self.peek() {
                match &token.kind {
                    TokenKind::Punctuation('?') => {
                        self.push_error(
                            "type annotation accepts at most one `?`",
                            Some(token.span),
                        );
                        self.advance();
                    }
                    TokenKind::Operator(op) if *op == "??" => {
                        self.push_error(
                            "type annotation accepts at most one `?`",
                            Some(token.span),
                        );
                        self.advance();
                    }
                    _ => {}
                }
            }

            let name_span = self.peek().map(|token| token.span);
            let Some(name) = self.consume_identifier("expected parameter name") else {
                self.synchronize_parameter();
                break;
            };
            let lends = self.parse_lends_clause();
            let default = if self.consume_operator("=") {
                let expr = self.collect_expression_until(&[',', ']']);
                if expr.span.is_none() && expr.text.trim().is_empty() {
                    let span = self.peek().map(|token| token.span);
                    self.push_error("expected default expression after '='", span);
                    None
                } else {
                    Some(expr)
                }
            } else {
                None
            };
            let default_span = default.as_ref().and_then(|expr| expr.span);

            parameters.push(Parameter {
                binding,
                binding_nullable,
                name,
                name_span,
                ty,
                attributes,
                di_inject: None,
                default,
                default_span,
                lends,
                is_extension_this: false,
            });

            if self.check_punctuation(']') {
                break;
            }
            if self.consume_punctuation(',') {
                continue;
            }
            let span = self.peek().map(|token| token.span);
            self.push_error("expected ',' or ']' after parameter", span);
            self.synchronize_parameter();
            break;
        }

        let _ = self.consume_punctuation(']');
        parameters
    }
}

parser_impl! {
    pub(super) fn parse_class_property(
        &mut self,
        visibility: Visibility,
        modifiers: Vec<Modifier>,
        name: String,
        name_token_index: usize,
        ty: TypeExpr,
        parameters: Vec<Parameter>,
        doc: Option<DocComment>,
        is_async: bool,
        is_required: bool,
        required_span: Option<Span>,
        dispatch_markers: DispatchModifiers,
        class_is_static: bool,
        explicit_interface: Option<String>,
        is_indexer: bool,
    ) -> Option<PropertyDecl> {
        self.parse_property(
            PropertyContext::Class,
            visibility,
            modifiers,
            name,
            name_token_index,
            ty,
            parameters,
            doc,
            is_async,
            is_required,
            required_span,
            dispatch_markers,
            class_is_static,
            explicit_interface,
            is_indexer,
        )
    }

    pub(super) fn parse_interface_property(
        &mut self,
        visibility: Visibility,
        modifiers: Vec<Modifier>,
        name: String,
        name_token_index: usize,
        ty: TypeExpr,
        parameters: Vec<Parameter>,
        doc: Option<DocComment>,
        is_async: bool,
        is_required: bool,
        required_span: Option<Span>,
        explicit_interface: Option<String>,
        is_indexer: bool,
    ) -> Option<PropertyDecl> {
        self.parse_property(
            PropertyContext::Interface,
            visibility,
            modifiers,
            name,
            name_token_index,
            ty,
            parameters,
            doc,
            is_async,
            is_required,
            required_span,
            DispatchModifiers::default(),
            false,
            explicit_interface,
            is_indexer,
        )
    }

    pub(super) fn parse_property(
        &mut self,
        context: PropertyContext,
        visibility: Visibility,
        modifiers: Vec<Modifier>,
        name: String,
        name_token_index: usize,
        ty: TypeExpr,
        parameters: Vec<Parameter>,
        doc: Option<DocComment>,
        is_async: bool,
        is_required: bool,
        required_span: Option<Span>,
        dispatch_markers: DispatchModifiers,
        class_is_static: bool,
        explicit_interface: Option<String>,
        is_indexer: bool,
    ) -> Option<PropertyDecl> {
        let name_span = self
            .tokens
            .get(name_token_index)
            .map(|token| token.span);
        if ty.name.eq_ignore_ascii_case("void") {
            let message = if is_indexer {
                "'void' is not a valid indexer return type"
            } else {
                "'void' is not a valid property type"
            };
            self.push_error(message, name_span);
        }
        let is_indexer = is_indexer || !parameters.is_empty();
        if is_indexer && parameters.is_empty() {
            self.push_error("indexer must declare at least one parameter", name_span);
        }
        if is_async {
            self.push_error("properties cannot be marked `async`", name_span);
        }

        let normalized_modifiers = self.normalise_property_modifiers(context, modifiers);
        let is_static = normalized_modifiers
            .iter()
            .any(|modifier| modifier == "static");
        if is_indexer && is_static {
            self.push_error("indexer cannot be static", required_span.or(name_span));
        }
        if is_required && matches!(context, PropertyContext::Interface) {
            self.push_error(
                "`required` modifier is not supported on interface properties",
                required_span.or(name_span),
            );
        }
        if is_required && is_static {
            self.push_error(
                "`required` modifier is not supported on static properties",
                required_span.or(name_span),
            );
        }
        let is_abstract = normalized_modifiers.iter().any(|m| m == "abstract");
        if is_abstract && !context.allows_bodies() {
            self.push_error(
                "`abstract` modifier is redundant on interface properties",
                name_span,
            );
        }

        let property_dispatch = match context {
            PropertyContext::Class => self.build_dispatch_from_markers(
                dispatch_markers,
                "properties",
                is_static,
                class_is_static,
                true,
            ),
            PropertyContext::Struct => {
                if dispatch_markers.any() {
                    self.reject_dispatch_markers(dispatch_markers, "struct properties");
                }
                MemberDispatch::default()
            }
            PropertyContext::Interface => {
                if dispatch_markers.any() {
                    self.reject_dispatch_markers(dispatch_markers, "interface properties");
                }
                MemberDispatch::default()
            }
        };

        if self.check_operator("=>") {
            if !context.allows_bodies() {
                self.push_error(
                    "interface properties must declare accessors using ';'",
                    name_span,
                );
                self.advance();
                self.skip_expression_until_semicolon();
                let _ = self.consume_punctuation(';');
                return None;
            }
            self.advance();
            let mut cursor = self.index;
            let mut depth_paren = 0usize;
            let mut depth_bracket = 0usize;
            let mut saw_conflicting_brace = None;
            while let Some(token) = self.tokens.get(cursor) {
                match token.kind {
                    TokenKind::Punctuation('(') => depth_paren += 1,
                    TokenKind::Punctuation(')') => depth_paren = depth_paren.saturating_sub(1),
                    TokenKind::Punctuation('[') => depth_bracket += 1,
                    TokenKind::Punctuation(']') => {
                        depth_bracket = depth_bracket.saturating_sub(1);
                    }
                    TokenKind::Punctuation(';')
                        if depth_paren == 0 && depth_bracket == 0 =>
                    {
                        break;
                    }
                    TokenKind::Punctuation('{')
                        if depth_paren == 0 && depth_bracket == 0 =>
                    {
                        saw_conflicting_brace = Some(token.span);
                        break;
                    }
                    _ => {}
                }
                cursor += 1;
            }
            if let Some(conflict_span) = saw_conflicting_brace {
                self.index = cursor;
                self.push_error(
                    "cannot combine expression-bodied property with accessor list",
                    Some(conflict_span),
                );
                self.skip_braced_block();
                let _ = self.consume_punctuation(';');
                return None;
            }
            let expression = self.collect_expression_until(&[';']);
            if !self.expect_punctuation(';') {
                return None;
            }
            let span = self
                .span_from_indices(name_token_index, self.index)
                .or(name_span);
            let accessor = PropertyAccessor {
                kind: PropertyAccessorKind::Get,
                visibility: None,
                body: PropertyAccessorBody::Expression(expression),
                doc: None,
                attributes: None,
                span,
                dispatch: property_dispatch,
            };
            return Some(PropertyDecl {
                visibility,
                modifiers: normalized_modifiers,
                name,
                ty,
                parameters,
                accessors: vec![accessor],
                doc,
                is_required,
                is_static,
                initializer: None,
                span,
                attributes: Vec::new(),
                di_inject: None,
                dispatch: property_dispatch,
                explicit_interface,
            });
        }

        if !self.check_punctuation('{') {
            self.push_error(
                "expected property accessor list `{ ... }`, expression-bodied accessor `=>`, or ';'",
                self.peek().map(|token| token.span),
            );
            return None;
        }

        self.advance();
        let mut accessors: Vec<PropertyAccessor> = Vec::new();
        let mut saw_auto = false;
        let mut saw_manual = false;
        let mut seen_get = false;
        let mut seen_set = false;
        let mut seen_init = false;

        while !self.check_punctuation('}') && !self.is_at_end() {
            self.stash_leading_doc();
            self.skip_attributes();
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                break;
            }

            let accessor_doc = self.take_pending_doc();
            let accessor_span_start = self.index;
            let accessor_visibility = self.parse_accessor_visibility_override();
            let accessor_markers = self.parse_accessor_dispatch_modifiers();

            let Some(kind_token) = self.peek().cloned() else {
                break;
            };
            let kind = match kind_token.kind {
                TokenKind::Keyword(Keyword::Get) => {
                    self.advance();
                    if seen_get {
                        self.push_error(
                            "property already declares a `get` accessor",
                            Some(kind_token.span),
                        );
                        self.skip_accessor_body(context);
                        continue;
                    }
                    seen_get = true;
                    PropertyAccessorKind::Get
                }
                TokenKind::Keyword(Keyword::Set) => {
                    self.advance();
                    if seen_set {
                        self.push_error(
                            "property already declares a `set` accessor",
                            Some(kind_token.span),
                        );
                        self.skip_accessor_body(context);
                        continue;
                    }
                    if seen_init {
                        self.push_error(
                            "properties cannot declare both `set` and `init` accessors",
                            Some(kind_token.span),
                        );
                        self.skip_accessor_body(context);
                        continue;
                    }
                    seen_set = true;
                    PropertyAccessorKind::Set
                }
                TokenKind::Keyword(Keyword::Init) => {
                    self.advance();
                    if seen_init {
                        self.push_error(
                            "property already declares an `init` accessor",
                            Some(kind_token.span),
                        );
                        self.skip_accessor_body(context);
                        continue;
                    }
                    if seen_set {
                        self.push_error(
                            "properties cannot declare both `set` and `init` accessors",
                            Some(kind_token.span),
                        );
                        self.skip_accessor_body(context);
                        continue;
                    }
                    seen_init = true;
                    PropertyAccessorKind::Init
                }
                _ => {
                    self.push_error(
                        "expected property accessor `get`, `set`, or `init`",
                        Some(kind_token.span),
                    );
                    self.advance();
                    self.skip_accessor_body(context);
                    continue;
                }
            };

            let body = match self.parse_accessor_body(context, is_abstract) {
                Some(body) => body,
                None => continue,
            };

            let accessor_span = self
                .span_from_indices(accessor_span_start, self.index)
                .or(kind_token.span.into());
            let accessor_dispatch = if accessor_markers.any() {
                self.build_dispatch_from_markers(
                    accessor_markers,
                    accessor_context_name(kind),
                    is_static,
                    class_is_static,
                    false,
                )
            } else {
                property_dispatch
            };

            match body {
                PropertyAccessorBody::Auto => saw_auto = true,
                _ => saw_manual = true,
            }

            if saw_auto && saw_manual {
                self.push_error(
                    "auto-property accessors may not mix `;` with custom bodies",
                    accessor_span,
                );
            }
            accessors.push(PropertyAccessor {
                kind,
                visibility: accessor_visibility,
                body,
                doc: accessor_doc,
                attributes: None,
                span: accessor_span,
                dispatch: accessor_dispatch,
            });
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        if accessors.is_empty() {
            self.push_error(
                "property must declare at least one accessor",
                name_span,
            );
        }
        let auto_accessors = parameters.is_empty() && accessors.iter().all(|a| a.body.is_auto());
        if auto_accessors && !seen_get {
            self.push_error(
                "auto-implemented property must declare a `get` accessor",
                name_span,
            );
        }

        let span = self
            .span_from_indices(name_token_index, self.index)
            .or(name_span);

        Some(PropertyDecl {
            visibility,
            modifiers: normalized_modifiers,
            name,
            ty,
            parameters,
            accessors,
            doc,
            is_required,
            is_static,
            initializer: None,
            span,
            attributes: Vec::new(),
            di_inject: None,
            dispatch: property_dispatch,
            explicit_interface,
        })
    }

    fn normalise_property_modifiers(
        &mut self,
        context: PropertyContext,
        modifiers: Vec<Modifier>,
    ) -> Vec<String> {
        let mut filtered = Vec::new();
        for modifier in modifiers {
            let modifier_lower = modifier.name.to_ascii_lowercase();
            let allowed = match context {
                PropertyContext::Class => matches!(
                    modifier_lower.as_str(),
                    "static" | "virtual" | "override" | "sealed" | "abstract"
                ),
                PropertyContext::Struct => modifier_lower.as_str() == "static",
                PropertyContext::Interface => matches!(
                    modifier_lower.as_str(),
                    "static" | "virtual" | "override" | "sealed" | "abstract"
                ),
            };
            if !allowed {
                self.push_error(
                    format!("modifier `{}` is not supported on properties", modifier.name),
                    Some(modifier.span),
                );
                continue;
            }
            if modifier_lower == "static" && matches!(context, PropertyContext::Interface) {
                self.push_error(
                    "interface properties may not be declared `static`",
                    Some(modifier.span),
                );
                continue;
            }
            if filtered.contains(&modifier_lower) {
                self.push_error(
                    format!("duplicate `{}` modifier on property", modifier.name),
                    Some(modifier.span),
                );
                continue;
            }
            filtered.push(modifier_lower);
        }
        filtered
    }

    fn parse_accessor_visibility_override(&mut self) -> Option<Visibility> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::Keyword(
                    Keyword::Public
                    | Keyword::Private
                    | Keyword::Protected
                    | Keyword::Internal,
                ),
                ..
            }) => Some(self.parse_visibility()),
            _ => None,
        }
    }

    fn parse_accessor_dispatch_modifiers(&mut self) -> DispatchModifiers {
        let mut markers = DispatchModifiers::default();
        loop {
            let Some(token) = self.peek() else {
                break;
            };
            let lexeme = token.lexeme.as_str();
            let slot = if lexeme.eq_ignore_ascii_case("virtual") {
                &mut markers.virtual_span
            } else if lexeme.eq_ignore_ascii_case("override") {
                &mut markers.override_span
            } else if lexeme.eq_ignore_ascii_case("sealed") {
                &mut markers.sealed_span
            } else if lexeme.eq_ignore_ascii_case("abstract") {
                &mut markers.abstract_span
            } else {
                break;
            };
            if slot.is_some() {
                self.push_error(
                    format!("duplicate `{}` modifier on property accessor", lexeme),
                    Some(token.span),
                );
            } else {
                *slot = Some(token.span);
            }
            self.advance();
        }
        markers
    }

    fn parse_accessor_body(
        &mut self,
        context: PropertyContext,
        is_abstract: bool,
    ) -> Option<PropertyAccessorBody> {
        if self.consume_punctuation(';') {
            return Some(PropertyAccessorBody::Auto);
        }

        if !context.allows_bodies() || is_abstract {
            self.push_error(
                "accessor bodies are not permitted in this context",
                self.peek().map(|token| token.span),
            );
            self.skip_accessor_body(context);
            return None;
        }

        if self.check_operator("=>") {
            self.advance();
            let expression = self.collect_expression_until(&[';']);
            if !self.expect_punctuation(';') {
                return None;
            }
            return Some(PropertyAccessorBody::Expression(expression));
        }

        if self.check_punctuation('{') {
            return self.parse_block().map(PropertyAccessorBody::Block);
        }

        self.push_error(
            "expected accessor body `{ ... }`, expression `=>`, or ';'",
            self.peek().map(|token| token.span),
        );
        self.skip_accessor_body(context);
        None
    }

    fn skip_accessor_body(&mut self, context: PropertyContext) {
        if self.consume_punctuation(';') {
            return;
        }

        if context.allows_bodies() {
            if self.check_operator("=>") {
                self.advance();
                self.skip_expression_until_semicolon();
                let _ = self.consume_punctuation(';');
                return;
            }
            if self.check_punctuation('{') {
                self.skip_braced_block();
                return;
            }
        }

        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(';' | '}') => break,
                _ => {
                    self.advance();
                }
            }
        }
        let _ = self.consume_punctuation(';');
    }

    fn skip_braced_block(&mut self) {
        if !self.check_punctuation('{') {
            return;
        }
        self.advance();
        let mut depth = 1usize;
        while let Some(token) = self.advance() {
            match token.kind {
                TokenKind::Punctuation('{') => depth += 1,
                TokenKind::Punctuation('}') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            self.push_error("unterminated property accessor body", None);
        }
    }
}

fn accessor_context_name(kind: PropertyAccessorKind) -> &'static str {
    match kind {
        PropertyAccessorKind::Get => "get accessors",
        PropertyAccessorKind::Set => "set accessors",
        PropertyAccessorKind::Init => "init accessors",
    }
}
