#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Placement hint for tensor storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorPlacement {
    Stack,
    Heap,
}

/// Allocation plan chosen for a tensor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorAllocPlan {
    pub size_bytes: usize,
    pub align: usize,
    pub placement: TensorPlacement,
    pub mem_space: String,
}

/// Errors that can occur while planning a tensor allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorPlanError {
    Overflow,
    EmptyShape,
    ZeroElement,
}

/// Resolve the byte length for a tensor given its shape and element width.
pub fn byte_len(shape: &[usize], element_size: usize) -> Result<usize, TensorPlanError> {
    if element_size == 0 {
        return Err(TensorPlanError::ZeroElement);
    }
    if shape.is_empty() {
        return Err(TensorPlanError::EmptyShape);
    }
    let mut acc = element_size;
    for &dim in shape {
        acc = acc.checked_mul(dim).ok_or(TensorPlanError::Overflow)?;
    }
    Ok(acc)
}

/// Choose an allocation plan that respects explicit alignment and stack budget.
pub fn plan_tensor_alloc(
    shape: &[usize],
    element_size: usize,
    explicit_align: Option<usize>,
    mem_space: &str,
    prefer_stack: bool,
    stack_limit_bytes: usize,
) -> Result<TensorAllocPlan, TensorPlanError> {
    let size_bytes = byte_len(shape, element_size)?;
    let align = resolve_alignment(explicit_align, element_size);
    let placement = if prefer_stack && size_bytes <= stack_limit_bytes {
        TensorPlacement::Stack
    } else {
        TensorPlacement::Heap
    };

    Ok(TensorAllocPlan {
        size_bytes,
        align,
        placement,
        mem_space: mem_space.to_string(),
    })
}

/// Pick the deterministic alignment used for both allocation and freeing paths.
pub fn resolve_alignment(explicit_align: Option<usize>, element_align: usize) -> usize {
    match explicit_align {
        Some(0) => element_align.max(1),
        Some(val) => val,
        None => {
            let mut align = 1usize;
            while align < element_align {
                align <<= 1;
            }
            align
        }
    }
}
