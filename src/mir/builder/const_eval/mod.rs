use crate::mir::data::ConstValue;
use crate::syntax::numeric::NumericLiteralMetadata;

pub mod diagnostics;
pub mod environment;
pub mod fold;

pub use environment::{ConstEvalContext, ConstEvalSummary};

#[derive(Debug, Clone)]
pub struct ConstEvalResult {
    pub value: ConstValue,
    pub literal: Option<NumericLiteralMetadata>,
}

impl ConstEvalResult {
    fn normalise(value: ConstValue) -> ConstValue {
        match value {
            ConstValue::Int32(v) => ConstValue::Int(v),
            other => other,
        }
    }

    #[must_use]
    pub fn new(value: ConstValue) -> Self {
        Self {
            value: Self::normalise(value),
            literal: None,
        }
    }

    #[must_use]
    pub fn with_literal(value: ConstValue, literal: Option<NumericLiteralMetadata>) -> Self {
        Self {
            value: Self::normalise(value),
            literal,
        }
    }
}
