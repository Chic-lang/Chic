use std::fs;
use std::path::{Path, PathBuf};

use crate::chic_kind::ChicKind;
use crate::error::Error;
use crate::frontend::ast::Module;
use crate::mir::MirModule;
use crate::perf::PerfMetadata;
use crate::runtime_package::RuntimeMetadata;
use crate::target::Target;

use super::builder::build_metadata_bytes_with_caches;
use super::populate::MetadataTelemetry;

/// Compute the deterministic path used for the metadata object associated with an output artifact.
pub(crate) fn metadata_object_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let meta_name = format!("{file_name}.meta.o");
    output.with_file_name(meta_name)
}

/// Write the metadata object for the given artifact and return the path that was written.
pub(crate) fn write_metadata_object(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
    output: &Path,
) -> Result<(PathBuf, MetadataTelemetry), Error> {
    let path = metadata_object_path(output);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let (bytes, telemetry) = build_metadata_bytes_with_caches(
        module,
        mir,
        perf,
        target,
        canonical_triple,
        kind,
        runtime,
    )?;
    fs::write(&path, bytes)?;
    Ok((path, telemetry))
}
