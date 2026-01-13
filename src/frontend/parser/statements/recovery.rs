//! Statement recovery helpers (embedded statements, loop initializers, expression sequencing, resource/case recovery, catch filters, synchronization).
//! Recovery telemetry hooks are recorded opportunistically here; callers opt-in via
//! `recovery_telemetry` to capture the synchronized token/statement boundaries for diagnostics.

use super::*;
use crate::frontend::ast::{CasePattern, PatternGuard, SwitchCaseLabel, SwitchLabel};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::Token;
use crate::frontend::parser::RecoveryTelemetryKind;
use crate::syntax::pattern::parse_pattern;

parser_impl! {
    pub(in crate::frontend::parser) fn parse_embedded_statement(
        &mut self,
    ) -> Option<Statement> {
        let token = self.peek().cloned();
        if let Some(telemetry) = self.recovery_telemetry.as_mut() {
            telemetry.record(RecoveryTelemetryKind::EmbeddedStatement, token.as_ref());
        }
        if self.check_punctuation('{') {
            let block = self.parse_block()?;
            let span = block.span;
            Some(Statement::new(span, StatementKind::Block(block)))
        } else {
            self.parse_statement()
        }
    }

    pub(in crate::frontend::parser) fn synchronize_statement(&mut self) {
        let mut last_token: Option<Token> = None;
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation(';') => {
                    last_token = Some(token.clone());
                    self.advance();
                    break;
                }
                TokenKind::Punctuation('}')
                | TokenKind::Keyword(
                    Keyword::If
                    | Keyword::For
                    | Keyword::Foreach
                    | Keyword::While
                    | Keyword::Switch
                    | Keyword::Try
                    | Keyword::Return
                    | Keyword::Break
                    | Keyword::Continue
                    | Keyword::Throw
                    | Keyword::Using
                    | Keyword::Lock
                    | Keyword::Atomic
                    | Keyword::Checked
                    | Keyword::Unchecked
                    | Keyword::Yield
                    | Keyword::Goto
                    | Keyword::Do
                    | Keyword::Fixed
                    | Keyword::Unsafe
                    | Keyword::Function,
                ) => {
                    last_token = Some(token.clone());
                    break;
                }
                _ => {
                    last_token = Some(token.clone());
                    self.advance();
                }
            }
        }
        if let Some(telemetry) = self.recovery_telemetry.as_mut() {
            telemetry.record(RecoveryTelemetryKind::Synchronize, last_token.as_ref());
        }
    }

    /// Parses the initializer segment of a `for` statement, consuming the trailing `;`.
    ///
    /// Invariants:
    /// - Validates attribute usage (`@pin` only on declarations) via `report_attribute_misuse`.
    /// - Preserves span construction by delegating to `make_span` with the original start index.
    /// - Returns `None` for parser failure; `Some(None)` when the initializer section is empty.
    pub(in crate::frontend::parser) fn parse_for_initializer(&mut self) -> Option<Option<ForInitializer>> {
        if self.check_punctuation(';') {
            self.advance();
            return Some(None);
        }

        let attrs = self.collect_attributes();
        if let Some(kind) = self.detect_local_declaration() {
            let decl_start = self.peek().map(|token| token.span.start);
            match kind {
                LocalDeclStart::Const => {
                    if attrs.builtin.pin {
                        self.report_attribute_misuse(
                            attrs,
                            "`@pin` attribute is only supported on variable declarations",
                        );
                    }
                    self.match_keyword(Keyword::Const);
                    let mut declaration = self.parse_const_declaration_body(None, ';')?;
                    if !self.expect_punctuation(';') {
                        return None;
                    }
                    let span = self.make_span(decl_start);
                    declaration.span = span;
                    return Some(Some(ForInitializer::Const(ConstStatement { declaration })));
                }
                LocalDeclStart::Typed {
                    ty,
                    ty_start,
                    name_index,
                } => {
                    let mut declaration =
                        self.parse_rejected_typed_local(ty, ty_start, name_index, ';', false)?;
                    if attrs.builtin.pin {
                        declaration.is_pinned = true;
                    }
                    return Some(Some(ForInitializer::Declaration(declaration)));
                }
                other => {
                    let mut declaration =
                        self.parse_variable_declaration_with_kind(other, ';', false)?;
                    if attrs.builtin.pin {
                        declaration.is_pinned = true;
                    }
                    if !self.expect_punctuation(';') {
                        return None;
                    }
                    return Some(Some(ForInitializer::Declaration(declaration)));
                }
            }
        }

        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "`@pin` attribute is only supported on variable declarations",
            );
        }

        let expressions = self.collect_expression_list_until(';');
        if !self.expect_punctuation(';') {
            return None;
        }
        if expressions.is_empty() {
            Some(None)
        } else {
            Some(Some(ForInitializer::Expressions(expressions)))
        }
    }

    /// Collects the iteration expression list in a `for` statement without consuming the closing `)`.
    ///
    /// The helper preserves the previous recovery behaviour by delegating to
    /// `collect_expression_list_until(')')` once the trailing `;` has been consumed.
    pub(in crate::frontend::parser) fn parse_iteration_list(&mut self) -> Vec<Expression> {
        if self.check_punctuation(')') {
            Vec::new()
        } else {
            self.collect_expression_list(')')
        }
    }

    /// Collects expressions separated by commas until `terminator` without consuming the terminator.
    ///
    /// This wrapper keeps expression list recovery centralised inside the statements layer while
    /// reusing the lower-level cursor implementation.
    pub(in crate::frontend::parser) fn collect_expression_list(&mut self, terminator: char) -> Vec<Expression> {
        self.collect_expression_list_until(terminator)
    }

    /// Parses an expression terminated by `terminator`, consuming the punctuation.
    ///
    /// The helper preserves expression text verbatim (including comma operators) and guarantees that
    /// the terminator is consumed so callers do not duplicate punctuation handling.
    pub(in crate::frontend::parser) fn parse_expression_until(&mut self, terminator: char) -> Option<Expression> {
        let expr = self.collect_expression_until(&[terminator]);
        if !self.expect_punctuation(terminator) {
            return None;
        }
        Some(expr)
    }

    /// Parses a pattern expression terminated by `terminator`, consuming the punctuation.
    ///
    /// Useful for statements that admit pattern expressions (e.g., `yield return`, `using` without
    /// parentheses) while keeping recovery semantics identical to the standalone helper.
    pub(in crate::frontend::parser) fn parse_pattern_expression_until(
        &mut self,
        terminator: char,
    ) -> Option<Expression> {
        let expr = self.collect_pattern_expression_until(&[terminator]);
        if !self.expect_punctuation(terminator) {
            return None;
        }
        Some(expr)
    }

    /// Parses the resource portion of a `using` statement, validating collected attributes and declarations.
    ///
    /// Invariants:
    /// - Local declarations honour `@pin` by toggling `is_pinned` and require initialisers.
    /// - Attribute misuse on expression resources reports diagnostics via `report_attribute_misuse`.
    /// - The terminator token is **not** consumed; callers remain responsible for punctuation checks.
    pub(in crate::frontend::parser) fn parse_using_resource(
        &mut self,
        attrs: CollectedAttributes,
        terminator: char,
        allow_pattern_expression: bool,
    ) -> Option<UsingResource> {
        if let Some(kind) = self.detect_local_declaration() {
            let mut decl =
                self.parse_variable_declaration_with_kind(kind, terminator, true)?;
            if attrs.builtin.pin {
                decl.is_pinned = true;
            }
            return Some(UsingResource::Declaration(decl));
        }

        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "`@pin` attribute is only supported on variable declarations",
            );
        }

        let expr = if allow_pattern_expression {
            self.collect_pattern_expression_until(&[terminator])
        } else {
            self.collect_expression_until(&[terminator])
        };
        Some(UsingResource::Expression(expr))
    }

    /// Collects a `case` pattern and zero or more `when` guards without consuming the terminator.
    pub(in crate::frontend::parser) fn collect_case_pattern_and_guards(
        &mut self,
        terminators: &[char],
    ) -> (CasePattern, Vec<PatternGuard>) {
        let (text, span) = self.collect_expression_bounds(terminators);
        let trimmed = trim_case_pattern_text(&text);
        if trimmed.trimmed.is_empty() {
            let raw = Expression::new(String::new(), span);
            return (CasePattern::new(raw, None), Vec::new());
        }

        let (pattern_slice, guard_slices) = split_pattern_and_guard_slices(trimmed.trimmed);
        let pattern = self.build_case_pattern(span, &trimmed, pattern_slice);
        let guards = guard_slices
            .into_iter()
            .filter_map(|slice| self.build_pattern_guard(span, &trimmed, slice))
            .collect();
        (pattern, guards)
    }

    fn build_case_pattern(
        &mut self,
        span: Option<Span>,
        trimmed: &TrimmedCaseText<'_>,
        slice: TextSlice,
    ) -> CasePattern {
        let segment = trimmed.slice(slice);
        let Some((inner_start, inner_end)) = slice_trim_bounds(segment) else {
            self.push_error("expected pattern", span);
            let raw = Expression::new(String::new(), span);
            return CasePattern::new(raw, None);
        };
        let pattern_text = segment[inner_start..inner_end].to_string();
        let pattern_span = resolve_case_span(span, trimmed, slice.start + inner_start, pattern_text.len());
        let raw = Expression::new(pattern_text.clone(), pattern_span);
        let ast = match parse_pattern(&pattern_text, pattern_span) {
            Ok(ast) => Some(ast),
            Err(err) => {
                self.push_error(err.message, err.span.or(pattern_span).or(span));
                None
            }
        };
        CasePattern::new(raw, ast)
    }

    fn build_pattern_guard(
        &mut self,
        span: Option<Span>,
        trimmed: &TrimmedCaseText<'_>,
        slice: GuardSlice,
    ) -> Option<PatternGuard> {
        let segment = trimmed.slice(slice.body);
        let Some((inner_start, inner_end)) = slice_trim_bounds(segment) else {
            let keyword_span = resolve_case_span(span, trimmed, slice.keyword_start, "when".len());
            self.push_error("`when` guard requires an expression", keyword_span.or(span));
            return None;
        };
        let guard_start = slice.body.start + inner_start;
        let guard_len = inner_end - inner_start;
        let guard_span = resolve_case_span(span, trimmed, guard_start, guard_len);
        let guard_text = segment[inner_start..inner_end].to_string();
        let expression = self.build_expression(guard_text, guard_span);
        let keyword_span = resolve_case_span(span, trimmed, slice.keyword_start, "when".len());
        Some(PatternGuard {
            expression,
            depth: slice.depth,
            keyword_span,
        })
    }

    /// Parses a single switch label, returning `Ok(None)` when no label is present.
    ///
    /// Callers should invoke `synchronize_statement` after an `Err` to recover before
    /// continuing section parsing.
    pub(in crate::frontend::parser) fn parse_switch_label(
        &mut self,
    ) -> Result<Option<SwitchLabel>, ()> {
        if self.match_keyword(Keyword::Case) {
            let (pattern, guards) = self.collect_case_pattern_and_guards(&[':']);
            if !self.expect_punctuation(':') {
                return Err(());
            }
            Ok(Some(SwitchLabel::Case(SwitchCaseLabel { pattern, guards })))
        } else if self.match_keyword(Keyword::Default) {
            if !self.expect_punctuation(':') {
                return Err(());
            }
            Ok(Some(SwitchLabel::Default))
        } else {
            Ok(None)
        }
    }

    /// Parses an optional `when (...)` catch filter.
    ///
    /// Returns `Ok(None)` when no filter is present and `Err(())` when recovery should
    /// propagate to the caller (e.g., missing parentheses).
    pub(in crate::frontend::parser) fn parse_catch_filter(
        &mut self,
    ) -> Result<Option<Expression>, ()> {
        if !self.match_keyword(Keyword::When) {
            return Ok(None);
        }
        match self.parse_parenthesized_expression("catch filter") {
            Some(expr) => Ok(Some(expr)),
            None => Err(()),
        }
    }
}

