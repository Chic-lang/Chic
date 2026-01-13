//! Body-builder state orchestration helpers split across dedicated modules.

pub(super) mod assignments;
mod blocks;
pub(super) mod graph;
mod locals;
mod scopes;
mod transitions;

pub(crate) use assignments::AssignmentSourceKind;
pub(crate) use graph::LoopContext;
