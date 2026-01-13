use super::diagnostics;
use super::state::Lexer;
use crate::frontend::diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericBase {
    Binary,
    Decimal,
    Hexadecimal,
}

impl NumericBase {
    fn name(self) -> &'static str {
        match self {
            NumericBase::Binary => "binary",
            NumericBase::Decimal => "decimal",
            NumericBase::Hexadecimal => "hexadecimal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericLiteralKind {
    Integer,
    Float,
    Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericLiteralSuffix {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    F16,
    F128,
    Decimal,
}

impl NumericLiteralSuffix {
    pub fn label(self) -> &'static str {
        match self {
            NumericLiteralSuffix::I8 => "i8",
            NumericLiteralSuffix::I16 => "i16",
            NumericLiteralSuffix::I32 => "i32",
            NumericLiteralSuffix::I64 => "i64",
            NumericLiteralSuffix::I128 => "i128",
            NumericLiteralSuffix::Isize => "isize",
            NumericLiteralSuffix::U8 => "u8",
            NumericLiteralSuffix::U16 => "u16",
            NumericLiteralSuffix::U32 => "u32",
            NumericLiteralSuffix::U64 => "u64",
            NumericLiteralSuffix::U128 => "u128",
            NumericLiteralSuffix::Usize => "usize",
            NumericLiteralSuffix::F16 => "f16",
            NumericLiteralSuffix::F32 => "f32",
            NumericLiteralSuffix::F64 => "f64",
            NumericLiteralSuffix::F128 => "f128",
            NumericLiteralSuffix::Decimal => "m",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericExponent {
    pub sign: Option<char>,
    pub digits: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericLiteral {
    pub base: NumericBase,
    pub kind: NumericLiteralKind,
    pub integer: String,
    pub fraction: Option<String>,
    pub exponent: Option<NumericExponent>,
    pub suffix: Option<NumericLiteralSuffix>,
    pub errors: Vec<NumericLiteralError>,
}

impl NumericLiteral {
    pub fn is_integer(&self) -> bool {
        matches!(self.kind, NumericLiteralKind::Integer)
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn normalized_float_text(&self) -> String {
        let mut text = self.integer.clone();
        if let Some(fraction) = &self.fraction {
            text.push('.');
            text.push_str(fraction);
        }
        if let Some(exponent) = &self.exponent {
            text.push('e');
            if let Some(sign) = exponent.sign {
                text.push(sign);
            }
            text.push_str(&exponent.digits);
        }
        text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericLiteralError {
    pub span: Span,
    pub kind: NumericLiteralErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericLiteralErrorKind {
    MissingDigits,
    InvalidDigit {
        ch: char,
        base: NumericBase,
    },
    LeadingSeparator,
    TrailingSeparator,
    RepeatedSeparator,
    SeparatorAdjacentToPoint,
    MultipleDecimalPoints,
    MultipleExponents,
    FractionNotAllowed {
        base: NumericBase,
    },
    ExponentNotAllowed {
        base: NumericBase,
    },
    MissingExponentDigits,
    InvalidSuffix {
        text: String,
    },
    SuffixNotAllowed {
        suffix: NumericLiteralSuffix,
        reason: SuffixRestriction,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuffixRestriction {
    FractionalLiteral,
    ExponentPresent,
    NonDecimalBase,
}

pub(super) struct NumericLiteralScan {
    pub literal: NumericLiteral,
    pub end: usize,
}

pub(super) fn scan_numeric_literal(lexer: &mut Lexer<'_>, start: usize) -> NumericLiteralScan {
    let mut builder = NumericLiteralBuilder::new(start);
    builder.scan(lexer);
    let (literal, end) = builder.finish();
    report_errors(lexer, &literal.errors);
    NumericLiteralScan { literal, end }
}

fn report_errors(lexer: &mut Lexer<'_>, errors: &[NumericLiteralError]) {
    for error in errors {
        let message = match &error.kind {
            NumericLiteralErrorKind::MissingDigits => {
                "numeric literal requires at least one digit".to_string()
            }
            NumericLiteralErrorKind::InvalidDigit { ch, base } => {
                format!("invalid digit `{}` for {} literal", ch, base.name())
            }
            NumericLiteralErrorKind::LeadingSeparator => {
                "digit separator `_` must be between digits".to_string()
            }
            NumericLiteralErrorKind::TrailingSeparator => {
                "digit separator `_` cannot appear at the end of the digits".to_string()
            }
            NumericLiteralErrorKind::RepeatedSeparator => {
                "digit separator `_` cannot appear consecutively".to_string()
            }
            NumericLiteralErrorKind::SeparatorAdjacentToPoint => {
                "digit separator `_` cannot appear next to a decimal point or exponent marker"
                    .to_string()
            }
            NumericLiteralErrorKind::MultipleDecimalPoints => {
                "numeric literal contains multiple decimal points".to_string()
            }
            NumericLiteralErrorKind::MultipleExponents => {
                "numeric literal contains multiple exponent markers".to_string()
            }
            NumericLiteralErrorKind::FractionNotAllowed { base } => format!(
                "fractional part is not permitted for {} literals",
                base.name()
            ),
            NumericLiteralErrorKind::ExponentNotAllowed { base } => {
                format!("exponent is not permitted for {} literals", base.name())
            }
            NumericLiteralErrorKind::MissingExponentDigits => {
                "exponent requires at least one digit".to_string()
            }
            NumericLiteralErrorKind::InvalidSuffix { text } => {
                format!("unknown numeric literal suffix `{text}`")
            }
            NumericLiteralErrorKind::SuffixNotAllowed { suffix, reason } => {
                let reason = match reason {
                    SuffixRestriction::FractionalLiteral => {
                        "cannot be applied to a literal with a fractional part or exponent"
                    }
                    SuffixRestriction::ExponentPresent => {
                        "decimal literals with `m` suffix cannot include an exponent"
                    }
                    SuffixRestriction::NonDecimalBase => {
                        "suffix is only valid for decimal literals"
                    }
                };
                format!("suffix `{}` {}", suffix.label(), reason)
            }
        };
        diagnostics::report_simple_error(lexer, &message, error.span);
    }
}

struct NumericLiteralBuilder {
    start: usize,
    end: usize,
    base: NumericBase,
    integer: Section,
    fraction: Section,
    exponent: Section,
    exponent_sign: Option<char>,
    exponent_state: ExponentState,
    exponent_marker_span: Option<Span>,
    suffix_text: String,
    suffix_kind: Option<NumericLiteralSuffix>,
    suffix_invalid: bool,
    in_suffix: bool,
    saw_decimal_point: bool,
    saw_exponent: bool,
    kind: NumericLiteralKind,
    errors: Vec<NumericLiteralError>,
}

impl NumericLiteralBuilder {
    fn new(start: usize) -> Self {
        Self {
            start,
            end: start,
            base: NumericBase::Decimal,
            integer: Section::default(),
            fraction: Section::default(),
            exponent: Section::default(),
            exponent_sign: None,
            exponent_state: ExponentState::AwaitingDigit,
            exponent_marker_span: None,
            suffix_text: String::new(),
            suffix_kind: None,
            suffix_invalid: false,
            in_suffix: false,
            saw_decimal_point: false,
            saw_exponent: false,
            kind: NumericLiteralKind::Integer,
            errors: Vec::new(),
        }
    }

    fn scan(&mut self, lexer: &mut Lexer<'_>) {
        self.consume_prefix(lexer);

        while let Some((idx, ch)) = lexer.lookahead {
            if self.in_suffix {
                if !self.consume_suffix_char(lexer, idx, ch) {
                    break;
                }
                continue;
            }

            if self.saw_exponent {
                if !self.consume_exponent_char(lexer, idx, ch) {
                    break;
                }
                continue;
            }

            if self.saw_decimal_point {
                if !self.consume_fraction_char(lexer, idx, ch) {
                    break;
                }
                continue;
            }

            if !self.consume_integer_char(lexer, idx, ch) {
                break;
            }
        }
    }

    fn finish(mut self) -> (NumericLiteral, usize) {
        if !self.integer.has_digits() {
            self.errors.push(NumericLiteralError {
                span: Span::new(self.start, self.end.max(self.start + 1)),
                kind: NumericLiteralErrorKind::MissingDigits,
            });
        }

        self.integer.finish(&mut self.errors);

        if self.saw_decimal_point {
            self.fraction.finish(&mut self.errors);
        }

        if self.saw_exponent {
            if !self.exponent.has_digits() {
                if let Some(span) = self.exponent_marker_span {
                    self.errors.push(NumericLiteralError {
                        span,
                        kind: NumericLiteralErrorKind::MissingExponentDigits,
                    });
                }
            }
            self.exponent.finish(&mut self.errors);
        }

        if let Some(ref suffix) = self.suffix_kind {
            self.validate_suffix(*suffix);
        } else if !self.suffix_text.is_empty() && !self.suffix_invalid {
            self.errors.push(NumericLiteralError {
                span: self.suffix_span(),
                kind: NumericLiteralErrorKind::InvalidSuffix {
                    text: self.suffix_text.clone(),
                },
            });
        }

        let NumericLiteralBuilder {
            end,
            base,
            integer,
            fraction,
            exponent,
            exponent_sign,
            suffix_kind,
            saw_decimal_point,
            saw_exponent,
            kind,
            errors,
            ..
        } = self;

        let fraction = if saw_decimal_point {
            Some(fraction.digits)
        } else {
            None
        };

        let exponent = if saw_exponent {
            Some(NumericExponent {
                sign: exponent_sign,
                digits: exponent.digits,
            })
        } else {
            None
        };

        let literal = NumericLiteral {
            base,
            kind,
            integer: integer.digits,
            fraction,
            exponent,
            suffix: suffix_kind,
            errors,
        };
        (literal, end)
    }

    fn consume_prefix(&mut self, lexer: &mut Lexer<'_>) {
        let Some((zero_idx, first)) = lexer.lookahead else {
            return;
        };
        if first != '0' {
            return;
        }
        if let Some((_, next)) = lexer.peek_char_offset(0) {
            match next {
                'x' | 'X' => {
                    self.consume_char(lexer);
                    self.consume_char(lexer);
                    self.base = NumericBase::Hexadecimal;
                    self.integer.allow_leading_separator = true;
                }
                'b' | 'B' => {
                    self.consume_char(lexer);
                    self.consume_char(lexer);
                    self.base = NumericBase::Binary;
                    self.integer.allow_leading_separator = true;
                }
                _ => {}
            }
        } else {
            // literal is just "0"
            self.integer
                .push_digit('0', Span::new(zero_idx, zero_idx + 1));
            self.consume_char(lexer);
        }
    }

    fn consume_integer_char(&mut self, lexer: &mut Lexer<'_>, idx: usize, ch: char) -> bool {
        if let Some(digit) = sanitize_digit(self.base, ch) {
            let span = self.consume_char(lexer);
            self.integer.push_digit(digit, span);
            return true;
        }

        match ch {
            '_' => {
                let span = self.consume_char(lexer);
                self.integer
                    .push_separator(span, &mut self.errors, SeparatorPosition::Middle);
                true
            }
            '.' => {
                if let Some((_, '.')) = lexer.peek_char_offset(0) {
                    return false;
                }
                self.consume_char(lexer);
                self.start_fraction(idx);
                true
            }
            'e' | 'E' => {
                self.consume_char(lexer);
                self.start_exponent(idx);
                true
            }
            ch if is_suffix_start(ch) => {
                self.start_suffix_mode(lexer, idx, ch);
                true
            }
            ch if ch.is_ascii_alphabetic() || ch.is_ascii_digit() => {
                let span = self.consume_char(lexer);
                self.errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::InvalidDigit {
                        ch,
                        base: self.base,
                    },
                });
                true
            }
            _ => false,
        }
    }

    fn consume_fraction_char(&mut self, lexer: &mut Lexer<'_>, idx: usize, ch: char) -> bool {
        if let Some(digit) = sanitize_digit(NumericBase::Decimal, ch) {
            let span = self.consume_char(lexer);
            self.fraction.push_digit(digit, span);
            return true;
        }

        match ch {
            '_' => {
                let span = self.consume_char(lexer);
                self.fraction
                    .push_separator(span, &mut self.errors, SeparatorPosition::Middle);
                true
            }
            'e' | 'E' => {
                self.consume_char(lexer);
                self.start_exponent(idx);
                true
            }
            ch if is_suffix_start(ch) => {
                self.start_suffix_mode(lexer, idx, ch);
                true
            }
            ch if ch.is_ascii_alphabetic() || ch.is_ascii_digit() => {
                let span = self.consume_char(lexer);
                self.errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::InvalidDigit {
                        ch,
                        base: NumericBase::Decimal,
                    },
                });
                true
            }
            '.' => {
                let span = self.consume_char(lexer);
                self.errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::MultipleDecimalPoints,
                });
                true
            }
            _ => false,
        }
    }

    fn consume_exponent_char(&mut self, lexer: &mut Lexer<'_>, idx: usize, ch: char) -> bool {
        match self.exponent_state {
            ExponentState::AwaitingDigit => match ch {
                '+' | '-' if self.exponent_sign.is_none() => {
                    self.exponent_sign = Some(ch);
                    self.consume_char(lexer);
                    true
                }
                _ => {
                    self.exponent_state = ExponentState::Digits;
                    self.consume_exponent_char(lexer, idx, ch)
                }
            },
            ExponentState::Digits => {
                if let Some(digit) = sanitize_digit(NumericBase::Decimal, ch) {
                    let span = self.consume_char(lexer);
                    self.exponent.push_digit(digit, span);
                    return true;
                }
                match ch {
                    '_' => {
                        let span = self.consume_char(lexer);
                        self.exponent.push_separator(
                            span,
                            &mut self.errors,
                            SeparatorPosition::Middle,
                        );
                        true
                    }
                    ch if is_suffix_start(ch) => {
                        self.start_suffix_mode(lexer, idx, ch);
                        true
                    }
                    ch if ch.is_ascii_alphabetic() || ch.is_ascii_digit() => {
                        let span = self.consume_char(lexer);
                        self.errors.push(NumericLiteralError {
                            span,
                            kind: NumericLiteralErrorKind::InvalidDigit {
                                ch,
                                base: NumericBase::Decimal,
                            },
                        });
                        true
                    }
                    '.' => {
                        let span = self.consume_char(lexer);
                        self.errors.push(NumericLiteralError {
                            span,
                            kind: NumericLiteralErrorKind::FractionNotAllowed {
                                base: NumericBase::Decimal,
                            },
                        });
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    fn consume_suffix_char(&mut self, lexer: &mut Lexer<'_>, idx: usize, ch: char) -> bool {
        if ch.is_ascii_alphanumeric() {
            self.consume_char(lexer);
            self.suffix_text.push(ch);
            if !self.suffix_invalid {
                self.suffix_kind = map_suffix(&self.suffix_text);
            }
            return true;
        }
        if ch == '_' {
            let span = self.consume_char(lexer);
            self.suffix_text.push(ch);
            self.suffix_invalid = true;
            self.errors.push(NumericLiteralError {
                span,
                kind: NumericLiteralErrorKind::InvalidSuffix {
                    text: self.suffix_text.clone(),
                },
            });
            return true;
        }

        // suffix ended
        let _ = idx;
        false
    }

    fn start_fraction(&mut self, idx: usize) {
        if matches!(self.base, NumericBase::Binary | NumericBase::Hexadecimal) {
            self.errors.push(NumericLiteralError {
                span: Span::new(idx, idx + 1),
                kind: NumericLiteralErrorKind::FractionNotAllowed { base: self.base },
            });
        }
        if self.saw_decimal_point {
            self.errors.push(NumericLiteralError {
                span: Span::new(idx, idx + 1),
                kind: NumericLiteralErrorKind::MultipleDecimalPoints,
            });
        }
        if matches!(
            self.current_section().last_char,
            Some(SectionChar::Separator)
        ) {
            if let Some(span) = self.current_section().last_separator_span {
                self.errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::SeparatorAdjacentToPoint,
                });
            }
        }
        self.saw_decimal_point = true;
        self.kind = NumericLiteralKind::Float;
    }

    fn start_exponent(&mut self, idx: usize) {
        if matches!(self.base, NumericBase::Binary | NumericBase::Hexadecimal) {
            self.errors.push(NumericLiteralError {
                span: Span::new(idx, idx + 1),
                kind: NumericLiteralErrorKind::ExponentNotAllowed { base: self.base },
            });
        }
        if self.saw_exponent {
            self.errors.push(NumericLiteralError {
                span: Span::new(idx, idx + 1),
                kind: NumericLiteralErrorKind::MultipleExponents,
            });
        }
        if matches!(
            self.current_section().last_char,
            Some(SectionChar::Separator)
        ) {
            if let Some(span) = self.current_section().last_separator_span {
                self.errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::SeparatorAdjacentToPoint,
                });
            }
        }
        self.saw_exponent = true;
        self.kind = NumericLiteralKind::Float;
        self.exponent_state = ExponentState::AwaitingDigit;
        self.exponent_marker_span = Some(Span::new(idx, idx + 1));
    }

    fn start_suffix_mode(&mut self, lexer: &mut Lexer<'_>, idx: usize, ch: char) {
        self.in_suffix = true;
        let _ = idx;
        self.consume_suffix_char(lexer, idx, ch);
    }

    fn validate_suffix(&mut self, suffix: NumericLiteralSuffix) {
        match suffix {
            NumericLiteralSuffix::Decimal => {
                if !matches!(self.base, NumericBase::Decimal) {
                    self.errors.push(NumericLiteralError {
                        span: self.suffix_span(),
                        kind: NumericLiteralErrorKind::SuffixNotAllowed {
                            suffix,
                            reason: SuffixRestriction::NonDecimalBase,
                        },
                    });
                } else if self.saw_exponent {
                    self.errors.push(NumericLiteralError {
                        span: self.suffix_span(),
                        kind: NumericLiteralErrorKind::SuffixNotAllowed {
                            suffix,
                            reason: SuffixRestriction::ExponentPresent,
                        },
                    });
                } else {
                    self.kind = NumericLiteralKind::Decimal;
                }
            }
            NumericLiteralSuffix::F16
            | NumericLiteralSuffix::F32
            | NumericLiteralSuffix::F64
            | NumericLiteralSuffix::F128 => {
                if !matches!(self.base, NumericBase::Decimal) {
                    self.errors.push(NumericLiteralError {
                        span: self.suffix_span(),
                        kind: NumericLiteralErrorKind::SuffixNotAllowed {
                            suffix,
                            reason: SuffixRestriction::NonDecimalBase,
                        },
                    });
                } else {
                    self.kind = NumericLiteralKind::Float;
                }
            }
            _ => {
                if self.saw_decimal_point || self.saw_exponent {
                    self.errors.push(NumericLiteralError {
                        span: self.suffix_span(),
                        kind: NumericLiteralErrorKind::SuffixNotAllowed {
                            suffix,
                            reason: SuffixRestriction::FractionalLiteral,
                        },
                    });
                }
            }
        }
    }

    fn current_section(&self) -> &Section {
        if self.saw_exponent {
            &self.exponent
        } else if self.saw_decimal_point {
            &self.fraction
        } else {
            &self.integer
        }
    }

    fn consume_char(&mut self, lexer: &mut Lexer<'_>) -> Span {
        let (idx, ch) = lexer.lookahead.expect("expected character");
        let end = idx + ch.len_utf8();
        lexer.bump();
        self.end = end;
        Span::new(idx, end)
    }

    fn suffix_span(&self) -> Span {
        let start = self.end.saturating_sub(self.suffix_text.len());
        Span::new(start, self.end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExponentState {
    AwaitingDigit,
    Digits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionChar {
    Digit,
    Separator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeparatorPosition {
    Middle,
}

#[derive(Debug, Default, Clone)]
struct Section {
    digits: String,
    saw_digit: bool,
    last_char: Option<SectionChar>,
    last_separator_span: Option<Span>,
    allow_leading_separator: bool,
}

impl Section {
    fn push_digit(&mut self, ch: char, _span: Span) {
        self.digits.push(ch);
        self.saw_digit = true;
        self.last_char = Some(SectionChar::Digit);
        self.allow_leading_separator = false;
    }

    fn push_separator(
        &mut self,
        span: Span,
        errors: &mut Vec<NumericLiteralError>,
        _position: SeparatorPosition,
    ) {
        match self.last_char {
            None if !self.saw_digit => {
                if self.allow_leading_separator {
                    self.last_char = Some(SectionChar::Separator);
                    self.last_separator_span = Some(span);
                    self.allow_leading_separator = false;
                    return;
                }
                errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::LeadingSeparator,
                });
            }
            Some(SectionChar::Separator) => errors.push(NumericLiteralError {
                span,
                kind: NumericLiteralErrorKind::RepeatedSeparator,
            }),
            _ => {}
        }
        self.last_char = Some(SectionChar::Separator);
        self.last_separator_span = Some(span);
    }

    fn has_digits(&self) -> bool {
        self.saw_digit
    }

    fn finish(&mut self, errors: &mut Vec<NumericLiteralError>) {
        if matches!(self.last_char, Some(SectionChar::Separator)) {
            if let Some(span) = self.last_separator_span {
                errors.push(NumericLiteralError {
                    span,
                    kind: NumericLiteralErrorKind::TrailingSeparator,
                });
            }
        }
    }
}

fn sanitize_digit(base: NumericBase, ch: char) -> Option<char> {
    match base {
        NumericBase::Binary => match ch {
            '0' | '1' => Some(ch),
            _ => None,
        },
        NumericBase::Decimal => ch.is_ascii_digit().then_some(ch),
        NumericBase::Hexadecimal => {
            if ch.is_ascii_digit() {
                Some(ch)
            } else if ch.is_ascii_hexdigit() {
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        }
    }
}

fn is_suffix_start(ch: char) -> bool {
    matches!(
        ch,
        'i' | 'I' | 'u' | 'U' | 'l' | 'L' | 'f' | 'F' | 'd' | 'D' | 'm' | 'M' | 's' | 'S'
    )
}

fn map_suffix(text: &str) -> Option<NumericLiteralSuffix> {
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "i8" => Some(NumericLiteralSuffix::I8),
        "i16" => Some(NumericLiteralSuffix::I16),
        "i32" => Some(NumericLiteralSuffix::I32),
        "i64" => Some(NumericLiteralSuffix::I64),
        "i128" => Some(NumericLiteralSuffix::I128),
        "isize" => Some(NumericLiteralSuffix::Isize),
        "u8" => Some(NumericLiteralSuffix::U8),
        "u16" => Some(NumericLiteralSuffix::U16),
        "u32" => Some(NumericLiteralSuffix::U32),
        "u64" => Some(NumericLiteralSuffix::U64),
        "u128" => Some(NumericLiteralSuffix::U128),
        "usize" => Some(NumericLiteralSuffix::Usize),
        "u" => Some(NumericLiteralSuffix::U32),
        "l" => Some(NumericLiteralSuffix::I64),
        "ul" | "lu" => Some(NumericLiteralSuffix::U64),
        "f16" => Some(NumericLiteralSuffix::F16),
        "f" | "f32" => Some(NumericLiteralSuffix::F32),
        "d" | "f64" | "d64" | "double" => Some(NumericLiteralSuffix::F64),
        "f128" | "q" | "quad" => Some(NumericLiteralSuffix::F128),
        "m" => Some(NumericLiteralSuffix::Decimal),
        _ => None,
    }
}