#[derive(Default)]
struct CaseGuardDepths {
    paren: i32,
    brace: i32,
    bracket: i32,
}

impl CaseGuardDepths {
    fn update(&mut self, ch: char) {
        match ch {
            '(' => self.paren += 1,
            ')' => self.paren -= 1,
            '{' => self.brace += 1,
            '}' => self.brace -= 1,
            '[' => self.bracket += 1,
            ']' => self.bracket -= 1,
            _ => {}
        }
    }

    const fn at_surface(&self) -> bool {
        self.paren == 0 && self.brace == 0 && self.bracket == 0
    }
}

#[derive(Clone, Copy)]
struct TextSlice {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct GuardSlice {
    keyword_start: usize,
    body: TextSlice,
    depth: usize,
}

struct TrimmedCaseText<'a> {
    trimmed: &'a str,
    start_offset: usize,
}

impl<'a> TrimmedCaseText<'a> {
    fn new(text: &'a str) -> Self {
        let trimmed_start = text.trim_start();
        let leading_ws = text.len() - trimmed_start.len();
        let trimmed_end = trimmed_start.trim_end();
        let trailing_ws = trimmed_start.len() - trimmed_end.len();
        let start = leading_ws;
        let end = text.len() - trailing_ws;
        Self {
            trimmed: if start < end { &text[start..end] } else { "" },
            start_offset: start,
        }
    }

