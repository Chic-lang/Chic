#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; Rust tracing runtime has been removed."
);

mod native {
    unsafe extern "C" {
        pub fn chic_rt_trace_enter(trace_id: u64, label_ptr: *const u8, label_len: u64);
        pub fn chic_rt_trace_exit(trace_id: u64);
        pub fn chic_rt_trace_flush(path_ptr: *const u8, len: u64) -> i32;
    }
}

pub use native::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fs;

    #[test]
    fn trace_flush_writes_label() {
        let dir = tempfile::tempdir().expect("tempdir");
        let output = dir.path().join("perf.json");
        let output_cstr =
            CString::new(output.to_string_lossy().as_bytes()).expect("path to CString");
        let output_bytes = output_cstr.as_bytes_with_nul();
        let label = b"Wasm::trace";
        unsafe {
            eprintln!("trace_enter");
            chic_rt_trace_enter(0xABCDEFu64, label.as_ptr(), label.len() as u64);
            chic_rt_trace_exit(0xABCDEFu64);
            let status = chic_rt_trace_flush(
                output_bytes.as_ptr() as *const u8,
                output_bytes.len() as u64,
            );
            eprintln!("trace_flush status {}", status);
            assert_eq!(status, 0, "trace_flush should succeed");
        }
        let body = fs::read_to_string(&output).expect("read perf.json");
        assert!(
            body.contains("Wasm::trace"),
            "perf output missing label: {}",
            body
        );
    }
}
