use std::path::Path;
use std::time::Instant;

use crate::typeck::TraitSolverMetrics;
use tracing::info;

pub(super) struct PipelineLogMetadata {
    pub(super) command: &'static str,
    pub(super) target: String,
    pub(super) backend: &'static str,
    pub(super) kind: &'static str,
    pub(super) input_count: usize,
    pub(super) load_stdlib: bool,
    pub(super) trait_solver_metrics: bool,
}

impl PipelineLogMetadata {
    pub(super) fn new(
        command: &'static str,
        target: String,
        backend: &'static str,
        kind: &'static str,
        input_count: usize,
        load_stdlib: bool,
        trait_solver_metrics: bool,
    ) -> Self {
        Self {
            command,
            target,
            backend,
            kind,
            input_count,
            load_stdlib,
            trait_solver_metrics,
        }
    }
}

pub(super) fn log_stage(
    trace_enabled: bool,
    metadata: &PipelineLogMetadata,
    stage: &str,
    start: Instant,
) {
    if trace_enabled {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        info!(
            target: "pipeline",
            stage = stage,
            command = metadata.command,
            status = "ok",
            target = %metadata.target,
            backend = metadata.backend,
            kind = metadata.kind,
            input_count = metadata.input_count,
            load_stdlib = metadata.load_stdlib,
            elapsed_ms
        );
    }
}

pub(super) fn log_stage_with_path(
    trace_enabled: bool,
    metadata: &PipelineLogMetadata,
    stage: &str,
    path: &Path,
    start: Instant,
) {
    if trace_enabled {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        info!(
            target: "pipeline",
            stage = stage,
            command = metadata.command,
            status = "ok",
            target = %metadata.target,
            backend = metadata.backend,
            kind = metadata.kind,
            input_count = metadata.input_count,
            load_stdlib = metadata.load_stdlib,
            path = %path.display(),
            elapsed_ms
        );
    }
}

pub(super) fn log_trait_solver_metrics(
    metadata: &PipelineLogMetadata,
    metrics: &TraitSolverMetrics,
) {
    info!(
        target: "pipeline",
        stage = "frontend.trait_solver",
        command = metadata.command,
        status = "ok",
        target = %metadata.target,
        backend = metadata.backend,
        kind = metadata.kind,
        input_count = metadata.input_count,
        load_stdlib = metadata.load_stdlib,
        metrics_enabled = metadata.trait_solver_metrics,
        impls_checked = metrics.impls_checked,
        overlaps = metrics.overlaps_detected,
        traits_checked = metrics.traits_checked,
        cycles_detected = metrics.cycles_detected,
        elapsed_ms = metrics.elapsed.as_millis() as u64
    );
}
