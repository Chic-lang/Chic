#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Linear-memory allocation plan for tensors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmTensorAllocPlan {
    pub base: u32,
    pub size: u32,
    pub align: u32,
    pub use_heap: bool,
    pub mem_space: String,
}

/// Compute tensor size in bytes, returning `None` on overflow.
pub fn tensor_size_bytes(shape: &[u32], element_size: u32) -> Option<u32> {
    if shape.is_empty() || element_size == 0 {
        return None;
    }
    let mut acc = element_size;
    for &dim in shape {
        acc = acc.checked_mul(dim)?;
    }
    Some(acc)
}

/// Align `value` up to `align` bytes (align must be power-of-two).
pub fn align_up(value: u32, align: u32) -> u32 {
    if align <= 1 {
        value
    } else {
        (value + (align - 1)) & !(align - 1)
    }
}

/// Check that `[base, base + size)` sits within the linear-memory limit.
pub fn bounds_check(base: u32, size: u32, memory_limit: u32) -> bool {
    base.checked_add(size)
        .map_or(false, |end| end <= memory_limit)
}

/// Choose a deterministic plan for allocating tensor storage in linear memory.
pub fn plan_tensor_allocation(
    shape: &[u32],
    element_size: u32,
    explicit_align: Option<u32>,
    stack_pointer: u32,
    memory_limit: u32,
    prefer_stack: bool,
    mem_space: &str,
) -> Option<WasmTensorAllocPlan> {
    let mut size = tensor_size_bytes(shape, element_size)?;
    let align = explicit_align.unwrap_or(element_size.max(1));
    size = align_up(size, align);
    let use_stack = prefer_stack && bounds_check(stack_pointer, size, memory_limit);
    let base = if use_stack { stack_pointer } else { 0 };
    Some(WasmTensorAllocPlan {
        base,
        size,
        align,
        use_heap: !use_stack,
        mem_space: mem_space.to_string(),
    })
}
