use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, TokenKind};

use super::{GenericConstraint, GenericConstraintKind, GenericParam, GenericParams, Parser};
use crate::frontend::ast::{AutoTraitConstraint, ConstWherePredicate, Variance};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GenericVarianceMode {
    Disallow,
    InterfaceOrDelegate,
}

impl GenericVarianceMode {
    fn allows_variance(self) -> bool {
        matches!(self, Self::InterfaceOrDelegate)
    }
}

parser_impl! {
    pub(super) fn parse_generic_parameter_list(&mut self) -> Option<GenericParams> {
        self.parse_generic_parameter_list_with_mode(GenericVarianceMode::Disallow)
    }

    pub(super) fn parse_generic_parameter_list_allowing_variance(&mut self) -> Option<GenericParams> {
        self.parse_generic_parameter_list_with_mode(GenericVarianceMode::InterfaceOrDelegate)
    }

    fn parse_generic_parameter_list_with_mode(
        &mut self,
        variance_mode: GenericVarianceMode,
    ) -> Option<GenericParams> {
        if !self.check_punctuation('<') {
            return None;
        }

        let start_span = self.peek().map(|token| token.span);
        self.advance();

        if self.check_punctuation('>') {
            self.push_error("generic parameter list cannot be empty", start_span);
            let end_span = self.peek().map(|token| token.span);
            self.advance();
            let span = start_span
                .zip(end_span)
                .map(|(start, end)| Span::in_file(start.file_id, start.start, end.end));
            return Some(GenericParams::new(span, Vec::new()));
        }

        let mut params = Vec::new();
        let mut seen = HashSet::new();
        loop {
            let mut variance = Variance::Invariant;
            let mut variance_span = None;
            if let Some(token) = self.peek().cloned() {
                if let TokenKind::Keyword(keyword) = token.kind {
                    let parsed_variance = match keyword {
                        Keyword::In => Some(Variance::Contravariant),
                        Keyword::Out => Some(Variance::Covariant),
                        _ => None,
                    };
                    if let Some(parsed) = parsed_variance {
                        self.advance();
                        variance_span = Some(token.span);
                        if variance_mode.allows_variance() {
                            variance = parsed;
                        } else {
                            self.push_error(
                                "`in`/`out` variance modifiers are only supported on interface or delegate type parameters",
                                variance_span,
                            );
                        }
                    }
                }
            }

            let is_const = self.match_keyword(Keyword::Const);
            let ident_span = self.peek().map(|token| token.span);
            let Some(name) = self.consume_identifier("expected type parameter name") else {
                self.skip_generic_parameter_list();
                break;
            };
            let duplicate = !seen.insert(name.clone());
            if duplicate {
                self.push_error(
                    format!("duplicate type parameter `{name}` in generic parameter list"),
                    ident_span,
                );
            }
            if is_const {
                if variance != Variance::Invariant {
                    self.push_error(
                        "variance modifiers are not allowed on const generic parameters",
                        variance_span.or(ident_span),
                    );
                }
                if !self.expect_punctuation(':') {
                    self.skip_generic_parameter_list();
                    break;
                }
                let Some(ty) = self.parse_type_expr() else {
                    self.push_error(
                        "const generic parameter requires a type annotation",
                        ident_span,
                    );
                    self.skip_generic_parameter_list();
                    break;
                };
                params.push(GenericParam::const_param(name, ident_span, ty));
            } else {
                let mut param = GenericParam::type_param(name, ident_span);
                if let Some(data) = param.as_type_mut() {
                    data.variance = variance;
                }
                params.push(param);
            }

            if self.check_punctuation('>') {
                self.advance();
                break;
            }

            match self.peek() {
                Some(token) if matches!(token.kind, TokenKind::Punctuation(',')) => {
                    self.advance();
                    if self.check_punctuation('>') {
                        self.advance();
                        break;
                    }
                }
                Some(token) if matches!(token.kind, TokenKind::Punctuation('>')) => {
                    self.advance();
                    break;
                }
                Some(token) => {
                    self.push_error(
                        "expected `,` or `>` in generic parameter list",
                        Some(token.span),
                    );
                    self.skip_generic_parameter_list();
                    break;
                }
                None => {
                    self.push_error(
                        "expected `>` to close generic parameter list",
                        ident_span,
                    );
                    break;
                }
            }
        }

        let span = match (start_span, self.last_span) {
            (Some(start), Some(end)) if end.end >= start.start => {
                Some(Span::in_file(start.file_id, start.start, end.end))
            }
            _ => start_span,
        };

        Some(GenericParams::new(span, params))
    }

    pub(super) fn parse_where_clauses(&mut self, generics: &mut Option<GenericParams>) {
        let mut clauses = Vec::new();
        let mut const_clauses = Vec::new();
        while self.match_keyword(Keyword::Where) {
            let where_span = self.last_span;
            let clause_start = where_span.map(|span| span.start);
            let name_span = self.peek().map(|token| token.span);
            let Some(parameter) = self.consume_identifier("expected type parameter name") else {
                self.synchronize_where_clause();
                continue;
            };

            if !self.expect_punctuation(':') {
                self.synchronize_where_clause();
                continue;
            }

            if self.check_keyword(Keyword::Const) {
                let predicates = self.parse_const_predicates();
                if predicates.is_empty() {
                    self.push_error(
                        "expected at least one const predicate after ':'",
                        where_span.or(name_span),
                    );
                }
                let clause_end = self.last_span.map(|span| span.end);
                let span = match (clause_start, clause_end) {
                    (Some(start), Some(end)) if end >= start => {
                        Some(Span::in_file(self.file_id, start, end))
                    }
                    _ => where_span.or(name_span),
                };
                const_clauses.push((parameter, predicates, span));
                continue;
            }

            let mut constraints = Vec::new();
            loop {
                let checkpoint = self.index;
                match self.parse_generic_constraint() {
                    Some(constraint) => constraints.push(constraint),
                    None => {
                        if self.index == checkpoint && self.peek().is_some() {
                            self.advance();
                        }
                        break;
                    }
                }

                if !self.consume_punctuation(',') {
                    break;
                }
            }

            if constraints.is_empty() {
                self.push_error(
                    "expected at least one constraint after ':'",
                    where_span.or(name_span),
                );
            }

            let clause_end = self.last_span.map(|span| span.end);
            let span = match (clause_start, clause_end) {
                (Some(start), Some(end)) if end >= start => {
                    Some(Span::in_file(self.file_id, start, end))
                }
                _ => where_span.or(name_span),
            };

            clauses.push((parameter, constraints, span));
        }

        if clauses.is_empty() && const_clauses.is_empty() {
            return;
        }

        let Some(generic_params) = generics.as_mut() else {
            for (_, _, span) in clauses {
                self.push_error("`where` clause requires type parameters", span);
            }
            for (_, _, span) in const_clauses {
                self.push_error("`where` clause requires type parameters", span);
            }
            return;
        };

        for (name, constraints, span) in clauses {
            match generic_params
                .params
                .iter_mut()
                .find(|param| param.name == name)
            {
                Some(param) => {
                    if let Some(data) = param.as_type_mut() {
                        data.constraints.extend(constraints);
                    } else {
                        self.push_error(
                            format!("`{name}` is not a type parameter"),
                            span,
                        );
                    }
                }
                None => self.push_error(
                    format!("constraint references unknown type parameter `{name}`"),
                    span,
                ),
            }
        }

        for (name, predicates, span) in const_clauses {
            match generic_params
                .params
                .iter_mut()
                .find(|param| param.name == name)
            {
                Some(param) => {
                    if let Some(data) = param.as_const_mut() {
                        data.constraints.extend(predicates);
                    } else {
                        self.push_error(
                            format!("`{name}` is not a const generic parameter"),
                            span,
                        );
                    }
                }
                None => self.push_error(
                    format!("constraint references unknown const parameter `{name}`"),
                    span,
                ),
            }
        }

        self.validate_generic_constraints(generic_params);
    }
}

