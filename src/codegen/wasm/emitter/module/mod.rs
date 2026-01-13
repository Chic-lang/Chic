mod builder;
mod sections;
mod signature;
mod support;
mod wat;

pub(crate) use builder::{ModuleBuilder, WasmStrLiteral};
#[cfg(test)]
pub(crate) use sections::Section;
pub(crate) use signature::FunctionSignature;
