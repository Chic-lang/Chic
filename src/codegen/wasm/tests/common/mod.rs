#![cfg(test)]

use crate::mir::{TypeLayoutTable, configure_pointer_width};

mod ast;
mod mir_calls;
mod mir_constants;
mod mir_matches;
mod mir_misc;
mod mir_ops;
mod type_fixtures;
mod wasm_harness;
pub(super) use ast::*;
pub(super) use mir_calls::*;
pub(super) use mir_constants::*;
pub(super) use mir_matches::*;
pub(super) use mir_misc::*;
pub(super) use mir_ops::*;
pub(super) use type_fixtures::*;
pub(super) use wasm_harness::*;

pub(super) fn wasm_layouts() -> TypeLayoutTable {
    configure_pointer_width(4, 4);
    TypeLayoutTable::default()
}
