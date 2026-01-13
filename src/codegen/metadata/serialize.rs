//! Metadata object/section serialization helpers.

use std::str::FromStr;

use object::macho;
use object::write::{MachOBuildVersion, Object, StandardSegment, Symbol, SymbolSection};
use object::{BinaryFormat, SectionKind, SymbolFlags, SymbolKind, SymbolScope};
use target_lexicon::Triple;

use crate::chic_kind::ChicKind;
use crate::error::Error;
use crate::target::Target;

use super::debug;
use super::schema::MetadataPayload;

pub(crate) fn build_metadata_object_bytes<F, E>(
    payload: MetadataPayload,
    reflection_payload: Option<&[u8]>,
    target: &Target,
    canonical_triple: &str,
    kind: ChicKind,
    writer: F,
) -> Result<Vec<u8>, Error>
where
    F: FnOnce(&Object<'_>) -> Result<Vec<u8>, E>,
    E: std::fmt::Display,
{
    let triple = Triple::from_str(canonical_triple).map_err(|err| {
        Error::Codegen(format!(
            "failed to parse target triple '{canonical_triple}': {err}"
        ))
    })?;

    let (format, arch, endianness) = debug::map_triple(&triple).ok_or_else(|| {
        let requested = target.triple();
        Error::Codegen(format!(
            "unsupported target triple '{requested}' for metadata emission"
        ))
    })?;

    let mut object = Object::new(format, arch, endianness);
    if format == BinaryFormat::MachO {
        if let Some((minos, sdk)) = debug::macos_build_version(&triple.operating_system) {
            let mut build_version = MachOBuildVersion::default();
            build_version.platform = macho::PLATFORM_MACOS;
            build_version.minos = minos;
            build_version.sdk = sdk;
            object.set_macho_build_version(build_version);
        }
    }
    let segment = object.segment_name(StandardSegment::Data).to_vec();
    let section_name = debug::section_name_for_format(format);
    let section = object.add_section(segment.clone(), section_name, SectionKind::ReadOnlyData);

    let payload = payload.into_bytes();
    let offset = object.append_section_data(section, &payload, 1);

    let symbol = Symbol {
        name: b"__chic_metadata".to_vec(),
        value: offset,
        size: payload.len() as u64,
        kind: SymbolKind::Data,
        // Keep metadata local to each object so multi-input builds don't collide on the symbol name.
        scope: SymbolScope::Compilation,
        weak: false,
        section: SymbolSection::Section(section),
        flags: SymbolFlags::None,
    };
    object.add_symbol(symbol);

    if let Some(payload) = reflection_payload {
        let section_name = debug::reflection_section_name_for_format(format);
        let section = object.add_section(segment, section_name, SectionKind::ReadOnlyData);
        let offset = object.append_section_data(section, payload, 1);
        let symbol = Symbol {
            name: b"__chic_reflection".to_vec(),
            value: offset,
            size: payload.len() as u64,
            kind: SymbolKind::Data,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Section(section),
            flags: SymbolFlags::None,
        };
        object.add_symbol(symbol);
    }

    writer(&object).map_err(|err| {
        let requested = target.triple();
        Error::Codegen(format!(
            "failed to serialise metadata object for '{requested}' (kind={}): {err}",
            kind.as_str()
        ))
    })
}
