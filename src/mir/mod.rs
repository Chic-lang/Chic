//! Chic MIR data structures, lowering, and verification.

mod async_control;
mod async_lowering;
pub mod async_types;
mod borrow;
pub(crate) mod builder;
pub(crate) mod casts;
mod data;
mod expr;
mod layout;
mod operators;
mod passes;
mod pretty;
mod state;
mod test_metadata;
mod traits;
mod verify;

pub use async_control::{
    ASYNC_DIAG_ATTRIBUTE, ASYNC_DIAG_FRAME_LIMIT, ASYNC_DIAG_NO_CAPTURE, ASYNC_DIAG_STACK_ONLY,
    AsyncFramePolicy, AttrSource, FrameLimitAttr, NoCaptureAttr, NoCaptureMode,
};
pub use async_lowering::{
    AsyncFrameFieldPlan, AsyncFrameMetrics, AsyncLoweringArtifact, AsyncSuspendPlan,
    lower_async_functions,
};
pub use borrow::{BorrowCheckResult, borrow_check_function, borrow_check_module};
pub use builder::accelerator::AcceleratorBuilder;
pub use builder::{
    ConstEvalContext, ConstEvalSummary, FieldMetadata, PropertyMetadata, SymbolIndex,
};
pub use builder::{
    LoweringDiagnostic, LoweringResult, ModuleUnitSlice, lower_module, lower_module_with_units,
    lower_module_with_units_and_hook,
};
pub use data::*;
pub use layout::*;
pub use passes::cost_model::normalise_cost_model;
pub use passes::fallible_drop::check_fallible_values;
pub use passes::memory_plan::{BufferPlan, MemoryPlan};
pub use passes::raw_strings::intern_raw_strings;
pub use passes::reachability::check_unreachable_code;
pub use pretty::format_module;
pub use state::*;
pub use test_metadata::*;
pub use traits::{class_vtable_symbol_name, trait_vtable_symbol_name};
pub use verify::{DebugEntity, VerifyError, verify_body};
