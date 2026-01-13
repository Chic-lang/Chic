//! MIR layout helpers split into dedicated modules.

mod auto_traits;
mod builtins;
mod functions;
mod nullable;
pub mod table;
mod tuples;

pub use auto_traits::{AutoTraitOverride, AutoTraitSet, AutoTraitStatus};
pub use table::*;
