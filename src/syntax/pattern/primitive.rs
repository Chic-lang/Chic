//! Primitive pattern parsing helpers: wildcard, literals, bindings, and dispatch to
//! composite/guard logic. Invariants:
//! - `try_parse_literal` only consumes tokens when a literal is recognised.
//! - `parse_primary_pattern` delegates composites/guards while maintaining the
//!   original precedence ordering from the monolithic parser.

use super::*;
use crate::mir::FloatValue;

impl PatternParser {
    pub(super) fn parse_primary_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        if let Some(binding) = self.try_parse_binding_pattern()? {
            return Ok(binding);
        }

        if self.consume_identifier("_") {
            return Ok(PatternNode::Wildcard);
        }

        if let Some(literal) = self.try_parse_literal()? {
            return Ok(PatternNode::Literal(literal));
        }

        if self.peek_punctuation('(') {
            return self.parse_tuple_pattern();
        }

        if self.peek_punctuation('[') {
            return self.parse_list_pattern();
        }

        if self.peek_punctuation('{') {
            return self.parse_record_pattern(None);
        }

        if self.peek_relational_operator() {
            return self.parse_relational_pattern();
        }

        self.parse_path_pattern()
    }

    fn try_parse_binding_pattern(&mut self) -> Result<Option<PatternNode>, PatternParseError> {
        let save = self.index;
        let prefix_mode = self.consume_binding_modifier()?;
        let mutability = if self.consume_keyword(Keyword::Let) {
            Some(PatternBindingMutability::Immutable)
        } else if self.consume_keyword(Keyword::Var) {
            Some(PatternBindingMutability::Mutable)
        } else {
            self.index = save;
            return Ok(None);
        };

        let (name, span) = self.parse_spanned_identifier("binding")?;
        let suffix_mode = self.consume_binding_modifier()?;

        let mode = match (prefix_mode, suffix_mode) {
            (Some(left), Some(right)) if left != right => {
                return Err(
                    self.error("binding modifiers before and after the identifier must match")
                );
            }
            (Some(binding_mode), _) | (_, Some(binding_mode)) => binding_mode,
            (None, None) => PatternBindingMode::Value,
        };

        if matches!(mode, PatternBindingMode::Ref)
            && matches!(mutability, Some(PatternBindingMutability::Immutable))
        {
            return Err(self.error("`let` bindings cannot use `ref` (mutable) patterns"));
        }

        let Some(mutability) = mutability else {
            self.index = save;
            return Ok(None);
        };

        self.metadata.bindings.push(PatternBindingMetadata {
            name: name.clone(),
            span,
        });

        Ok(Some(PatternNode::Binding(BindingPatternNode {
            name,
            mutability,
            mode,
            span,
        })))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn try_parse_literal(&mut self) -> Result<Option<ConstValue>, PatternParseError> {
        if let Some(token) = self.peek() {
            match &token.kind {
                TokenKind::Operator("-") => {
                    let save = self.index;
                    self.advance();
                    if let Some(next) = self.peek()
                        && matches!(&next.kind, TokenKind::NumberLiteral(_))
                    {
                        let number =
                            self.advance_or_error("expected numeric literal after prefix")?;
                        if let TokenKind::NumberLiteral(ref literal) = number.kind {
                            if let Some(value) =
                                parse_number_literal(literal).and_then(|value| value.checked_neg())
                            {
                                return Ok(Some(ConstValue::Int(value)));
                            }
                            if let Some(value) = parse_float_literal(literal) {
                                return Ok(Some(ConstValue::Float(FloatValue::from_f64(-value))));
                            }
                        }
                        return Err(self.error("invalid numeric literal"));
                    }
                    self.index = save;
                }
                TokenKind::NumberLiteral(_) => {
                    let number = self.advance_or_error("expected numeric literal")?;
                    let value = if let TokenKind::NumberLiteral(ref literal) = number.kind {
                        parse_number_literal(literal)
                            .map(ConstValue::Int)
                            .or_else(|| parse_unsigned_literal(literal).map(ConstValue::UInt))
                            .or_else(|| {
                                parse_float_literal(literal)
                                    .map(FloatValue::from_f64)
                                    .map(ConstValue::Float)
                            })
                    } else {
                        None
                    }
                    .ok_or_else(|| self.error("invalid numeric literal"))?;
                    return Ok(Some(value));
                }
                TokenKind::StringLiteral(_) => {
                    let string = self.advance_or_error("expected string literal")?;
                    if let TokenKind::StringLiteral(literal) = string.kind {
                        return match literal.contents {
                            StringLiteralContents::Simple(text) => {
                                Ok(Some(ConstValue::RawStr(text)))
                            }
                            StringLiteralContents::Interpolated(segments) => {
                                let mut buffer = String::new();
                                for segment in segments {
                                    match segment {
                                        StringSegment::Text(text) => buffer.push_str(&text),
                                        StringSegment::Interpolation(_) => {
                                            return Err(self.error(
                                                "interpolated string literals are not supported in patterns",
                                            ));
                                        }
                                    }
                                }
                                Ok(Some(ConstValue::RawStr(buffer)))
                            }
                        };
                    }
                    unreachable!("advance_or_error returned mismatched token kind");
                }
                TokenKind::CharLiteral(_) => {
                    let ch = self.advance_or_error("expected character literal")?;
                    if let TokenKind::CharLiteral(literal) = ch.kind {
                        return Ok(Some(ConstValue::Char(literal.value)));
                    }
                    unreachable!("advance_or_error returned mismatched token kind");
                }
                TokenKind::Identifier if token.lexeme == "true" => {
                    self.advance();
                    return Ok(Some(ConstValue::Bool(true)));
                }
                TokenKind::Identifier if token.lexeme == "false" => {
                    self.advance();
                    return Ok(Some(ConstValue::Bool(false)));
                }
                TokenKind::Identifier if token.lexeme == "null" => {
                    self.advance();
                    return Ok(Some(ConstValue::Null));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn consume_binding_modifier(
        &mut self,
    ) -> Result<Option<PatternBindingMode>, PatternParseError> {
        if self.consume_identifier_ci("move") {
            return Ok(Some(PatternBindingMode::Move));
        }

        if let Some(token) = self.peek() {
            if let TokenKind::Keyword(keyword) = token.kind {
                match keyword {
                    Keyword::Ref => {
                        self.advance();
                        if self.consume_keyword(Keyword::Readonly) {
                            return Ok(Some(PatternBindingMode::RefReadonly));
                        }
                        return Ok(Some(PatternBindingMode::Ref));
                    }
                    Keyword::In => {
                        self.advance();
                        return Ok(Some(PatternBindingMode::In));
                    }
                    Keyword::Readonly => {
                        return Err(self.error(
                            "`readonly` modifier is only supported on ref parameters and receivers",
                        ));
                    }
                    Keyword::Out => {
                        return Err(self.error(
                            "`out` qualifier is only supported on parameters and receivers",
                        ));
                    }
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    fn consume_identifier_ci(&mut self, expected: &str) -> bool {
        if let Some(token) = self.peek() {
            match &token.kind {
                TokenKind::Identifier | TokenKind::Keyword(_) => {
                    if token.lexeme.eq_ignore_ascii_case(expected) {
                        self.advance();
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}
