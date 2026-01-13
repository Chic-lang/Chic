use super::base::*;
use super::*;
use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::mir::data::{BlockId, BorrowId, BorrowKind, LocalId, Place, ProjectionElem, RegionVar};
use crate::mir::{MirFunction, TypeLayoutTable};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

mod core;
mod diagnostics;
mod operations;
mod statements;
mod terminators;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SyntheticLocationKey {
    Statement { block: BlockId, index: usize },
    Terminator { block: BlockId },
}

impl From<Location> for SyntheticLocationKey {
    fn from(location: Location) -> Self {
        match location {
            Location::Statement { block, index } => Self::Statement { block, index },
            Location::Terminator { block } => Self::Terminator { block },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct SyntheticPlaceKey {
    pub(super) local: LocalId,
    pub(super) projection: Vec<SyntheticProjectionKey>,
}

impl From<&Place> for SyntheticPlaceKey {
    fn from(place: &Place) -> Self {
        Self {
            local: place.local,
            projection: place
                .projection
                .iter()
                .map(|elem| match elem {
                    ProjectionElem::Field(index) => SyntheticProjectionKey::Field(*index),
                    ProjectionElem::FieldNamed(name) => {
                        SyntheticProjectionKey::FieldNamed(name.clone())
                    }
                    ProjectionElem::Index(local) => SyntheticProjectionKey::Index(*local),
                    ProjectionElem::ConstantIndex {
                        offset,
                        length,
                        from_end,
                    } => SyntheticProjectionKey::ConstantIndex {
                        offset: *offset,
                        length: *length,
                        from_end: *from_end,
                    },
                    ProjectionElem::Deref => SyntheticProjectionKey::Deref,
                    ProjectionElem::Downcast { variant } => {
                        SyntheticProjectionKey::Downcast { variant: *variant }
                    }
                    ProjectionElem::Subslice { from, to } => SyntheticProjectionKey::Subslice {
                        from: *from,
                        to: *to,
                    },
                    ProjectionElem::UnionField { index, name } => {
                        SyntheticProjectionKey::UnionField {
                            index: *index,
                            name: name.clone(),
                        }
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum SyntheticProjectionKey {
    Field(u32),
    FieldNamed(String),
    Index(LocalId),
    ConstantIndex {
        offset: usize,
        length: usize,
        from_end: bool,
    },
    Deref,
    Downcast {
        variant: u32,
    },
    Subslice {
        from: usize,
        to: usize,
    },
    UnionField {
        index: u32,
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct StreamEventLoanKey {
    pub(super) location: SyntheticLocationKey,
    pub(super) dependency: SyntheticPlaceKey,
    pub(super) until_local: LocalId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct SpanViewLoanKey {
    pub(super) location: SyntheticLocationKey,
    pub(super) view_local: LocalId,
    pub(super) root: SyntheticPlaceKey,
    pub(super) kind: BorrowKind,
}

pub(in crate::mir::borrow) struct BorrowChecker<'a> {
    pub(super) function: &'a MirFunction,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) reported: HashSet<ErrorKey>,
    pub(super) regions: HashMap<RegionVar, RegionInfo>,
    pub(super) out_argument_regions: HashSet<RegionVar>,
    pub(super) type_layouts: &'a TypeLayoutTable,
    next_synthetic_borrow: usize,
    next_synthetic_region: usize,
    pub(super) stream_event_loan_cache: HashMap<StreamEventLoanKey, (BorrowId, RegionVar)>,
    pub(super) span_view_loan_cache: HashMap<SpanViewLoanKey, (BorrowId, RegionVar)>,
}
