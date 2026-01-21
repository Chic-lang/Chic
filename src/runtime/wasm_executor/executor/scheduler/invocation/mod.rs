mod float;
mod guards;
mod invoke;

use super::{LINEAR_MEMORY_HEAP_BASE, STACK_BASE_RED_ZONE};
use std::sync::Once;

const CALL_DEPTH_LIMIT: usize = 1024;
const ARC_HEADER_SIZE: u32 = 32;
const ARC_ALIGN_OFFSET: u32 = 12;
const ARC_HEADER_MIN_ALIGN: u32 = 1;
const ARC_TYPE_ID_OFFSET: u32 = 24;
static DEBUG_EXPORTS_ONCE: Once = Once::new();
