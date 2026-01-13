#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

use crate::codegen::llvm::intrinsics::select_copy_intrinsic;
use crate::codegen::llvm::linalg::{TensorLayout, emit_strided_copy};
use crate::codegen::llvm::memory::{TensorAllocPlan, TensorPlacement};

/// Small helper that assembles deterministic LLVM IR snippets for tensor ops.
#[derive(Default)]
pub struct TensorEmitter {
    ir: String,
}

impl TensorEmitter {
    pub fn new() -> Self {
        Self { ir: String::new() }
    }

    pub fn into_ir(self) -> String {
        self.ir
    }

    pub fn emit_alloc(&mut self, name: &str, plan: &TensorAllocPlan) {
        let placement = match plan.placement {
            TensorPlacement::Stack => "stack",
            TensorPlacement::Heap => "heap",
        };
        self.ir
            .push_str(&format!("; TensorAlloc {name} ({placement})\n"));
        self.ir.push_str(&format!(
            "  %{name} = call ptr @chic_tensor_alloc(i64 {}, i64 {}) ; memspace={}\n",
            plan.size_bytes, plan.align, plan.mem_space
        ));
    }

    pub fn emit_view(
        &mut self,
        view_name: &str,
        base_name: &str,
        base_layout: &TensorLayout,
        view_layout: &TensorLayout,
    ) {
        let offset = base_layout.offset_bytes + view_layout.offset_bytes;
        self.ir.push_str(&format!(
            "; TensorView {view_name} from {base_name} @ offset {} bytes\n",
            offset
        ));
        if !self.compatible_rank(base_layout, view_layout) {
            self.ir
                .push_str("  ; incompatible ranks or strides; diagnostic emitted upstream\n");
            return;
        }
        self.ir.push_str(&format!(
            "  %{view_name} = getelementptr i8, ptr %{base_name}, i64 {}\n",
            offset
        ));
        self.ir.push_str(&format!(
            "  ; view strides = {:?}, shape = {:?}\n",
            view_layout.strides, view_layout.shape
        ));
    }

    pub fn emit_copy(
        &mut self,
        dst_name: &str,
        dst: &TensorLayout,
        src_name: &str,
        src: &TensorLayout,
        element_size: usize,
    ) {
        self.ir.push_str(&format!(
            "; TensorCopy {dst_name} <- {src_name} (element size {element_size})\n"
        ));
        if !self.compatible_rank(dst, src) {
            self.ir
                .push_str("  ; rank/layout mismatch; fallback copy skipped\n");
            return;
        }
        let byte_len = element_size.saturating_mul(dst.element_count().min(src.element_count()));
        let intrinsic = select_copy_intrinsic(
            src.is_contiguous(element_size),
            dst.is_contiguous(element_size),
            dst.align.min(src.align),
        );
        if let Some(intrin) = intrinsic {
            self.ir.push_str(&format!(
                "  call void @{symbol}(ptr %{dst_name}, ptr %{src_name}, i64 {byte_len}, i1 false) ; align {}\n",
                intrin.requires_alignment,
                symbol = intrin.symbol
            ));
        } else {
            emit_strided_copy(&mut self.ir, dst, src, element_size, "tensor_copy");
        }
    }

    fn compatible_rank(&self, lhs: &TensorLayout, rhs: &TensorLayout) -> bool {
        lhs.shape.len() == rhs.shape.len() && lhs.strides.len() == rhs.strides.len()
    }
}
