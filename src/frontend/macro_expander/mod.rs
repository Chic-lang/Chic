mod cache;
mod collector;
mod diagnostics;
mod engine;
mod handlers;
mod model;
mod origin;
mod registry;

pub use engine::{MacroExpansionResult, expand_module};
pub use registry::MacroRegistry;

#[cfg(test)]
mod tests;
