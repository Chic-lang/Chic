use super::type_expr_parser::{parse_type_expression_text, parse_type_expression_text_with_span};
use super::*;
use crate::frontend::ast::{LendsClause, RefKind};

parser_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "Type lookahead helper aggregates parsing rules; split once grammar stabilises."
    )]
    pub(super) fn try_type_expr_from(&self, start_index: usize) -> Option<(TypeExpr, usize)> {
        let mut index = start_index;
        loop {
            let Some(token) = self.tokens.get(index) else {
                return None;
            };
            if matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) {
                index += 1;
                continue;
            }
            match token.kind {
                TokenKind::Operator("*") => {
                    index += 1;
                    if let Some(next) = self.tokens.get(index) {
                        if matches!(
                            next.kind,
                            TokenKind::Keyword(Keyword::Mut | Keyword::Const)
                        ) {
                            index += 1;
                        }
                    }
                    if !skip_pointer_qualifiers(&self.tokens, &mut index) {
                        return None;
                    }
                }
                _ => break,
            }
        }

        while let Some(token) = self.tokens.get(index) {
            if matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) {
                index += 1;
                continue;
            }
            break;
        }
        let head = self.tokens.get(index)?;
        let mut in_trait_object = matches!(
            head.kind,
            TokenKind::Keyword(Keyword::Dyn | Keyword::Impl)
        );
        let mut expect_trait_ident = in_trait_object;
        let mut angle_depth = 0usize;
        let mut square_depth = 0usize;
        let mut paren_depth = 0usize;
        let mut tuple_start = false;

        if matches!(head.kind, TokenKind::Keyword(Keyword::Fn)) {
            return try_type_expr_via_text(&self.tokens, start_index);
        }

        if !matches!(
            head.kind,
            TokenKind::Identifier
                | TokenKind::Punctuation('(')
                | TokenKind::Keyword(Keyword::Dyn | Keyword::Impl)
        ) {
            if let Some(result) = try_type_expr_via_text(&self.tokens, start_index) {
                return Some(result);
            }
            return None;
        }

        match head.kind {
            TokenKind::Identifier => {
                index += 1;
            }
            TokenKind::Punctuation('(') => {
                tuple_start = true;
                paren_depth = 1;
                index += 1;
            }
            TokenKind::Keyword(Keyword::Dyn | Keyword::Impl) => {
                index += 1;
                in_trait_object = true;
                expect_trait_ident = true;
            }
            _ => return None,
        }

        while let Some(token) = self.tokens.get(index) {
            if matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) {
                index += 1;
                continue;
            }
            match &token.kind {
                TokenKind::Punctuation('(') => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth += 1;
                    index += 1;
                }
                TokenKind::Punctuation(')') => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                    index += 1;
                }
                TokenKind::Punctuation('.') => {
                    if paren_depth > 0 || angle_depth > 0 || square_depth > 0 {
                        index += 1;
                    } else {
                        let next = self.tokens.get(index + 1)?;
                        if !matches!(next.kind, TokenKind::Identifier) {
                            return None;
                        }
                        index += 2;
                        if in_trait_object {
                            expect_trait_ident = false;
                        }
                    }
                }
                TokenKind::Punctuation('<') | TokenKind::Operator("<") => {
                    if paren_depth > 0 {
                        // Nested generics inside tuple elements.
                        angle_depth += 1;
                        index += 1;
                        continue;
                    }
                    angle_depth += 1;
                    index += 1;
                }
                TokenKind::Punctuation('>') | TokenKind::Operator(">") => {
                    if angle_depth == 0 {
                        break;
                    }
                    angle_depth -= 1;
                    index += 1;
                }
                TokenKind::Operator(">>") => {
                    if angle_depth < 2 {
                        break;
                    }
                    angle_depth -= 2;
                    index += 1;
                }
                TokenKind::Punctuation('[') => {
                    square_depth += 1;
                    index += 1;
                }
                TokenKind::Punctuation(']') => {
                    if square_depth == 0 {
                        break;
                    }
                    square_depth -= 1;
                    index += 1;
                }
                TokenKind::Punctuation(',') => {
                    if paren_depth > 0 || angle_depth > 0 || square_depth > 0 {
                        index += 1;
                    } else {
                        break;
                    }
                }
                TokenKind::Punctuation('@') => {
                    if !skip_pointer_qualifiers(&self.tokens, &mut index) {
                        return None;
                    }
                }
                TokenKind::Punctuation('?') | TokenKind::Operator("*" | "&") => {
                    index += 1;
                }
                TokenKind::Operator("+") => {
                    if in_trait_object {
                        index += 1;
                        expect_trait_ident = true;
                    } else {
                        break;
                    }
                }
                TokenKind::Identifier => {
                    if in_trait_object && expect_trait_ident {
                        expect_trait_ident = false;
                        index += 1;
                    } else if angle_depth > 0 || square_depth > 0 || paren_depth > 0 {
                        index += 1;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
            if tuple_start && paren_depth == 0 {
                tuple_start = false;
            }
        }

        if index == start_index {
            return try_type_expr_via_text(&self.tokens, start_index);
        }
        if let (Some(start_token), Some(end_token)) =
            (self.tokens.get(start_index), self.tokens.get(index - 1))
        {
            let start = start_token.span.start.min(self.source.len());
            let end = end_token.span.end.min(self.source.len());
            if start < end {
                let text = &self.source[start..end];
                let trimmed = text.trim();
                let leading = text.len().saturating_sub(text.trim_start().len());
                let offset = start + leading;
                if let Some(parsed) =
                    parse_type_expression_text_with_span(trimmed, Some(start_token.span.file_id), offset)
                {
                    return Some((parsed, index));
                }
            }
        }
        try_type_expr_via_text(&self.tokens, start_index)
    }

    pub(super) fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        let ref_kind = self.consume_ref_modifier();
        self.consume_all_borrow_qualifier_misuse(false);
        if self.peek_identifier("impl") || self.check_keyword(Keyword::Impl) {
            let span = self.peek().map(|token| token.span);
            self.advance();
            self.push_error(
                "`impl` is no longer supported; use concrete interface types instead",
                span,
            );
            return None;
        }
        if let Some((mut ty, next_index)) = self.try_type_expr_from(self.index) {
            if next_index > 0 {
                self.last_span = Some(self.tokens[next_index - 1].span);
            }
            self.index = next_index;
            if let Some(kind) = ref_kind {
                ty.ref_kind = Some(kind);
            }
            Some(ty)
        } else {
            if let Some(token) = self.peek() {
                self.push_error("expected type name", Some(token.span));
                self.advance();
            } else {
                self.push_error("expected type", None);
            }
            None
        }
    }

    pub(super) fn parse_type_list(&mut self) -> Vec<TypeExpr> {
        let mut types = Vec::new();
        while let Some(ty) = self.parse_type_expr() {
            types.push(ty);
            if !self.consume_punctuation(',') {
                break;
            }
        }
        types
    }

    pub(super) fn parse_lends_clause(&mut self) -> Option<LendsClause> {
        if !self.match_keyword(Keyword::Lends) {
            return None;
        }

        let start_span = self.last_span;
        if !self.expect_punctuation('(') {
            return None;
        }

        let mut targets = Vec::new();
        loop {
            if self.check_punctuation(')') {
                break;
            }

            match self.consume_identifier("expected parameter name in `lends(...)` clause") {
                Some(name) => {
                    if targets.contains(&name) {
                        self.push_error(
                            format!("duplicate `lends` target `{name}`"),
                            self.last_span,
                        );
                    } else {
                        targets.push(name);
                    }
                }
                None => {
                    self.synchronize_parameter();
                    break;
                }
            }

            if self.consume_punctuation(',') {
                continue;
            }

            if self.check_punctuation(')') {
                break;
            }

            let span = self.peek().map(|token| token.span);
            self.push_error("expected ',' or ')' after lends target", span);
            self.synchronize_parameter();
            break;
        }

        if !self.expect_punctuation(')') {
            return None;
        }

        if targets.is_empty() {
            self.push_error("`lends` clause must name at least one parameter", start_span);
        }

        let span = match (start_span, self.last_span) {
            (Some(start), Some(end)) if end.end >= start.start => {
                Some(Span::in_file(start.file_id, start.start, end.end))
            }
            _ => start_span.or(self.last_span),
        };

        Some(LendsClause::new(targets, span))
    }

    fn consume_ref_modifier(&mut self) -> Option<RefKind> {
        if let Some(token) = self.peek() {
            if matches!(token.kind, TokenKind::Keyword(Keyword::Ref)) {
                self.advance();
                if self
                    .peek()
                    .is_some_and(|next| matches!(next.kind, TokenKind::Keyword(Keyword::Readonly)))
                {
                    self.advance();
                    return Some(RefKind::ReadOnly);
                }
                return Some(RefKind::Ref);
            }
        }
        None
    }
}

