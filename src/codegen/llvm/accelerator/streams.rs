#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Mock stream logger used for deterministic ordering tests.
#[derive(Default, Debug, Clone)]
pub struct StreamLog {
    ops: Vec<String>,
}

impl StreamLog {
    pub fn record_enqueue_kernel(&mut self, stream_id: u32, kernel: &str, event: Option<u32>) {
        let evt = event.map_or_else(|| "none".into(), |e| e.to_string());
        self.ops.push(format!(
            "enqueue_kernel stream={stream_id} kernel={kernel} event={evt}"
        ));
    }

    pub fn record_enqueue_copy(&mut self, stream_id: u32, bytes: usize, event: Option<u32>) {
        let evt = event.map_or_else(|| "none".into(), |e| e.to_string());
        self.ops.push(format!(
            "enqueue_copy stream={stream_id} bytes={bytes} event={evt}"
        ));
    }

    pub fn record_event(&mut self, stream_id: u32, event_id: u32) {
        self.ops
            .push(format!("record_event stream={stream_id} event={event_id}"));
    }

    pub fn wait_event(&mut self, stream_id: Option<u32>, event_id: u32) {
        let stream = stream_id.map_or_else(|| "none".into(), |s| s.to_string());
        self.ops
            .push(format!("wait_event stream={stream} event={event_id}"));
    }

    #[must_use]
    pub fn ordered(self) -> Vec<String> {
        self.ops
    }
}
