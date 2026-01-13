//! Shared performance metadata for tracing and cost annotations.

use crate::frontend::diagnostics::Span;
use crate::mir::StrId;
use crate::run_log::RunLog;
use serde::{Deserialize, Serialize};

/// Container for tracepoints and declared cost budgets extracted during lowering.
#[derive(Debug, Clone, Default)]
pub struct PerfMetadata {
    pub tracepoints: Vec<Tracepoint>,
    pub costs: Vec<CostModel>,
}

/// Declared cost budgets for a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostModel {
    pub function: String,
    pub cpu_budget_us: Option<u64>,
    pub gpu_budget_us: Option<u64>,
    pub mem_budget_bytes: Option<u64>,
    pub span: Option<Span>,
}

/// Trace metadata associated with a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracepoint {
    pub function: String,
    pub label: String,
    pub label_id: Option<StrId>,
    pub level: TraceLevel,
    pub trace_id: u64,
    pub span: Option<Span>,
    pub budget: Option<CostModel>,
}

/// Tracepoint verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceLevel {
    Perf,
    Debug,
}

impl TraceLevel {
    #[must_use]
    pub fn from_str(level: &str) -> Option<Self> {
        match level.to_ascii_lowercase().as_str() {
            "perf" | "performance" => Some(Self::Perf),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }
}

/// Deterministically derive a 64-bit trace identifier from a function and label.
#[must_use]
pub fn trace_id(function: &str, label: &str) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(function.as_bytes());
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

/// Persisted perf.json snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfSnapshot {
    pub version: String,
    pub target: String,
    pub runs: Vec<PerfRun>,
    #[serde(default)]
    pub summary: Option<ImpactSummary>,
}

/// A single profiling run captured in perf.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRun {
    pub profile: String,
    pub metrics: Vec<PerfMetric>,
    #[serde(default)]
    pub run_log: Option<RunLog>,
}

/// A single tracepoint measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfMetric {
    pub trace_id: u64,
    pub mir_id: String,
    pub label: String,
    pub cpu_us: f64,
    pub budget_cpu_us: Option<u64>,
    pub budget_gpu_us: Option<u64>,
    pub budget_mem_bytes: Option<u64>,
}

/// Resource-level summary attached to a profiling run.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ImpactSummary {
    pub profile: String,
    pub target: String,
    pub wall_time_ns: u128,
    #[serde(default)]
    pub cpu_user_ns: Option<u128>,
    #[serde(default)]
    pub cpu_system_ns: Option<u128>,
    #[serde(default)]
    pub max_rss_kb: Option<u64>,
    #[serde(default)]
    pub io_read_blocks: Option<u64>,
    #[serde(default)]
    pub io_write_blocks: Option<u64>,
    #[serde(default)]
    pub alloc: Option<AllocationSummary>,
    #[serde(default)]
    pub sampling_interval_ns: Option<u128>,
}

/// Allocation counters captured during a profiling session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AllocationSummary {
    pub alloc_calls: u64,
    pub alloc_zeroed_calls: u64,
    pub realloc_calls: u64,
    pub free_calls: u64,
    pub alloc_bytes: u64,
    pub alloc_zeroed_bytes: u64,
    pub realloc_bytes: u64,
    pub freed_bytes: u64,
}

impl PerfSnapshot {
    /// Return the run matching the provided profile, falling back to the first available run.
    #[must_use]
    pub fn run_by_profile(&self, profile: Option<&str>) -> Option<&PerfRun> {
        if let Some(profile) = profile {
            if let Some(run) = self.runs.iter().find(|run| run.profile == profile) {
                return Some(run);
            }
        }
        self.runs.first()
    }
}
