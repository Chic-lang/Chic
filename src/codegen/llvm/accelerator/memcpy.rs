#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Simple compatibility checker for accelerator copies.
#[must_use]
pub fn validate_copy_layout(src_mem: &str, dst_mem: &str, alignment: usize) -> Result<(), String> {
    if src_mem != dst_mem {
        return Err(format!(
            "memspace mismatch: src={src_mem} dst={dst_mem} requires explicit staging buffer"
        ));
    }
    if alignment == 0 {
        return Err("alignment must be non-zero for accelerator copies".into());
    }
    Ok(())
}
