//! Metadata for async/generator state machines and exception regions in MIR.

use crate::frontend::diagnostics::Span;
use crate::mir::AsyncFramePolicy;
use crate::mir::data::{BlockId, LocalId, Ty};

/// Captures async state-machine metadata for a MIR body.
#[derive(Debug, Clone, Default)]
pub struct AsyncStateMachine {
    pub suspend_points: Vec<AsyncSuspendPoint>,
    pub pinned_locals: Vec<LocalId>,
    pub cross_locals: Vec<LocalId>,
    pub frame_fields: Vec<AsyncFrameField>,
    pub result_local: Option<LocalId>,
    pub result_ty: Option<Ty>,
    pub context_local: Option<LocalId>,
    pub policy: AsyncFramePolicy,
}

/// Records one `await` suspension point in an async body.
#[derive(Debug, Clone)]
pub struct AsyncSuspendPoint {
    pub id: usize,
    pub await_block: BlockId,
    pub resume_block: BlockId,
    pub drop_block: BlockId,
    pub future: LocalId,
    pub destination: Option<LocalId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct AsyncFrameField {
    pub local: LocalId,
    pub name: Option<String>,
    pub ty: Ty,
}

/// Captures iterator state-machine metadata for generator bodies.
#[derive(Debug, Clone, Default)]
pub struct GeneratorStateMachine {
    pub yields: Vec<GeneratorYieldPoint>,
}

/// Records a `yield` suspension point inside an iterator body.
#[derive(Debug, Clone)]
pub struct GeneratorYieldPoint {
    pub id: usize,
    pub yield_block: BlockId,
    pub resume_block: BlockId,
    pub drop_block: BlockId,
    pub value: Option<LocalId>,
    pub span: Option<Span>,
}

/// Records the structure of a `try` statement and its handlers.
#[derive(Debug, Clone)]
pub struct ExceptionRegion {
    pub id: usize,
    pub span: Option<Span>,
    pub try_entry: BlockId,
    pub try_exit: BlockId,
    pub after_block: BlockId,
    pub dispatch: Option<BlockId>,
    pub catches: Vec<CatchRegion>,
    pub finally: Option<FinallyRegion>,
}

/// Metadata for a single `catch` clause.
#[derive(Debug, Clone)]
pub struct CatchRegion {
    pub span: Option<Span>,
    pub entry: BlockId,
    pub body: BlockId,
    pub cleanup: BlockId,
    pub ty: Option<Ty>,
    pub binding: Option<LocalId>,
    pub filter: Option<CatchFilter>,
}

/// Metadata describing a `when` filter attached to a catch clause.
#[derive(Debug, Clone)]
pub struct CatchFilter {
    pub expr: String,
    pub span: Option<Span>,
    pub parsed: bool,
    pub block: BlockId,
}

/// Metadata describing the `finally` clause of a `try` statement.
#[derive(Debug, Clone)]
pub struct FinallyRegion {
    pub span: Option<Span>,
    pub entry: BlockId,
    pub exit: BlockId,
}
