//! Emits metadata sidecar objects that embed compiler details into artifacts.

mod accelerator;
mod attributes;
mod builder;
mod cost;
mod debug;
mod emit;
mod functions;
mod populate;
mod reflection;
mod schema;
mod serialize;
mod types;

#[allow(unused_imports)]
pub(crate) use builder::build_metadata_bytes_with_caches;
#[cfg(test)]
pub(crate) use builder::{
    build_metadata_bytes, build_metadata_bytes_with_writer,
    build_metadata_bytes_with_writer_internal, metadata_payload,
};
#[allow(unused_imports)]
pub(crate) use emit::{metadata_object_path, write_metadata_object};
#[allow(unused_imports)]
pub(crate) use populate::{MetadataCaches, MetadataTelemetry};
pub(crate) use reflection::write_reflection_manifest;

#[cfg(test)]
mod tests;
