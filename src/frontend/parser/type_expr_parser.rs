use crate::diagnostics::FileId;
use crate::frontend::ast::{
    ArrayRankSpecifier, FnTypeAbi, FnTypeExpr, GenericArgument, PointerModifier, RefKind,
    TraitObjectTypeExpr, TypeExpr, TypeSuffix, expressions::Expression,
};
use crate::frontend::diagnostics::Span;
use crate::unicode::identifier;

pub(crate) fn parse_type_expression_text(source: &str) -> Option<TypeExpr> {
    parse_type_expression_text_with_span(source, None, 0)
}

pub(crate) fn parse_type_expression_text_with_span(
    source: &str,
    file_id: Option<FileId>,
    start_offset: usize,
) -> Option<TypeExpr> {
    let mut parser = TypeTextParser::new(source, file_id, start_offset);
    let mut ty = parser.parse_type_expr()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return None;
    }
    // Preserve the original surface text (trim to mirror previous behaviour).
    ty.name = source.trim().to_string();
    if ty.span.is_none() {
        ty.span = parser.span_from_bounds(0, source.len());
    }
    Some(ty)
}

struct TypeTextParser<'a> {
    text: &'a str,
    pos: usize,
    file_id: Option<FileId>,
    offset: usize,
}

impl<'a> TypeTextParser<'a> {
    fn new(text: &'a str, file_id: Option<FileId>, offset: usize) -> Self {
        Self {
            text,
            pos: 0,
            file_id,
            offset,
        }
    }

    fn span_from_bounds(&self, start: usize, end: usize) -> Option<Span> {
        if start >= end {
            return None;
        }
        let file_id = self.file_id.unwrap_or(FileId::UNKNOWN);
        Some(Span::in_file(
            file_id,
            self.offset + start,
            self.offset + end,
        ))
    }

    fn trim_bounds(&self, start: usize, end: usize) -> (usize, usize) {
        let mut trimmed_start = start;
        let mut trimmed_end = end;
        while let Some(ch) = self.text[trimmed_start..end].chars().next() {
            if ch.is_whitespace() {
                trimmed_start += ch.len_utf8();
            } else {
                break;
            }
        }
        while trimmed_end > trimmed_start {
            let ch = self.text[..trimmed_end].chars().rev().next().unwrap();
            if ch.is_whitespace() {
                trimmed_end -= ch.len_utf8();
            } else {
                break;
            }
        }
        (trimmed_start, trimmed_end)
    }

    fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        self.skip_whitespace();
        let view = self.consume_keyword("view");
        if view {
            self.skip_whitespace();
        }
        let start = self.pos;

        let ref_kind = self.parse_ref_modifier();
        let pointer_prefixes = self.parse_pointer_prefixes()?;

        let mut ty = if self.consume_keyword("dyn") {
            self.parse_trait_object_type(start, false)?
        } else if self.consume_keyword("impl") {
            self.parse_trait_object_type(start, true)?
        } else if self.consume_keyword("fn") {
            self.parse_fn_type(start)?
        } else if self.consume_if('(') {
            self.parse_tuple_type(start)?
        } else {
            self.parse_named_type(start)?
        };

        if !pointer_prefixes.is_empty() {
            ty.suffixes.extend(pointer_prefixes.into_iter().rev());
            ty.name = self.text[start..self.pos].trim().to_string();
        }

        if view {
            ty.is_view = true;
        }
        if let Some(kind) = ref_kind {
            ty.ref_kind = Some(kind);
        }
        if ty.span.is_none() {
            ty.span = self.span_from_bounds(start, self.pos);
        }

