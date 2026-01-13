//! Helpers for parsing Chic string and character literals.

mod character;
mod escape;
mod string;

pub use character::{CharLiteral, parse_char_literal};
pub use escape::{LiteralError, LiteralErrorKind};
pub use string::{
    InterpolationSegment, StringLiteral, StringLiteralContents, StringLiteralKind, StringSegment,
    parse_string_literal,
};
