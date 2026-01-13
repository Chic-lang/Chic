//! Populate metadata payloads from frontend and MIR structures.

use std::fmt::Write;

use crate::chic_kind::ChicKind;
use crate::frontend::ast::Module;
use crate::mir::{MirModule, TypeLayout, TypeLayoutTable};
use crate::perf::PerfMetadata;
use crate::runtime_package::RuntimeMetadata;
use crate::target::Target;

use super::attributes;
use super::cost;
use super::functions;
use super::schema::{MetadataHeader, MetadataPayload, MetadataWriter, SchemaError};
use super::types;

#[derive(Debug, Default)]
pub(crate) struct MetadataCaches {
    types: types::TypeMetadataCache,
    functions: functions::FunctionMetadataCache,
}

impl MetadataCaches {
    pub(crate) fn types_mut(&mut self) -> &mut types::TypeMetadataCache {
        &mut self.types
    }

    pub(crate) fn functions_mut(&mut self) -> &mut functions::FunctionMetadataCache {
        &mut self.functions
    }

    pub(crate) fn telemetry(&self) -> MetadataTelemetry {
        MetadataTelemetry {
            types: self.types.telemetry(),
            functions: self.functions.telemetry(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MetadataTelemetry {
    pub(crate) types: types::TypeMetadataTelemetry,
    pub(crate) functions: functions::FunctionMetadataTelemetry,
}

pub(crate) fn build_metadata_payload(
    module: &Module,
    mir: &MirModule,
    perf: &PerfMetadata,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    runtime: Option<&RuntimeMetadata>,
    caches: &mut MetadataCaches,
) -> Result<MetadataPayload, SchemaError> {
    let header = MetadataHeader::new(
        env!("CARGO_PKG_VERSION"),
        target.triple(),
        canonical_triple,
        kind,
        runtime.cloned(),
    );
    let mut writer = MetadataWriter::default();
    attributes::append_module_attributes(&mut writer, mir);
    functions::append_export_metadata(&mut writer, &mir.exports, caches.functions_mut());
    functions::append_extern_metadata(&mut writer, &mir.functions);
    functions::append_default_argument_metadata(&mut writer, &mir.default_arguments);
    functions::append_lending_metadata(&mut writer, &mir.functions);
    append_view_metadata(&mut writer, &mir.type_layouts);
    attributes::append_doc_metadata(&mut writer, module, caches.types_mut());
    attributes::append_inline_metadata(&mut writer, module);
    cost::append_trace_metadata(&mut writer, perf);
    cost::append_cost_metadata(&mut writer, perf);
    Ok(MetadataPayload::new(header, writer))
}

fn append_view_metadata(payload: &mut MetadataWriter, layouts: &TypeLayoutTable) {
    for layout in layouts.types.values() {
        if let TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) = layout {
            for field in &struct_layout.fields {
                if let Some(owner) = &field.view_of {
                    let qualified = format!("{}::{}", struct_layout.name, field.name);
                    let _ = writeln!(payload, "view:{qualified}=of:{owner}");
                }
            }
        }
    }
}
