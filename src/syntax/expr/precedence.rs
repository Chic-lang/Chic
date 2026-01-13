//! Operator precedence and classification helpers shared between the parser
//! and formatter.

use crate::frontend::lexer::{Keyword, Token, TokenKind};
use crate::mir::{BinOp, UnOp};

use super::builders::AssignOp;

#[derive(Clone, Copy)]
struct BinaryOpSpec {
    symbol: &'static str,
    precedence: u8,
    op: BinOp,
}

const BINARY_OP_SPECS: &[BinaryOpSpec] = &[
    BinaryOpSpec {
        symbol: "??",
        precedence: 0,
        op: BinOp::NullCoalesce,
    },
    BinaryOpSpec {
        symbol: "||",
        precedence: 1,
        op: BinOp::Or,
    },
    BinaryOpSpec {
        symbol: "&&",
        precedence: 3,
        op: BinOp::And,
    },
    BinaryOpSpec {
        symbol: "|",
        precedence: 2,
        op: BinOp::BitOr,
    },
    BinaryOpSpec {
        symbol: "^",
        precedence: 4,
        op: BinOp::BitXor,
    },
    BinaryOpSpec {
        symbol: "&",
        precedence: 4,
        op: BinOp::BitAnd,
    },
    BinaryOpSpec {
        symbol: "==",
        precedence: 5,
        op: BinOp::Eq,
    },
    BinaryOpSpec {
        symbol: "!=",
        precedence: 5,
        op: BinOp::Ne,
    },
    BinaryOpSpec {
        symbol: "<",
        precedence: 6,
        op: BinOp::Lt,
    },
    BinaryOpSpec {
        symbol: "<=",
        precedence: 6,
        op: BinOp::Le,
    },
    BinaryOpSpec {
        symbol: ">",
        precedence: 6,
        op: BinOp::Gt,
    },
    BinaryOpSpec {
        symbol: ">=",
        precedence: 6,
        op: BinOp::Ge,
    },
    BinaryOpSpec {
        symbol: "<<",
        precedence: 7,
        op: BinOp::Shl,
    },
    BinaryOpSpec {
        symbol: ">>",
        precedence: 7,
        op: BinOp::Shr,
    },
    BinaryOpSpec {
        symbol: "+",
        precedence: 8,
        op: BinOp::Add,
    },
    BinaryOpSpec {
        symbol: "-",
        precedence: 8,
        op: BinOp::Sub,
    },
    BinaryOpSpec {
        symbol: "*",
        precedence: 9,
        op: BinOp::Mul,
    },
    BinaryOpSpec {
        symbol: "/",
        precedence: 9,
        op: BinOp::Div,
    },
    BinaryOpSpec {
        symbol: "%",
        precedence: 9,
        op: BinOp::Rem,
    },
];

#[derive(Clone, Copy)]
struct AssignOpSpec {
    symbol: &'static str,
    op: AssignOp,
}

const ASSIGN_OP_SPECS: &[AssignOpSpec] = &[
    AssignOpSpec {
        symbol: "=",
        op: AssignOp::Assign,
    },
    AssignOpSpec {
        symbol: "??=",
        op: AssignOp::NullCoalesceAssign,
    },
    AssignOpSpec {
        symbol: "+=",
        op: AssignOp::AddAssign,
    },
    AssignOpSpec {
        symbol: "-=",
        op: AssignOp::SubAssign,
    },
    AssignOpSpec {
        symbol: "*=",
        op: AssignOp::MulAssign,
    },
    AssignOpSpec {
        symbol: "/=",
        op: AssignOp::DivAssign,
    },
    AssignOpSpec {
        symbol: "%=",
        op: AssignOp::RemAssign,
    },
    AssignOpSpec {
        symbol: "&=",
        op: AssignOp::BitAndAssign,
    },
    AssignOpSpec {
        symbol: "|=",
        op: AssignOp::BitOrAssign,
    },
    AssignOpSpec {
        symbol: "^=",
        op: AssignOp::BitXorAssign,
    },
    AssignOpSpec {
        symbol: "<<=",
        op: AssignOp::ShlAssign,
    },
    AssignOpSpec {
        symbol: ">>=",
        op: AssignOp::ShrAssign,
    },
];

