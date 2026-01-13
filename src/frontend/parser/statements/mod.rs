//! Statement parsing dispatcher and shared helpers.
//!
//! Recovery helpers live in `recovery.rs`; upcoming tasks will focus on telemetry/tests.

use super::*;

mod exception;
mod labels;
mod local;
mod loops;
mod recovery;
mod region;
mod resource;
mod selection;
mod simple;

parser_impl! {
    pub(super) fn parse_statement(&mut self) -> Option<Statement> {
        if self.is_at_end() {
            return None;
        }

        let start_pos = self.peek().map(|token| token.span.start);
        let attrs = self.collect_attributes();

        if self.peek_local_function_declaration() {
            return self.parse_local_function_statement(start_pos, attrs);
        }

        if self.check_punctuation('{') {
            let mut statement = self.parse_block_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_punctuation(';') {
            let mut statement = self.parse_empty_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::If) {
            self.advance();
            let mut statement = self.parse_if_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::While) {
            self.advance();
            let mut statement = self.parse_while_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Do) {
            self.advance();
            let mut statement = self.parse_do_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::For) {
            self.advance();
            let mut statement = self.parse_for_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Foreach) {
            self.advance();
            let mut statement = self.parse_foreach_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Switch) {
            self.advance();
            let mut statement = self.parse_switch_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Try) {
            self.advance();
            let mut statement = self.parse_try_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Region) {
            self.advance();
            let mut statement = self.parse_region_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Using) {
            self.advance();
            let mut statement = self.parse_using_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Lock) {
            self.advance();
            let mut statement = self.parse_lock_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Atomic) {
            self.advance();
            let mut statement = self.parse_atomic_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Checked) {
            self.advance();
            let mut statement = self.parse_checked_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Unchecked) {
            self.advance();
            let mut statement = self.parse_unchecked_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Yield) {
            self.advance();
            let mut statement = self.parse_yield_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Goto) {
            self.advance();
            let mut statement = self.parse_goto_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Return) {
            self.advance();
            let mut statement = self.parse_return_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Break) {
            self.advance();
            let mut statement = self.parse_break_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Continue) {
            self.advance();
            let mut statement = self.parse_continue_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Throw) {
            self.advance();
            let mut statement = self.parse_throw_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Fixed) {
            self.advance();
            let mut statement = self.parse_fixed_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if self.check_keyword(Keyword::Unsafe) {
            self.advance();
            let mut statement = self.parse_unsafe_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        // LL1_ALLOW: Labeled statements are detected via an identifier followed by `:`, so we peek one token ahead to keep the rest of the statement grammar LL(1) (docs/compiler/parser.md#ll1-allowances).
        if matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Identifier,
                ..
            })
        ) && {
            matches!(
                // LL1_ALLOW: Labeled statements are detected via an identifier followed by `:`, so we peek one token ahead to keep the rest of the statement grammar LL(1) (docs/compiler/parser.md#ll1-allowances).
                self.peek_n(1),
                Some(Token {
                    kind: TokenKind::Punctuation(':'),
                    ..
                })
            )
        } {
            let mut statement = self.parse_labeled_statement(start_pos)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        if let Some(kind) = self.detect_local_declaration() {
            let mut statement = self.parse_variable_declaration_statement(start_pos, kind)?;
            self.apply_statement_attributes(&mut statement, attrs);
            return Some(statement);
        }

        let mut statement = self.parse_expression_statement(start_pos)?;
        self.apply_statement_attributes(&mut statement, attrs);
        Some(statement)
    }
}