impl<'a> Parser<'a> {
    fn parse_const_predicates(&mut self) -> Vec<ConstWherePredicate> {
        let mut predicates = Vec::new();
        loop {
            if !self.match_keyword(Keyword::Const) {
                break;
            }
            let const_span = self.last_span;
            if !self.expect_punctuation('(') {
                self.synchronize_where_clause();
                break;
            }
            let expr = self.collect_expression_until(&[')']);
            if expr.text.trim().is_empty() {
                self.push_error("const predicate requires an expression", expr.span);
            }
            if !self.expect_punctuation(')') {
                self.synchronize_where_clause();
                break;
            }
            let span = const_span.or(expr.span);
            predicates.push(ConstWherePredicate::new(expr, span));
            if !self.consume_punctuation(',') {
                break;
            }
        }
        predicates
    }

    fn parse_generic_constraint(&mut self) -> Option<GenericConstraint> {
        let token = self.peek()?;
        match &token.kind {
            TokenKind::Keyword(Keyword::Struct) => {
                let span = Some(token.span);
                self.advance();
                Some(GenericConstraint::new(GenericConstraintKind::Struct, span))
            }
            TokenKind::Keyword(Keyword::Class) => {
                let span = Some(token.span);
                self.advance();
                Some(GenericConstraint::new(GenericConstraintKind::Class, span))
            }
            TokenKind::Keyword(Keyword::Notnull) => {
                let span = Some(token.span);
                self.advance();
                Some(GenericConstraint::new(GenericConstraintKind::NotNull, span))
            }
            TokenKind::Keyword(Keyword::New) => self.parse_new_constraint(),
            TokenKind::Punctuation('@') => self.parse_auto_trait_constraint(),
            _ => self.parse_type_constraint(),
        }
    }

