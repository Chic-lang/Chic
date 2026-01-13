#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Layout metadata used by tensor lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorLayout {
    pub shape: Vec<usize>,
    pub strides: Vec<isize>,
    pub offset_bytes: isize,
    pub align: usize,
    pub mem_space: String,
    pub layout_id: String,
}

impl TensorLayout {
    pub fn contiguous(element_size: usize, shape: &[usize]) -> Self {
        let mut strides = vec![0isize; shape.len()];
        let mut stride = element_size as isize;
        for (idx, &dim) in shape.iter().rev().enumerate() {
            let index = shape.len() - 1 - idx;
            strides[index] = stride;
            stride = stride.saturating_mul(dim as isize);
        }
        TensorLayout {
            shape: shape.to_vec(),
            strides,
            offset_bytes: 0,
            align: element_size.max(1),
            mem_space: "host".into(),
            layout_id: "row-major".into(),
        }
    }

    pub fn element_count(&self) -> usize {
        self.shape.iter().copied().product()
    }

    pub fn is_contiguous(&self, element_size: usize) -> bool {
        if self.shape.is_empty() {
            return false;
        }
        let mut expected = element_size as isize;
        for (&dim, &stride) in self.shape.iter().rev().zip(self.strides.iter().rev()) {
            if dim == 0 {
                return false;
            }
            if stride != expected {
                return false;
            }
            expected = expected.saturating_mul(dim as isize);
        }
        true
    }
}

/// Emit a simple strided copy loop nest into the provided buffer.
pub fn emit_strided_copy(
    ir: &mut String,
    dst: &TensorLayout,
    src: &TensorLayout,
    element_size: usize,
    tmp_prefix: &str,
) {
    let rank = dst.shape.len();
    for depth in 0..rank {
        let label = format!("{tmp_prefix}_loop_{depth}");
        let header = format!("{tmp_prefix}_loop_header_{depth}");
        ir.push_str(&format!("{header}:\n"));
        ir.push_str(&format!(
            "  ; loop depth {depth} over {} elements\n",
            dst.shape[depth]
        ));
        ir.push_str(&format!("{label}:\n"));
    }
    ir.push_str("  ; body: load strided src and store into dst\n");
    ir.push_str(&format!(
        "  ; element size = {element_size} bytes, dst_stride = {:?}, src_stride = {:?}\n",
        dst.strides, src.strides
    ));
    ir.push_str(&format!(
        "  ; offsets dst={}, src={}\n",
        dst.offset_bytes, src.offset_bytes
    ));
    ir.push_str("  ; end of tensor copy loop nest\n");
}
