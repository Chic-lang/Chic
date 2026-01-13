//! Closure lowering scaffolding: analysis, environment synthesis, and lowering.

pub(crate) mod analysis;
pub(crate) mod environment;
pub(crate) mod lowering;
#[cfg(test)]
mod tests;

pub(crate) use lowering::ClosureInfo;