    fn slice(&self, slice: TextSlice) -> &'a str {
        if self.trimmed.is_empty() {
            return "";
        }
        &self.trimmed[slice.start.min(self.trimmed.len())..slice.end.min(self.trimmed.len())]
    }
}

fn trim_case_pattern_text(text: &str) -> TrimmedCaseText<'_> {
    TrimmedCaseText::new(text)
}

fn split_pattern_and_guard_slices(trimmed: &str) -> (TextSlice, Vec<GuardSlice>) {
    let guard_indices = find_case_guard_indices(trimmed);
    let mut guards = Vec::with_capacity(guard_indices.len());
    for (idx, keyword_start) in guard_indices.iter().copied().enumerate() {
        let body_start = keyword_start.saturating_add(4);
        let body_end = guard_indices
            .get(idx + 1)
            .copied()
            .unwrap_or_else(|| trimmed.len());
        guards.push(GuardSlice {
            keyword_start,
            body: TextSlice {
                start: body_start.min(trimmed.len()),
                end: body_end.min(trimmed.len()),
            },
            depth: idx,
        });
    }

    let pattern_end = guard_indices
        .first()
        .copied()
        .unwrap_or_else(|| trimmed.len());
    (
        TextSlice {
            start: 0,
            end: pattern_end.min(trimmed.len()),
        },
        guards,
    )
}