/// Lookup the binary operator associated with a token and return its precedence.
#[must_use]
pub fn binary_precedence(token: &Token) -> Option<(u8, BinOp)> {
    let symbol = match token.kind {
        TokenKind::Operator(sym) => sym,
        TokenKind::Punctuation('<' | '>') => token.lexeme.as_str(),
        _ => return None,
    };
    BINARY_OP_SPECS
        .iter()
        .find(|entry| entry.symbol == symbol)
        .map(|entry| (entry.precedence, entry.op))
}

/// Determine the assignment operator encoded by a token if present.
#[must_use]
pub fn assignment_operator(token: &Token) -> Option<AssignOp> {
    let TokenKind::Operator(symbol) = token.kind.clone() else {
        return None;
    };
    ASSIGN_OP_SPECS
        .iter()
        .find(|entry| entry.symbol == symbol)
        .map(|entry| entry.op)
}

/// Check whether a token can begin a unary expression.
#[must_use]
pub fn can_start_unary_expression(token: &Token) -> bool {
    match &token.kind {
        TokenKind::Operator(op) => {
            matches!(*op, "!" | "-" | "+" | "~" | "++" | "--" | "&" | "*" | "^")
        }
        TokenKind::Keyword(
            Keyword::Await | Keyword::Throw | Keyword::New | Keyword::Sizeof | Keyword::Alignof,
        )
        | TokenKind::Identifier
        | TokenKind::NumberLiteral(_)
        | TokenKind::StringLiteral(_)
        | TokenKind::CharLiteral(_)
        | TokenKind::Punctuation('(') => true,
        _ => false,
    }
}

/// Map a token to a unary operator when valid.
#[must_use]
pub fn unary_operator(token: &Token) -> Option<UnOp> {
    match token.kind {
        TokenKind::Operator(op) => match op {
            "!" => Some(UnOp::Not),
            "-" => Some(UnOp::Neg),
            "+" => Some(UnOp::UnaryPlus),
            "~" => Some(UnOp::BitNot),
            "++" => Some(UnOp::Increment),
            "--" => Some(UnOp::Decrement),
            _ => None,
        },
        _ => None,
    }
}

/// Retrieve the printed symbol for a binary operator.
///
/// # Panics
///
/// Panics if the provided operator is missing from `BINARY_OP_SPECS`,
/// which should be impossible while the table mirrors `BinOp`.
#[must_use]
pub fn binary_operator_symbol(op: BinOp) -> &'static str {
    binary_spec(op).map_or_else(
        || unreachable!("binary operator missing from table: {op:?}"),
        |entry| entry.symbol,
    )
}

/// Lookup the precedence value for a binary operator.
///
/// # Panics
///
/// Panics if the provided operator is missing from `BINARY_OP_SPECS`,
/// which should be impossible while the table mirrors `BinOp`.
#[must_use]
pub fn precedence_for_bin_op(op: BinOp) -> u8 {
    binary_spec(op).map_or_else(
        || unreachable!("binary operator missing from table: {op:?}"),
        |entry| entry.precedence,
    )
}

/// Determine whether a binary operator associates to the right.
#[must_use]
pub fn is_right_associative(op: BinOp) -> bool {
    matches!(op, BinOp::NullCoalesce)
}

/// Retrieve the textual symbol for an assignment operator.
///
/// # Panics
///
/// Panics if the provided operator is missing from `ASSIGN_OP_SPECS`,
/// which should be impossible while the table mirrors `AssignOp`.
#[must_use]
pub fn assignment_operator_symbol(op: AssignOp) -> &'static str {
    assign_spec(op).map_or_else(
        || unreachable!("assignment operator missing from table: {op:?}"),
        |entry| entry.symbol,
    )
}

#[must_use]
fn binary_spec(op: BinOp) -> Option<&'static BinaryOpSpec> {
    BINARY_OP_SPECS.iter().find(|entry| entry.op == op)
}

#[must_use]
fn assign_spec(op: AssignOp) -> Option<&'static AssignOpSpec> {
    ASSIGN_OP_SPECS.iter().find(|entry| entry.op == op)
}
