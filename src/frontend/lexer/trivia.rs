use super::TokenKind;
use super::state::Lexer;
use crate::frontend::diagnostics::Span;

impl<'a> Lexer<'a> {
    pub(super) fn consume_whitespace(&mut self) {
        let Some((mut end, ch)) = self.lookahead else {
            return;
        };
        if !ch.is_ascii_whitespace() {
            return;
        }
        let start = end;
        end += ch.len_utf8();
        self.bump();
        while let Some((idx, ch)) = self.lookahead {
            if ch.is_ascii_whitespace() {
                end = idx + ch.len_utf8();
                self.bump();
            } else {
                break;
            }
        }

        self.tokens.push(super::Token {
            kind: TokenKind::Whitespace,
            lexeme: self.slice(start, end).to_string(),
            span: Span::in_file(self.file_id, start, end),
        });
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Lexing comment forms requires multiple branches for tokens"
    )]
    pub(super) fn consume_slash(&mut self, start: usize) {
        match self.peek_next_char() {
            Some((_, '/')) => {
                self.bump(); // consume '/'
                self.bump(); // consume second '/'
                let mut end = start + 2;
                let mut is_doc = false;
                if let Some((idx, '/')) = self.lookahead {
                    is_doc = true;
                    end = idx + '/'.len_utf8();
                    self.bump(); // consume third '/'
                }
                while let Some((idx, ch)) = self.lookahead {
                    if ch == '\n' {
                        break;
                    }
                    end = idx + ch.len_utf8();
                    self.bump();
                }
                let kind = if is_doc {
                    TokenKind::DocComment
                } else {
                    TokenKind::Comment
                };
                self.emit(start, end, kind);
            }
            Some((_, '*')) => {
                self.bump(); // consume '/'
                self.bump(); // consume '*'
                let mut depth = 1usize;
                let mut end = start + 2;
                let mut last_char = '\0';
                while let Some((idx, ch)) = self.lookahead {
                    end = idx + ch.len_utf8();
                    self.bump();
                    if last_char == '/' && ch == '*' {
                        depth += 1;
                    } else if last_char == '*' && ch == '/' {
                        depth -= 1;
                        if depth == 0 {
                            self.emit(start, end, TokenKind::Comment);
                            return;
                        }
                    }
                    last_char = ch;
                }
                super::diagnostics::report_simple_error(
                    self,
                    "unterminated block comment",
                    Span::new(start, end),
                );
                self.emit(start, end, TokenKind::Comment);
            }
            Some((_, '=')) => {
                self.bump();
                self.bump();
                let end = start + 2;
                self.emit(start, end, TokenKind::Operator("/="));
            }
            _ => {
                self.emit_single_char_token(start, '/', TokenKind::Operator("/"));
                self.bump();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> super::super::LexOutput {
        super::super::lex(source)
    }

    #[test]
    fn captures_whitespace_as_tokens() {
        let output = lex("foo   bar");
        assert!(
            output
                .tokens
                .iter()
                .any(|t| matches!(t.kind, TokenKind::Whitespace))
        );
    }

    #[test]
    fn recognises_doc_comment_trivia() {
        let output = lex("/// docs\nlet value = 0;");
        assert!(matches!(
            output.tokens.first().map(|t| &t.kind),
            Some(TokenKind::DocComment)
        ));
    }
}
