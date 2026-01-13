#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

use crate::codegen::wasm::memory::{WasmTensorAllocPlan, bounds_check, plan_tensor_allocation};

/// Layout metadata used by the WASM tensor fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmTensorLayout {
    pub shape: Vec<u32>,
    pub strides: Vec<i32>,
    pub offset_bytes: i32,
    pub align: u32,
    pub mem_space: String,
}

impl WasmTensorLayout {
    pub fn element_count(&self) -> u32 {
        self.shape.iter().copied().product()
    }

    pub fn is_contiguous(&self, element_size: u32) -> bool {
        if self.shape.is_empty() {
            return false;
        }
        let mut expected = element_size as i32;
        for (&dim, &stride) in self.shape.iter().rev().zip(self.strides.iter().rev()) {
            if dim == 0 || stride != expected {
                return false;
            }
            expected = expected.saturating_mul(dim as i32);
        }
        true
    }
}

/// Pseudo-WAT emitter for tensor ops in the WASM backend.
#[derive(Default)]
pub struct WasmTensorEmitter {
    wat: String,
}

impl WasmTensorEmitter {
    pub fn new() -> Self {
        Self { wat: String::new() }
    }

    pub fn into_wat(self) -> String {
        self.wat
    }

    pub fn emit_alloc(&mut self, name: &str, plan: &WasmTensorAllocPlan, memory_limit: u32) {
        self.wat.push_str(&format!(
            ";; TensorAlloc {name} memspace={} align={}\n",
            plan.mem_space, plan.align
        ));
        if !bounds_check(plan.base, plan.size, memory_limit) {
            self.wat
                .push_str("  unreachable ;; bounds check failed for tensor alloc\n");
            return;
        }
        let alloc_kind = if plan.use_heap { "heap" } else { "stack" };
        self.wat.push_str(&format!(
            "  ;; allocate {alloc_kind} bytes={} align={}\n",
            plan.size, plan.align
        ));
        self.wat.push_str(&format!(
            "  (local.set ${name}_ptr (i32.const {}))\n",
            plan.base
        ));
    }

    pub fn emit_view(
        &mut self,
        view_name: &str,
        base_name: &str,
        view_layout: &WasmTensorLayout,
        base_layout: &WasmTensorLayout,
    ) {
        let offset = base_layout.offset_bytes + view_layout.offset_bytes;
        if base_layout.shape.len() != view_layout.shape.len() {
            self.wat
                .push_str("  unreachable ;; incompatible view rank\n");
            return;
        }
        self.wat.push_str(&format!(
            ";; TensorView {view_name} from {base_name} offset={offset}\n"
        ));
        self.wat.push_str(&format!(
            "  (local.set ${view_name}_ptr (i32.add (local.get ${base_name}_ptr) (i32.const {offset})))\n"
        ));
        self.wat.push_str(&format!(
            "  ;; strides {:?} shape {:?}\n",
            view_layout.strides, view_layout.shape
        ));
    }

    pub fn emit_copy(
        &mut self,
        dst_name: &str,
        dst: &WasmTensorLayout,
        src_name: &str,
        src: &WasmTensorLayout,
        element_size: u32,
    ) {
        self.wat.push_str(&format!(
            ";; TensorCopy {dst_name} <- {src_name} elem_size={element_size}\n"
        ));
        if dst.shape.len() != src.shape.len() {
            self.wat
                .push_str("  unreachable ;; rank mismatch for tensor copy\n");
            return;
        }
        let byte_len = element_size.saturating_mul(dst.element_count().min(src.element_count()));
        if dst.is_contiguous(element_size) && src.is_contiguous(element_size) {
            self.wat.push_str(&format!(
                "  (memory.copy (local.get ${dst_name}_ptr) (local.get ${src_name}_ptr) (i32.const {byte_len}))\n"
            ));
        } else {
            self.emit_strided_loops(dst, src, element_size);
        }
    }

    fn emit_strided_loops(
        &mut self,
        dst: &WasmTensorLayout,
        src: &WasmTensorLayout,
        element_size: u32,
    ) {
        for (depth, dim) in dst.shape.iter().enumerate() {
            self.wat
                .push_str(&format!("  (loop ;; depth {depth} dim {dim}\n"));
        }
        self.wat.push_str(&format!(
            "    ;; body: load/store with strides dst={:?} src={:?} elem={element_size}\n",
            dst.strides, src.strides
        ));
        for _ in &dst.shape {
            self.wat.push_str("  )\n");
        }
    }
}

/// Convenience helper for planning + emitting an allocation in one shot.
pub fn plan_and_emit_alloc(
    emitter: &mut WasmTensorEmitter,
    name: &str,
    shape: &[u32],
    element_size: u32,
    align: u32,
    stack_pointer: u32,
    memory_limit: u32,
    prefer_stack: bool,
    mem_space: &str,
) {
    if let Some(plan) = plan_tensor_allocation(
        shape,
        element_size,
        Some(align),
        stack_pointer,
        memory_limit,
        prefer_stack,
        mem_space,
    ) {
        emitter.emit_alloc(name, &plan, memory_limit);
    } else {
        emitter
            .wat
            .push_str("  unreachable ;; tensor alloc planning failed\n");
    }
}
