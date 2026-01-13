//! Pattern parsing entry point split into primitive, composite, and guard
//! helpers. The submodules expose focused parsing routines while this module
//! retains shared types, token utilities, and error reporting.

use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, NumericLiteral, NumericLiteralKind, Token, TokenKind, lex};
use crate::frontend::literals::{StringLiteralContents, StringSegment};
use crate::mir::{ConstValue, PatternBindingMode, PatternBindingMutability};
use crate::syntax::numeric;

mod composite;
mod guard;
mod primitive;

#[derive(Debug, Clone)]
pub struct PatternAst {
    pub node: PatternNode,
    pub span: Option<Span>,
    pub metadata: PatternMetadata,
}

#[derive(Debug, Clone)]
pub enum PatternNode {
    Wildcard,
    Literal(ConstValue),
    Binding(BindingPatternNode),
    Tuple(Vec<PatternNode>),
    Struct {
        path: Vec<String>,
        fields: Vec<PatternFieldNode>,
    },
    Enum {
        path: Vec<String>,
        variant: String,
        fields: VariantPatternFieldsNode,
    },
    Positional {
        path: Vec<String>,
        elements: Vec<PatternNode>,
    },
    Type {
        path: Vec<String>,
        subpattern: Option<Box<PatternNode>>,
    },
    Relational {
        op: RelationalOp,
        expr: PatternExpression,
    },
    Binary {
        op: PatternBinaryOp,
        left: Box<PatternNode>,
        right: Box<PatternNode>,
    },
    Not(Box<PatternNode>),
    List(ListPatternNode),
    Record(RecordPatternNode),
}

