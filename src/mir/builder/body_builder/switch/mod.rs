use super::*;

mod analysis;
mod cases;
mod entry;
mod int_lowering;
mod match_lowering;

#[derive(Clone, Copy)]
pub(super) struct SwitchBindingLocal {
    pub(super) local: LocalId,
    pub(super) mutability: PatternBindingMutability,
    pub(super) mode: PatternBindingMode,
}