fn skip_pointer_qualifiers(tokens: &[Token], index: &mut usize) -> bool {
    loop {
        let Some(token) = tokens.get(*index) else {
            return true;
        };
        if matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) {
            *index += 1;
            continue;
        }
        if !matches!(token.kind, TokenKind::Punctuation('@')) {
            return true;
        }
        *index += 1;
        let Some(next) = tokens.get(*index) else {
            return false;
        };
        if matches!(next.kind, TokenKind::Identifier | TokenKind::Keyword(_)) {
            *index += 1;
        } else {
            return false;
        }
        if let Some(paren) = tokens.get(*index) {
            if matches!(paren.kind, TokenKind::Punctuation('(')) {
                let mut depth = 1usize;
                *index += 1;
                while let Some(tok) = tokens.get(*index) {
                    match tok.kind {
                        TokenKind::Punctuation('(') => {
                            depth += 1;
                            *index += 1;
                        }
                        TokenKind::Punctuation(')') => {
                            depth -= 1;
                            *index += 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => *index += 1,
                    }
                }
                if depth != 0 {
                    return false;
                }
            }
        }
    }
}

fn try_type_expr_via_text(tokens: &[Token], start_index: usize) -> Option<(TypeExpr, usize)> {
    let mut acc = String::new();
    let mut last_ty = None;
    let mut last_end = start_index;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut square_depth = 0usize;
    for (offset, token) in tokens[start_index..].iter().enumerate() {
        match &token.kind {
            TokenKind::Punctuation('<') => angle_depth += 1,
            TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => {
                angle_depth += op.len();
            }
            TokenKind::Punctuation('>') => {
                if angle_depth == 0 {
                    break;
                }
                angle_depth -= 1;
            }
            TokenKind::Operator(op) if op.chars().all(|ch| ch == '>') => {
                if angle_depth < op.len() {
                    break;
                }
                angle_depth -= op.len();
            }
            TokenKind::Punctuation('(') => paren_depth += 1,
            TokenKind::Punctuation(')') => {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
            }
            TokenKind::Punctuation('[') => square_depth += 1,
            TokenKind::Punctuation(']') => {
                if square_depth == 0 {
                    break;
                }
                square_depth -= 1;
            }
            _ => {}
        }
        acc.push_str(token.lexeme.as_str());
        let next_kind = tokens.get(start_index + offset + 1).map(|next| &next.kind);
        let add_space = match (&token.kind, next_kind) {
            (TokenKind::Identifier, Some(TokenKind::Operator("*" | "&"))) => false,
            (TokenKind::Identifier, _) => true,
            (TokenKind::Keyword(_), _) => true,
            (TokenKind::Operator("*"), _) => true,
            (TokenKind::Operator("->"), _) => true,
            _ => false,
        };
        if add_space {
            acc.push(' ');
        }
        let candidate_end = start_index + offset + 1;
        if let Some(parsed) = parse_type_expression_text(acc.trim()) {
            last_ty = Some(parsed);
            last_end = candidate_end;
            continue;
        }
        if last_ty.is_some() && angle_depth == 0 && paren_depth == 0 && square_depth == 0 {
            break;
        }
    }
    last_ty.map(|ty| (ty, last_end))
}