    fn parse_auto_trait_constraint(&mut self) -> Option<GenericConstraint> {
        let start = self.peek().map(|token| token.span);
        self.advance();
        let name_span = self.peek().map(|token| token.span);
        let Some(name) = self.consume_identifier("expected auto-trait name after '@'") else {
            return None;
        };
        let lower = name.to_ascii_lowercase();
        let trait_kind = match lower.as_str() {
            "thread_safe" => AutoTraitConstraint::ThreadSafe,
            "shareable" => AutoTraitConstraint::Shareable,
            other => {
                self.push_error(
                    format!("unknown auto-trait constraint `@{other}`"),
                    name_span,
                );
                return None;
            }
        };
        let span = match (start, self.last_span) {
            (Some(begin), Some(end)) if end.end >= begin.start => {
                Some(Span::in_file(begin.file_id, begin.start, end.end))
            }
            _ => name_span,
        };
        Some(GenericConstraint::new(
            GenericConstraintKind::AutoTrait(trait_kind),
            span,
        ))
    }

    fn parse_new_constraint(&mut self) -> Option<GenericConstraint> {
        let start = self.peek().map(|token| token.span);
        self.advance();
        if !self.expect_punctuation('(') {
            self.synchronize_where_clause();
            return None;
        }
        if !self.expect_punctuation(')') {
            self.synchronize_where_clause();
            return None;
        }

        let span = match (start, self.last_span) {
            (Some(begin), Some(end)) if end.end >= begin.start => {
                Some(Span::in_file(begin.file_id, begin.start, end.end))
            }
            _ => start,
        };
        Some(GenericConstraint::new(
            GenericConstraintKind::DefaultConstructor,
            span,
        ))
    }

    fn parse_type_constraint(&mut self) -> Option<GenericConstraint> {
        let start = self.peek().map(|token| token.span.start);
        let ty = self.parse_type_expr()?;
        let span = match (start, self.last_span) {
            (Some(start), Some(end)) if end.end >= start => {
                Some(Span::in_file(end.file_id, start, end.end))
            }
            _ => None,
        };
        Some(GenericConstraint::new(
            GenericConstraintKind::Type(ty),
            span,
        ))
    }

