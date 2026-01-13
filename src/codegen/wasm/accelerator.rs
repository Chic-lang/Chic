#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Recorded accelerator operations for the WASM stub.
#[derive(Default, Debug, Clone)]
pub struct WasmAcceleratorLog {
    ops: Vec<String>,
}

impl WasmAcceleratorLog {
    pub fn enqueue_kernel(&mut self, stream: u32, kernel: &str) {
        self.ops
            .push(format!("enqueue_kernel stream={stream} kernel={kernel}"));
    }

    pub fn enqueue_copy(&mut self, stream: u32, bytes: u32) {
        self.ops
            .push(format!("enqueue_copy stream={stream} bytes={bytes}"));
    }

    pub fn record_event(&mut self, stream: u32, event: u32) {
        self.ops
            .push(format!("record_event stream={stream} event={event}"));
    }

    pub fn wait_event(&mut self, stream: Option<u32>, event: u32) {
        let stream = stream.map_or_else(|| "none".into(), |s| s.to_string());
        self.ops
            .push(format!("wait_event stream={stream} event={event}"));
    }

    #[must_use]
    pub fn ordered(self) -> Vec<String> {
        self.ops
    }
}