        Some(ty)
    }

    fn parse_trait_object_type(&mut self, start: usize, opaque_impl: bool) -> Option<TypeExpr> {
        let mut bounds = Vec::new();

        loop {
            self.skip_whitespace();
            let bound_start = self.pos;
            let bound = self.parse_named_type(bound_start)?;
            bounds.push(bound);
            self.skip_whitespace();
            if self.consume_if('+') {
                continue;
            }
            break;
        }

        if bounds.is_empty() {
            return None;
        }

        let mut suffixes = Vec::new();
        let mut seen_nullable = false;
        loop {
            self.skip_whitespace();
            if self.consume_double_colon() {
                let qualifier = self.parse_identifier()?;
                suffixes.push(TypeSuffix::Qualifier(qualifier));
                continue;
            }
            let Some(ch) = self.peek() else {
                break;
            };
            match ch {
                '[' => {
                    self.consume();
                    let rank = self.parse_array_rank()?;
                    suffixes.push(TypeSuffix::Array(rank));
                }
                '?' => {
                    self.consume();
                    if seen_nullable {
                        return None;
                    }
                    seen_nullable = true;
                    suffixes.push(TypeSuffix::Nullable);
                }
                '*' => {
                    self.consume();
                    let pointer = self.parse_pointer_suffix_data()?;
                    suffixes.push(pointer);
                }
                '.' => {
                    self.consume();
                    let qualifier = self.parse_identifier()?;
                    suffixes.push(TypeSuffix::Qualifier(qualifier));
                }
                _ => break,
            }
        }

        let end = self.pos;
        Some(TypeExpr {
            name: self.text[start..end].trim().to_string(),
            base: Vec::new(),
            suffixes,
            span: self.span_from_bounds(start, end),
            generic_span: None,
            tuple_elements: None,
            tuple_element_names: None,
            fn_signature: None,
            trait_object: Some(TraitObjectTypeExpr {
                bounds,
                opaque_impl,
            }),
            ref_kind: None,
            is_view: false,
        })
    }

    fn parse_tuple_type(&mut self, start: usize) -> Option<TypeExpr> {
        let mut elements = Vec::new();
        let mut element_names = Vec::new();

        loop {
            self.skip_whitespace();
            if self.consume_if(')') {
                // Empty tuples are not supported.
                return None;
            }

            let element = self.parse_type_expr()?;
            elements.push(element);
            self.skip_whitespace();
            let mut name = None;
            if let Some(ch) = self.peek() {
                if ch.is_ascii_alphabetic() || ch == '_' {
                    let checkpoint = self.pos;
                    if let Some(identifier) = self.parse_identifier() {
                        name = Some(identifier);
                    } else {
                        self.pos = checkpoint;
                    }
                }
            }
            element_names.push(name);

            self.skip_whitespace();
            if self.consume_if(',') {
                self.skip_whitespace();
                if self.peek() == Some(')') {
                    // Trailing comma without element.
                    return None;
                }
                continue;
            }

            if self.consume_if(')') {
                break;
            }
            return None;
        }

        if elements.len() < 2 {
            return None;
        }

        let mut suffixes = Vec::new();
        let mut seen_nullable = false;
        loop {
            self.skip_whitespace();
            if self.consume_double_colon() {
                let qualifier = self.parse_identifier()?;
                suffixes.push(TypeSuffix::Qualifier(qualifier));
                continue;
            }
            let Some(ch) = self.peek() else {
                break;
            };
            match ch {
                '[' => {
                    self.consume();
                    let rank = self.parse_array_rank()?;
                    suffixes.push(TypeSuffix::Array(rank));
                }
                '?' => {
                    self.consume();
                    if seen_nullable {
                        return None;
                    }
                    seen_nullable = true;
                    suffixes.push(TypeSuffix::Nullable);
                }
                '*' => {
                    self.consume();
                    let pointer = self.parse_pointer_suffix_data()?;
                    suffixes.push(pointer);
                }
                '.' => {
                    self.consume();
                    let qualifier = self.parse_identifier()?;
                    suffixes.push(TypeSuffix::Qualifier(qualifier));
                }
                _ => break,
            }
        }

        let end = self.pos;
        let names = if element_names.iter().any(|name| name.is_some()) {
            Some(element_names)
        } else {
            None
        };
        Some(TypeExpr {
            name: self.text[start..end].trim().to_string(),
            base: Vec::new(),
            suffixes,
            span: self.span_from_bounds(start, end),
            generic_span: None,
            tuple_elements: Some(elements),
            tuple_element_names: names,
            fn_signature: None,
            trait_object: None,
            ref_kind: None,
            is_view: false,
        })
    }

    fn parse_fn_type(&mut self, start: usize) -> Option<TypeExpr> {
        let abi = self.parse_fn_abi()?;
        self.skip_whitespace();
        if !self.consume_if('(') {
            return None;
        }

        let mut variadic = false;
        let mut params = Vec::new();
        self.skip_whitespace();
        if !self.consume_if(')') {
            loop {
                self.skip_whitespace();
                if self.consume_str("...") {
                    variadic = true;
                    self.skip_whitespace();
                    if !self.consume_if(')') {
                        return None;
                    }
                    break;
                }
                let param = self.parse_type_expr()?;
                params.push(param);
                self.skip_whitespace();
                if self.consume_if(',') {
                    self.skip_whitespace();
                    if self.peek() == Some(')') {
                        return None;
                    }
                    continue;
                }
                if self.consume_if(')') {
                    break;
                }
                return None;
            }
        }

        self.skip_whitespace();
        if !self.consume_str("->") {
            return None;
        }

        let return_type = self.parse_type_expr()?;

        let mut suffixes = Vec::new();
        let mut seen_nullable = false;
        loop {
            self.skip_whitespace();
            let Some(ch) = self.peek() else {
                break;
            };
            match ch {
                '[' => {
                    self.consume();
                    let rank = self.parse_array_rank()?;
                    suffixes.push(TypeSuffix::Array(rank));
                }
                '?' => {
                    self.consume();
                    if seen_nullable {
                        return None;
                    }
                    seen_nullable = true;
                    suffixes.push(TypeSuffix::Nullable);
                }
                '*' => {
                    self.consume();
                    let pointer = self.parse_pointer_suffix_data()?;
                    suffixes.push(pointer);
                }
                '.' => {
                    self.consume();
                    let qualifier = self.parse_identifier()?;
                    suffixes.push(TypeSuffix::Qualifier(qualifier));
                }
                _ => break,
            }
        }

        let end = self.pos;
        let name = self.text[start..end].trim().to_string();
        let mut fn_sig = FnTypeExpr::new(abi, params, return_type);
        fn_sig.variadic = variadic;
        Some(TypeExpr {
            name,
            base: Vec::new(),
            suffixes,
            span: self.span_from_bounds(start, end),
            generic_span: None,
            tuple_elements: None,
            tuple_element_names: None,
            fn_signature: Some(fn_sig),
            trait_object: None,
            ref_kind: None,
            is_view: false,
        })
    }

    fn parse_fn_abi(&mut self) -> Option<FnTypeAbi> {
        self.skip_whitespace();
        if !self.consume_if('@') {
            return Some(FnTypeAbi::Chic);
        }

        let ident = self.parse_identifier()?;
        if ident.to_ascii_lowercase() != "extern" {
            return None;
        }

        self.skip_whitespace();
        if !self.consume_if('(') {
            return None;
        }

        self.skip_whitespace();
        let abi = self.parse_string_literal()?;
        self.skip_whitespace();

        if !self.consume_if(')') {
            return None;
        }

        Some(FnTypeAbi::Extern(abi))
    }

    fn parse_pointer_prefixes(&mut self) -> Option<Vec<TypeSuffix>> {
        let mut prefixes = Vec::new();
        loop {
            self.skip_whitespace();
            if !self.consume_if('*') {
                break;
            }
            let pointer = self.parse_pointer_suffix_data()?;
            prefixes.push(pointer);
        }
        Some(prefixes)
    }

    fn parse_pointer_suffix_data(&mut self) -> Option<TypeSuffix> {
        self.skip_whitespace();
        let mutable = self.consume_pointer_qualifier().unwrap_or(false);
        let modifiers = self.parse_pointer_modifiers()?;
        Some(TypeSuffix::Pointer { mutable, modifiers })
    }

    fn parse_pointer_modifiers(&mut self) -> Option<Vec<PointerModifier>> {
        let mut modifiers = Vec::new();
        loop {
            self.skip_whitespace();
            if !self.consume_if('@') {
                break;
            }
            let modifier = self.parse_pointer_modifier()?;
            modifiers.push(modifier);
        }
        Some(modifiers)
    }

    fn parse_pointer_modifier(&mut self) -> Option<PointerModifier> {
        let ident = self.parse_identifier()?;
        match ident.to_ascii_lowercase().as_str() {
            "restrict" => Some(PointerModifier::Restrict),
            "noalias" => Some(PointerModifier::NoAlias),
            "readonly" => Some(PointerModifier::ReadOnly),
            "expose_address" => Some(PointerModifier::ExposeAddress),
            "aligned" => {
                self.skip_whitespace();
                if !self.consume_if('(') {
                    return None;
                }
                let value = self.parse_uint_literal()?;
                self.skip_whitespace();
                if !self.consume_if(')') {
                    return None;
                }
                if value == 0 {
                    return None;
                }
                Some(PointerModifier::Aligned(value))
            }
            _ => None,
        }
    }

    fn parse_uint_literal(&mut self) -> Option<u32> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.consume();
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        self.text[start..self.pos].parse().ok()
    }

    fn consume_pointer_qualifier(&mut self) -> Option<bool> {
        if self.consume_keyword("mut") {
            return Some(true);
        }
        if self.consume_keyword("const") {
            return Some(false);
        }
        let remaining = &self.text[self.pos..];
        if remaining.len() >= 3 && remaining.starts_with("mut") {
            self.pos += 3;
            return Some(true);
        }
        if remaining.len() >= 5 && remaining.starts_with("const") {
            self.pos += 5;
            return Some(false);
        }
        None
    }

    fn parse_named_type(&mut self, start: usize) -> Option<TypeExpr> {
        let mut base = Vec::new();
        base.push(self.parse_identifier()?);

        loop {
            self.skip_whitespace();
            if self.consume_double_colon() || self.consume_if('.') {
                base.push(self.parse_identifier()?);
            } else {
                break;
            }
        }

        let mut suffixes = Vec::new();
        let mut seen_nullable = false;
        let mut generic_span = None;
        loop {
            self.skip_whitespace();
            if self.consume_double_colon() {
                let qualifier = self.parse_identifier()?;
                suffixes.push(TypeSuffix::Qualifier(qualifier));
                continue;
            }
            let Some(ch) = self.peek() else {
                break;
            };
            match ch {
                '<' => {
                    let list_start = self.pos;
                    self.consume();
                    let args = self.parse_generic_args()?;
                    generic_span = generic_span.or(self.span_from_bounds(list_start, self.pos));
                    suffixes.push(TypeSuffix::GenericArgs(args));
                }
                '[' => {
                    self.consume();
                    let rank = self.parse_array_rank()?;
                    suffixes.push(TypeSuffix::Array(rank));
                }
                '?' => {
                    self.consume();
                    if seen_nullable {
                        return None;
                    }
                    seen_nullable = true;
                    suffixes.push(TypeSuffix::Nullable);
                }
                '*' => {
                    self.consume();
                    let pointer = self.parse_pointer_suffix_data()?;
                    suffixes.push(pointer);
                }
                '.' => {
                    self.consume();
                    let qualifier = self.parse_identifier()?;
                    suffixes.push(TypeSuffix::Qualifier(qualifier));
                }
                _ => break,
            }
        }

        let end = self.pos;
        let name = self.text[start..end].trim().to_string();
        Some(TypeExpr {
            name,
            base,
            suffixes,
            span: self.span_from_bounds(start, end),
            generic_span,
            tuple_elements: None,
            tuple_element_names: None,
            fn_signature: None,
            trait_object: None,
            ref_kind: None,
            is_view: false,
        })
    }

    fn parse_ref_modifier(&mut self) -> Option<RefKind> {
        self.skip_whitespace();
        if self.consume_keyword("ref") {
            self.skip_whitespace();
            if self.consume_keyword("readonly") {
                return Some(RefKind::ReadOnly);
            }
            return Some(RefKind::Ref);
        }
        None
    }

    fn parse_generic_args(&mut self) -> Option<Vec<GenericArgument>> {
        let mut args = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume_if('>') {
                break;
            }
            let start = self.pos;
            let snapshot = self.pos;
            if let Some(mut arg_ty) = self.parse_type_expr() {
                let end = self.pos;
                let (trim_start, trim_end) = self.trim_bounds(start, end);
                if arg_ty.span.is_none() {
                    arg_ty.span = self.span_from_bounds(trim_start, trim_end);
                }
                let text = self.text[trim_start..trim_end].to_string();
                let expr_span = self.span_from_bounds(trim_start, trim_end);
                let expr = Expression::new(text, expr_span);
                args.push(GenericArgument::new(Some(arg_ty), expr));
            } else {
                self.pos = snapshot;
                let expr_start = self.pos;
                let mut expr = self.parse_const_generic_expression()?;
                let expr_end = self.pos;
                let (trim_start, trim_end) = self.trim_bounds(expr_start, expr_end);
                if expr.span.is_none() {
                    expr.span = self.span_from_bounds(trim_start, trim_end);
                }
                args.push(GenericArgument::new(None, expr));
            }
            self.skip_whitespace();
            if self.consume_if(',') {
                continue;
            }
            if self.consume_if('>') {
                break;
            }
            return None;
        }
        Some(args)
    }

    fn parse_const_generic_expression(&mut self) -> Option<Expression> {
        self.skip_whitespace();
        let start = self.pos;
        let mut depth_paren = 0usize;
        let mut depth_brace = 0usize;
        let mut depth_bracket = 0usize;
        let mut in_string = false;
        let mut string_delim = '\0';
        let mut consumed = false;

        while let Some(ch) = self.peek() {
            if in_string {
                self.consume();
                if ch == '\\' {
                    let _ = self.consume();
                } else if ch == string_delim {
                    in_string = false;
                }
                consumed = true;
                continue;
            }
            match ch {
                '"' | '\'' => {
                    string_delim = ch;
                    in_string = true;
                    self.consume();
                }
                '(' => {
                    depth_paren += 1;
                    self.consume();
                }
                ')' => {
                    if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                        break;
                    }
                    if depth_paren > 0 {
                        depth_paren -= 1;
                        self.consume();
                    } else {
                        break;
                    }
                }
                '{' => {
                    depth_brace += 1;
                    self.consume();
                }
                '}' => {
                    if depth_brace == 0 && depth_paren == 0 && depth_bracket == 0 {
                        break;
                    }
                    if depth_brace > 0 {
                        depth_brace -= 1;
                        self.consume();
                    } else {
                        break;
                    }
                }
                '[' => {
                    depth_bracket += 1;
                    self.consume();
                }
                ']' => {
                    if depth_bracket == 0 {
                        break;
                    }
                    depth_bracket -= 1;
                    self.consume();
                }
                ',' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => break,
                '>' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                    match self.peek_next_char() {
                        Some('=' | '>') => {
                            self.consume();
                            let _ = self.consume();
                            consumed = true;
                            continue;
                        }
                        _ => break,
                    }
                }
                _ => {
                    self.consume();
                }
            }
            consumed = true;
        }

        let end = self.pos;
        if !consumed {
            return None;
        }
        let (trim_start, trim_end) = self.trim_bounds(start, end);
        let text = self.text[trim_start..trim_end].trim();
        if text.is_empty() {
            return None;
        }
        Some(Expression::new(
            text.to_string(),
            self.span_from_bounds(trim_start, trim_end),
        ))
    }

    fn parse_array_rank(&mut self) -> Option<ArrayRankSpecifier> {
        self.skip_whitespace();
        let mut commas = 0usize;
        loop {
            match self.consume()? {
                ']' => break,
                ',' => commas += 1,
                ch if ch.is_whitespace() => {}
                _ => return None,
            }
        }
        Some(ArrayRankSpecifier::new(commas + 1))
    }

    fn parse_identifier(&mut self) -> Option<String> {
        self.skip_whitespace();
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if identifier::is_identifier_continue(ch) {
                value.push(ch);
                self.consume();
            } else {
                break;
            }
        }
        if value.is_empty() || !identifier::is_identifier_start(value.chars().next().unwrap()) {
            return None;
        }
        let status = identifier::analyse_identifier(&value);
        if status.disallowed.is_some() {
            return None;
        }
        Some(status.normalized)
    }

    fn parse_string_literal(&mut self) -> Option<String> {
        self.skip_whitespace();
        if !self.consume_if('"') {
            return None;
        }

        let mut value = String::new();
        while let Some(ch) = self.consume() {
            match ch {
                '"' => return Some(value),
                '\\' => {
                    let escaped = self.consume()?;
                    value.push(escaped);
                }
                other => value.push(other),
            }
        }
        None
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.consume();
            } else {
                break;
            }
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        self.skip_whitespace();
        if !self.text[self.pos..].starts_with(keyword) {
            return false;
        }
        let end = self.pos + keyword.len();
        if let Some(next) = self.char_at(end) {
            if next.is_alphanumeric() || next == '_' {
                return false;
            }
        }
        self.pos = end;
        true
    }

    fn consume_str(&mut self, expected: &str) -> bool {
        if self.text[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    fn char_at(&self, index: usize) -> Option<char> {
        if index >= self.text.len() {
            None
        } else {
            self.text[index..].chars().next()
        }
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.consume();
            true
        } else {
            false
        }
    }

    fn consume_double_colon(&mut self) -> bool {
        if self.peek() == Some(':') && self.peek_next_char() == Some(':') {
            self.consume();
            self.consume();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut iter = self.text[self.pos..].chars();
        iter.next()?;
        iter.next()
    }

    fn consume(&mut self) -> Option<char> {
        let mut iter = self.text[self.pos..].char_indices();
        let (_, ch) = iter.next()?;
        let next_pos = if let Some((offset, _)) = iter.next() {
            self.pos + offset
        } else {
            self.text.len()
        };
        self.pos = next_pos;
        Some(ch)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.text.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_type_expression_text_accepts_double_colon() {
        let expr = parse_type_expression_text("Std.InvalidOperationException")
            .expect("type expression should parse");
        assert_eq!(
            expr.base,
            vec!["Std".to_string(), "InvalidOperationException".to_string()]
        );
    }

    #[test]
    fn parse_type_expression_text_handles_pointer_modifiers() {
        let expr = parse_type_expression_text("*mut @restrict @aligned(8) @expose_address int")
            .expect("pointer expression should parse");
        assert_eq!(expr.pointer_depth(), 1);
        let suffix = expr.suffixes.last().expect("missing pointer suffix");
        let TypeSuffix::Pointer { modifiers, .. } = suffix else {
            panic!("expected pointer suffix with modifiers");
        };
        assert!(modifiers.contains(&PointerModifier::Restrict));
        assert!(
            modifiers
                .iter()
                .any(|modifier| matches!(modifier, PointerModifier::Aligned(8)))
        );
        assert!(modifiers.contains(&PointerModifier::ExposeAddress));
    }

    #[test]
    fn parse_type_expression_text_handles_multiple_pointer_layers() {
        let expr = parse_type_expression_text("*mut *mut char")
            .expect("double pointer expression should parse");
        assert_eq!(expr.pointer_depth(), 2);
        assert_eq!(expr.base, vec!["char".to_string()]);
    }
}
