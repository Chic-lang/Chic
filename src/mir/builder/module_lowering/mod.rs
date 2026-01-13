pub(crate) mod attributes;
pub(crate) mod cache;
pub(crate) mod classes;
pub(crate) mod driver;
pub(crate) mod interfaces;
pub(crate) mod layout;
pub(crate) mod operators;
pub(crate) mod pipeline;
pub(crate) mod primitives;
pub(crate) mod queue;
pub(crate) mod traits;

pub use driver::{
    LoweringDiagnostic, LoweringResult, ModuleUnitSlice, lower_module, lower_module_with_units,
    lower_module_with_units_and_hook,
};
