#![cfg(test)]

pub(super) use super::fixtures;

mod harness;
pub(super) use harness::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};

mod auto_traits;
mod borrows;
mod constraints;
mod consts;
mod di;
mod effects;
mod generics;
mod impl_trait;
mod initializers;
mod interfaces;
mod literals;
mod network;
mod operators;
mod patterns;
mod random;
mod signatures;
mod trait_impls;
