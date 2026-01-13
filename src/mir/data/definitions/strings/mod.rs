//! Core MIR data structures representing Chic mid-level IR.

pub mod interning;
pub mod module;
pub mod module_metadata;

mod basic_blocks;
mod functions;
mod types;
mod utils;
mod vtables;

pub use basic_blocks::*;
pub use functions::*;
pub use interning::*;
pub use module::*;
pub use module_metadata::*;
pub use types::*;
pub use utils::*;
pub use vtables::*;