#[derive(Debug, Clone)]
pub struct BindingPatternNode {
    pub name: String,
    pub mutability: PatternBindingMutability,
    pub mode: PatternBindingMode,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct PatternFieldNode {
    pub name: String,
    pub pattern: PatternNode,
    pub span: Option<Span>,
    pub name_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub enum VariantPatternFieldsNode {
    Unit,
    Tuple(Vec<PatternNode>),
    Struct(Vec<PatternFieldNode>),
}

#[derive(Debug, Clone)]
pub struct ListPatternNode {
    pub prefix: Vec<PatternNode>,
    pub slice: Option<Box<PatternNode>>,
    pub suffix: Vec<PatternNode>,
    pub span: Option<Span>,
    pub slice_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct RecordPatternNode {
    pub path: Option<Vec<String>>,
    pub fields: Vec<PatternFieldNode>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationalOp {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternBinaryOp {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct PatternExpression {
    pub text: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Default)]
pub struct PatternMetadata {
    pub bindings: Vec<PatternBindingMetadata>,
    pub list_slices: Vec<ListSliceMetadata>,
    pub record_fields: Vec<RecordFieldMetadata>,
}

#[derive(Debug, Clone)]
pub struct PatternBindingMetadata {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ListSliceMetadata {
    pub span: Option<Span>,
    pub binding: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordFieldMetadata {
    pub name: String,
    pub name_span: Option<Span>,
    pub pattern_span: Option<Span>,
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct PatternParseError {
    pub message: String,
    pub span: Option<Span>,
}

/// Parse a pattern expression from a source string.
///
/// # Errors
/// Returns an error when the source contains tokens that do not form a valid pattern.
pub fn parse_pattern(source: &str, span: Option<Span>) -> Result<PatternAst, PatternParseError> {
    let mut parser = PatternParser::new(source, span);
    let pattern = parser.parse_or_pattern()?;
    parser.expect_end()?;
    let span = parser.span_from_range(0, parser.index);
    Ok(PatternAst {
        node: pattern,
        span,
        metadata: parser.metadata,
    })
}

/// Parse a pattern prefix from pre-tokenised input.
///
/// # Errors
/// Returns an error when the token sequence does not represent a valid leading pattern.
pub fn parse_pattern_prefix(
    tokens: &[Token],
    source: &str,
) -> Result<(PatternAst, usize), PatternParseError> {
    let mut parser = PatternParser::from_tokens(tokens, source);
    let pattern = parser.parse_or_pattern()?;
    let consumed = parser.index;
    if consumed == 0 {
        return Err(parser.error("expected pattern"));
    }
    let span = parser.span_from_range(0, consumed);
    Ok((
        PatternAst {
            node: pattern,
            span,
            metadata: parser.metadata,
        },
        consumed,
    ))
}

pub(super) struct PatternParser {
    tokens: Vec<Token>,
    index: usize,
    span: Option<Span>,
    source: String,
    base_offset: usize,
    pub(super) metadata: PatternMetadata,
}

#[derive(Default)]
pub(super) struct DelimiterDepth {
    paren: usize,
    brace: usize,
    bracket: usize,
}

impl DelimiterDepth {
    pub(super) fn is_top_level(&self) -> bool {
        self.paren == 0 && self.brace == 0 && self.bracket == 0
    }

    pub(super) fn should_stop(&self, token: &Token) -> bool {
        match token.kind {
            TokenKind::Punctuation(')') => self.paren == 0,
            TokenKind::Punctuation('}') => self.brace == 0,
            TokenKind::Punctuation(']') => self.bracket == 0,
            TokenKind::Punctuation(',') | TokenKind::Keyword(Keyword::And | Keyword::Or)
                if self.is_top_level() =>
            {
                true
            }
            TokenKind::Operator(ref op) if self.is_top_level() && *op == "=>" => true,
            _ => false,
        }
    }

    pub(super) fn record(&mut self, token: &Token) {
        match token.kind {
            TokenKind::Punctuation('(') => self.paren += 1,
            TokenKind::Punctuation(')') => {
                self.paren = self.paren.saturating_sub(1);
            }
            TokenKind::Punctuation('{') => self.brace += 1,
            TokenKind::Punctuation('}') => {
                self.brace = self.brace.saturating_sub(1);
            }
            TokenKind::Punctuation('[') => self.bracket += 1,
            TokenKind::Punctuation(']') => {
                self.bracket = self.bracket.saturating_sub(1);
            }
            _ => {}
        }
    }
}

impl PatternParser {
    fn new(source: &str, span: Option<Span>) -> Self {
        let lexed = lex(source);
        let tokens = lexed
            .tokens
            .into_iter()
            .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
            .collect();
        Self {
            tokens,
            index: 0,
            span,
            source: source.to_string(),
            base_offset: span.map_or(0, |s| s.start),
            metadata: PatternMetadata::default(),
        }
    }

    fn from_tokens(tokens: &[Token], source: &str) -> Self {
        let base_offset = tokens.first().map_or(0, |t| t.span.start);
        Self {
            tokens: tokens.to_vec(),
            index: 0,
            span: None,
            source: source.to_string(),
            base_offset,
            metadata: PatternMetadata::default(),
        }
    }

    pub(super) fn expect_end(&mut self) -> Result<(), PatternParseError> {
        if self.peek().is_some() {
            Err(self.error("unexpected tokens after pattern"))
        } else {
            Ok(())
        }
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    pub(super) fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    pub(super) fn advance_or_error(&mut self, message: &str) -> Result<Token, PatternParseError> {
        self.advance()
            .ok_or_else(|| self.error(message.to_string()))
    }

    pub(super) fn peek_punctuation(&self, ch: char) -> bool {
        matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Punctuation(actual)) if *actual == ch
        )
    }

    pub(super) fn consume_keyword(&mut self, keyword: Keyword) -> bool {
        if matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Keyword(k)) if *k == keyword
        ) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn consume_identifier(&mut self, expected: &str) -> bool {
        if matches!(
            self.peek(),
            Some(Token { kind: TokenKind::Identifier, lexeme, .. }) if lexeme == expected
        ) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn consume_operator(&mut self, expected: &str) -> bool {
        if matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Operator(op)) if *op == expected
        ) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consumes a `..` slice operator regardless of whether the lexer emitted it as
    /// a single operator token or two consecutive `.` punctuations.
    pub(super) fn consume_slice_operator(&mut self) -> Option<usize> {
        if self.consume_operator("..") {
            return Some(self.index.saturating_sub(1));
        }
        let first = self.peek();
        let second = self.tokens.get(self.index + 1);
        if matches!(first.map(|t| &t.kind), Some(TokenKind::Punctuation('.')))
            && matches!(second.map(|t| &t.kind), Some(TokenKind::Punctuation('.')))
        {
            let start = self.index;
            self.advance();
            self.advance();
            return Some(start);
        }
        None
    }

    pub(super) fn parse_spanned_identifier(
        &mut self,
        context: &str,
    ) -> Result<(String, Option<Span>), PatternParseError> {
        let token = self
            .advance()
            .ok_or_else(|| self.error(format!("expected identifier for {context}")))?;
        match token.kind {
            TokenKind::Identifier | TokenKind::Keyword(_) => {
                let span = self.span.map(|_| {
                    Span::new(
                        self.base_offset + token.span.start,
                        self.base_offset + token.span.end,
                    )
                });
                Ok((token.lexeme, span))
            }
            _ => Err(self.error(format!("expected identifier for {context}"))),
        }
    }

    pub(super) fn parse_identifier(&mut self, context: &str) -> Result<String, PatternParseError> {
        self.parse_spanned_identifier(context).map(|(name, _)| name)
    }

    pub(super) fn expect_punctuation(&mut self, ch: char) -> Result<(), PatternParseError> {
        let token = self
            .advance()
            .ok_or_else(|| self.error(format!("expected `{ch}`")))?;
        match token.kind {
            TokenKind::Punctuation(actual) if actual == ch => Ok(()),
            _ => Err(self.error(format!("expected `{ch}`"))),
        }
    }

    pub(super) fn peek_relational_operator(&self) -> bool {
        matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Operator("<" | "<=" | ">" | ">="))
        ) || matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Punctuation('<' | '>'))
        )
    }

    pub(super) fn peek_pattern_terminator(&self) -> bool {
        matches!(
            self.peek().map(|t| &t.kind),
            None | Some(
                TokenKind::Punctuation(')' | '}' | ']' | ',')
                    | TokenKind::Keyword(Keyword::When | Keyword::Or | Keyword::And)
            )
        ) || matches!(
            self.peek().map(|t| &t.kind),
            Some(TokenKind::Operator(op))
                if matches!(
                    *op,
                    "&&" | "||"
                        | "|"
                        | "^"
                        | "&"
                        | "=="
                        | "!="
                        | "+"
                        | "-"
                        | "*"
                        | "/"
                        | "%"
                        | "??"
                        | "??="
                        | "="
                        | "+="
                        | "-="
                        | "*="
                        | "/="
                        | "%="
                        | "&="
                        | "|="
                        | "^="
                        | "<<="
                        | ">>="
                )
        )
    }

    pub(super) fn expression_from_tokens(&self, start: usize, end: usize) -> PatternExpression {
        let start_span = self.tokens[start].span;
        let end_span = self.tokens[end - 1].span;
        let text = self.source[start_span.start..end_span.end].to_string();
        let start_offset = self.base_offset + start_span.start;
        let end_offset = self.base_offset + end_span.end;
        let span = self.span.map(|_| Span::new(start_offset, end_offset));
        PatternExpression {
            text: text.trim().to_string(),
            span,
        }
    }

    pub(super) fn error(&self, message: impl Into<String>) -> PatternParseError {
        PatternParseError {
            message: message.into(),
            span: self.span,
        }
    }

    pub(super) fn span_from_range(&self, start: usize, end: usize) -> Option<Span> {
        if start >= end {
            return None;
        }
        let first = self.tokens.get(start)?;
        let last = self.tokens.get(end - 1)?;
        let start_offset = self.base_offset + first.span.start;
        let end_offset = self.base_offset + last.span.end;
        Some(Span::new(start_offset, end_offset))
    }

    pub(super) fn node_span(&self, node: &PatternNode) -> Option<Span> {
        match node {
            PatternNode::Binding(binding) => binding.span,
            PatternNode::List(list) => list.span,
            PatternNode::Struct { .. } => self.span,
            PatternNode::Enum { .. } => self.span,
            PatternNode::Positional { .. } => self.span,
            PatternNode::Tuple(_) => self.span,
            PatternNode::Type { .. } => self.span,
            PatternNode::Relational { expr, .. } => expr.span,
            PatternNode::Binary { left, right, .. } => {
                self.node_span(left).or_else(|| self.node_span(right))
            }
            PatternNode::Not(inner) => self.node_span(inner),
            PatternNode::Record(record) => record.span,
            PatternNode::Wildcard | PatternNode::Literal(_) => self.span,
        }
    }
}

pub(super) fn parse_number_literal(literal: &NumericLiteral) -> Option<i128> {
    let integer = numeric::parse_integer_literal(literal)?;
    if integer.is_unsigned || integer.value > i128::MAX as u128 {
        return None;
    }
    Some(integer.value as i128)
}

pub(super) fn parse_unsigned_literal(literal: &NumericLiteral) -> Option<u128> {
    numeric::parse_integer_literal(literal).map(|integer| integer.value)
}

pub(super) fn parse_float_literal(literal: &NumericLiteral) -> Option<f64> {
    if literal.kind != NumericLiteralKind::Float {
        return None;
    }
    literal.normalized_float_text().parse::<f64>().ok()
}
