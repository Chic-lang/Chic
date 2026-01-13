#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

use serde::Serialize;

/// Accelerator metadata attached to `mir.json` and perf streams.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StreamMetadata {
    pub stream_id: u32,
    pub device_id: u16,
    pub memspace: String,
    pub events: Vec<EventMetadata>,
}

/// Event dependencies recorded for deterministic ordering.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EventMetadata {
    pub event_id: u32,
    pub kind: String,
}
