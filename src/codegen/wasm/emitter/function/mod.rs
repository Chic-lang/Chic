mod blocks;
mod borrows;
mod builder;
mod locals;
mod ops;
mod runtime;
mod statements;
mod terminators;
mod trace;
mod values;

#[allow(unused_imports)]
pub(crate) use builder::FunctionEmitter;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use builder::emit_function;
#[allow(unused_imports)]
pub(crate) use builder::emit_function_with_async;
#[allow(unused_imports)]
pub(crate) use locals::{LocalPlan, LocalRepresentation, plan_locals};
#[allow(unused_imports)]
pub(crate) use ops::{Op, emit_instruction};
#[allow(unused_imports)]
pub(crate) use values::{ElemSize, MemoryAccess, VecIndexAccess, VecIndexKind};
