mod early_returns;
mod helpers;
mod loops;
mod switches;

pub(super) mod prelude {
    pub(super) use super::super::common::{
        RequireExt, assert_drop_sequence, assert_no_defer_drop, assert_no_pending,
        extract_for_spans, extract_while_spans, find_block_with_span,
        find_block_with_statement_span, storage_dead_index,
    };
    pub(super) use super::super::*;
    pub(super) use super::helpers::*;
    pub(super) use crate::mir::AggregateKind;
    pub(super) use crate::mir::ConstOperand;
    pub(super) use crate::mir::data::{BinOp, SpanTy, Ty};
    pub(super) use crate::mir::layout::table::{
        test_clear_cross_inline_overrides, test_set_cross_inline_override,
    };
}
