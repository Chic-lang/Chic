use crate::error::Error;
use crate::mir::{
    ArrayTy, FunctionKind, LocalId, ParamMode, Place, ProjectionElem, Ty, TypeLayout,
};

use super::super::builder::FunctionEmitter;
use super::super::values::ValueRef;

mod alignment;
mod metadata;
mod pointer;
mod resolve;
#[cfg(test)]
mod tests;

const VEC_BOUNDS_PANIC_CODE: i32 = 0x2001;
const ARRAY_BOUNDS_PANIC_CODE: i32 = 0x2002;
const SPAN_BOUNDS_PANIC_CODE: i32 = 0x2003;
const READONLY_SPAN_BOUNDS_PANIC_CODE: i32 = 0x2004;
const STRING_BOUNDS_PANIC_CODE: i32 = 0x2005;
const STR_BOUNDS_PANIC_CODE: i32 = 0x2006;

enum InlineElemSize {
    Dynamic(String),
    Const(u64),
}