    fn validate_generic_constraints(&mut self, generics: &GenericParams) {
        for param in &generics.params {
            let Some(data) = param.as_type() else {
                continue;
            };
            let mut struct_seen = false;
            let mut struct_span: Option<Span> = None;
            let mut class_seen = false;
            let mut class_span: Option<Span> = None;
            let mut notnull_seen = false;
            let mut notnull_span: Option<Span> = None;
            let mut saw_new: Option<(usize, Option<Span>)> = None;
            let mut seen_types: HashMap<String, Option<Span>> = HashMap::new();

            for (index, constraint) in data.constraints.iter().enumerate() {
                match &constraint.kind {
                    GenericConstraintKind::Struct => {
                        if struct_seen {
                            self.push_error(
                                format!(
                                    "duplicate `struct` constraint on type parameter `{}`",
                                    param.name
                                ),
                                constraint.span.or(struct_span),
                            );
                        }
                        struct_seen = true;
                        if constraint.span.is_some() {
                            struct_span = constraint.span;
                        }
                        if class_seen {
                            self.push_error(
                                format!(
                                    "type parameter `{}` cannot be constrained as both `struct` and `class`",
                                    param.name
                                ),
                                constraint.span.or(class_span),
                            );
                        }
                    }
                    GenericConstraintKind::Class => {
                        if class_seen {
                            self.push_error(
                                format!(
                                    "duplicate `class` constraint on type parameter `{}`",
                                    param.name
                                ),
                                constraint.span.or(class_span),
                            );
                        }
                        class_seen = true;
                        if constraint.span.is_some() {
                            class_span = constraint.span;
                        }
                        if struct_seen {
                            self.push_error(
                                format!(
                                    "type parameter `{}` cannot be constrained as both `struct` and `class`",
                                    param.name
                                ),
                                constraint.span.or(struct_span),
                            );
                        }
                    }
                    GenericConstraintKind::NotNull => {
                        if notnull_seen {
                            self.push_error(
                                format!(
                                    "duplicate `notnull` constraint on type parameter `{}`",
                                    param.name
                                ),
                                constraint.span.or(notnull_span),
                            );
                        }
                        notnull_seen = true;
                        if constraint.span.is_some() {
                            notnull_span = constraint.span;
                        }
                    }
                    GenericConstraintKind::DefaultConstructor => {
                        if let Some((_, prev_span)) = saw_new {
                            self.push_error(
                                format!(
                                    "duplicate `new()` constraint on type parameter `{}`",
                                    param.name
                                ),
                                constraint.span.or(prev_span),
                            );
                        }
                        saw_new = Some((index, constraint.span));
                    }
                    GenericConstraintKind::Type(ty) => {
                        let key = ty.name.clone();
                        if let Some(prev_span) = seen_types.insert(key.clone(), constraint.span) {
                            self.push_error(
                                format!(
                                    "duplicate constraint `{}` on type parameter `{}`",
                                    key, param.name
                                ),
                                constraint.span.or(prev_span),
                            );
                        }
                    }
                    GenericConstraintKind::AutoTrait(_) => {}
                }
            }

            if let Some((position, span)) = saw_new {
                if position + 1 != data.constraints.len() {
                    self.push_error(
                        format!(
                            "`new()` constraint must be the final constraint for type parameter `{}`",
                            param.name
                        ),
                        span,
                                            );
                }
            }
        }
    }

    fn skip_generic_parameter_list(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation('>') => {
                    self.advance();
                    break;
                }
                TokenKind::Punctuation('{' | ';' | '(' | ')') => break,
                TokenKind::Keyword(Keyword::Where) => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn synchronize_where_clause(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Keyword(Keyword::Where)
                | TokenKind::Punctuation('{' | '}' | ';' | '(' | ')')
                | TokenKind::Operator("=>") => break,
                _ => {
                    self.advance();
                }
            }
        }
    }
}