fn slice_trim_bounds(segment: &str) -> Option<(usize, usize)> {
    let trimmed_start = segment.trim_start();
    let leading_ws = segment.len() - trimmed_start.len();
    let trimmed_end = trimmed_start.trim_end();
    let trimmed_len = trimmed_end.len();
    if trimmed_len == 0 {
        None
    } else {
        Some((leading_ws, leading_ws + trimmed_len))
    }
}

fn resolve_case_span(
    span: Option<Span>,
    trimmed: &TrimmedCaseText<'_>,
    relative_start: usize,
    len: usize,
) -> Option<Span> {
    span.map(|sp| {
        let start = sp.start + trimmed.start_offset + relative_start;
        Span::in_file(sp.file_id, start, start + len)
    })
}

fn find_case_guard_indices(text: &str) -> Vec<usize> {
    let mut depths = CaseGuardDepths::default();
    let mut indices = Vec::new();
    for (idx, ch) in text.char_indices() {
        depths.update(ch);
        if depths.at_surface() && text[idx..].starts_with("when") && guard_has_boundaries(text, idx)
        {
            indices.push(idx);
        }
    }
    indices
}

fn guard_has_boundaries(text: &str, idx: usize) -> bool {
    let before = if idx == 0 {
        None
    } else {
        text[..idx].chars().next_back()
    };
    let after_start = idx + 4;
    let after = text.get(after_start..).and_then(|rest| rest.chars().next());

    let boundary_before = before.is_none_or(char::is_whitespace);
    let boundary_after = after.is_none_or(|c| c.is_whitespace() || c == '(');
    boundary_before && boundary_after
}

#[allow(dead_code)]
/// Marker constant documenting forthcoming recovery extraction work.
pub(super) const RECOVERY_NOTE: &str =
    "Recovery diagnostics are emitted via telemetry hooks; future extraction logic lives here.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_placeholder_documented() {
        assert!(!RECOVERY_NOTE.is_empty());
    }
}
