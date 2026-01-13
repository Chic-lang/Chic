use object::write::Object;

use crate::chic_kind::ChicKind;
use crate::error::Error;
use crate::frontend::ast::Module;
use crate::frontend::metadata::{collect_reflection_tables, serialize_reflection_tables};
use crate::mir::MirModule;
use crate::perf::PerfMetadata;
use crate::runtime_package::RuntimeMetadata;
use crate::target::Target;

use super::populate::{self, MetadataCaches, MetadataTelemetry};
use super::serialize;

pub(crate) fn build_metadata_bytes_with_caches(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
) -> Result<(Vec<u8>, MetadataTelemetry), Error> {
    let mut caches = MetadataCaches::default();
    let bytes = build_metadata_bytes_with_writer_internal(
        module,
        mir,
        perf,
        target,
        canonical_triple,
        kind,
        runtime,
        &mut caches,
        |object| object.write(),
    )?;
    let telemetry = caches.telemetry();
    let _ = telemetry.types.cached_entries + telemetry.functions.cached_entries;
    Ok((bytes, telemetry))
}

pub(crate) fn build_metadata_bytes_with_writer_internal<F, E>(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
    caches: &mut MetadataCaches,
    writer: F,
) -> Result<Vec<u8>, Error>
where
    F: FnOnce(&Object<'_>) -> Result<Vec<u8>, E>,
    E: std::fmt::Display,
{
    let payload = populate::build_metadata_payload(
        module,
        mir,
        perf,
        target,
        canonical_triple,
        kind,
        runtime,
        caches,
    )
    .map_err(|err| {
        Error::Codegen(format!(
            "failed to assemble metadata payload for {}: {:?}",
            target.triple(),
            err
        ))
    })?;

    let reflection_json =
        serialize_reflection_tables(&collect_reflection_tables(module)).map_err(|err| {
            Error::Codegen(format!(
                "failed to serialise reflection metadata for {}: {err}",
                target.triple()
            ))
        })?;

    serialize::build_metadata_object_bytes(
        payload,
        Some(reflection_json.as_bytes()),
        target,
        canonical_triple,
        kind,
        writer,
    )
}

#[cfg(test)]
pub(crate) fn build_metadata_bytes_with_writer<F, E>(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
    writer: F,
) -> Result<Vec<u8>, Error>
where
    F: FnOnce(&Object<'_>) -> Result<Vec<u8>, E>,
    E: std::fmt::Display,
{
    let mut caches = MetadataCaches::default();
    build_metadata_bytes_with_writer_internal(
        module,
        mir,
        perf,
        target,
        canonical_triple,
        kind,
        runtime,
        &mut caches,
        writer,
    )
}

#[cfg(test)]
pub(crate) fn build_metadata_bytes(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
) -> Result<Vec<u8>, Error> {
    build_metadata_bytes_with_writer(
        module,
        mir,
        perf,
        target,
        canonical_triple,
        kind,
        runtime,
        |object| object.write(),
    )
}

#[cfg(test)]
pub(crate) fn metadata_payload(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
    caches: &mut MetadataCaches,
) -> Vec<u8> {
    populate::build_metadata_payload(
        module,
        mir,
        perf,
        target,
        canonical_triple,
        kind,
        runtime,
        caches,
    )
    .expect("metadata payload")
    .into_bytes()
}
