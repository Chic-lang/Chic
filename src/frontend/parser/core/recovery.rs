use super::*;

parser_impl! {
    pub(in crate::frontend::parser) fn skip_expression_until_semicolon(&mut self) {
        let mut paren_depth = 0usize;
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    paren_depth += 1;
                    self.advance();
                }
                TokenKind::Punctuation(')') => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                    self.advance();
                }
                TokenKind::Punctuation(';') if paren_depth == 0 => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub(in crate::frontend::parser) fn synchronize_item(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(';' | '}') => {
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub(in crate::frontend::parser) fn synchronize_class_member(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(';' | '}') => break,
                _ => {
                    self.advance();
                }
            }
        }
        if self.check_punctuation(';') {
            self.advance();
        }
    }

    pub(in crate::frontend::parser) fn synchronize_field(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(';' | '}') => break,
                _ => {
                    self.advance();
                }
            }
        }
        if self.check_punctuation(';') {
            self.advance();
        }
    }

    pub(in crate::frontend::parser) fn synchronize_variant(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(',' | '}') => break,
                _ => {
                    self.advance();
                }
            }
        }
        if self.check_punctuation(',') {
            self.advance();
        }
    }

    pub(in crate::frontend::parser) fn synchronize_parameter(&mut self) {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(',' | ')') => break,
                _ => {
                    self.advance();
                }
            }
        }
    }
}
