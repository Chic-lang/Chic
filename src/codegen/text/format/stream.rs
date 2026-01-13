//! Formatting helpers for Chic textual code generation.

mod declarations;
#[cfg(test)]
mod tests;

pub(crate) use declarations::write_module;
