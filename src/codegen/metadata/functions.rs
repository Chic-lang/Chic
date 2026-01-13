//! Function metadata helpers shared across backends.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Write;

use super::schema::MetadataWriter;
use crate::mir::module_metadata::Export;
use crate::mir::{DefaultArgumentKind, DefaultArgumentRecord, MirFunction};

/// Descriptor for exported function metadata entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionMetadataDescriptor {
    pub(crate) name: String,
    pub(crate) symbol: String,
}

impl FunctionMetadataDescriptor {
    /// Construct a descriptor for a function export.
    pub(crate) fn new(name: String, symbol: String) -> Self {
        Self { name, symbol }
    }
}

/// Cache that records per-function metadata hits/misses.
#[derive(Debug, Default)]
pub(crate) struct FunctionMetadataCache {
    entries: HashMap<String, FunctionMetadataDescriptor>,
    hit_count: usize,
    miss_count: usize,
}

impl FunctionMetadataCache {
    /// Register an export descriptor and return the cached entry.
    pub(crate) fn ensure(
        &mut self,
        descriptor: FunctionMetadataDescriptor,
    ) -> &FunctionMetadataDescriptor {
        match self.entries.entry(descriptor.name.clone()) {
            Entry::Occupied(entry) => {
                self.hit_count += 1;
                entry.into_mut()
            }
            Entry::Vacant(slot) => {
                self.miss_count += 1;
                slot.insert(descriptor)
            }
        }
    }

    /// Summarise cache activity for telemetry reporting.
    pub(crate) fn telemetry(&self) -> FunctionMetadataTelemetry {
        FunctionMetadataTelemetry {
            function_hits: self.hit_count,
            function_misses: self.miss_count,
            cached_entries: self.entries.len(),
        }
    }
}

/// Snapshot of cache statistics.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionMetadataTelemetry {
    pub(crate) function_hits: usize,
    pub(crate) function_misses: usize,
    pub(crate) cached_entries: usize,
}

/// Append export metadata entries to the payload while updating the cache.
pub(crate) fn append_export_metadata(
    payload: &mut MetadataWriter,
    exports: &[Export],
    cache: &mut FunctionMetadataCache,
) {
    for export in exports {
        let descriptor =
            FunctionMetadataDescriptor::new(export.function.clone(), export.symbol.clone());
        let cached = cache.ensure(descriptor);
        let _ = writeln!(payload, "export:{}={}", cached.name, cached.symbol);
    }
}

/// Append extern metadata entries describing dynamic bindings.
pub(crate) fn append_extern_metadata(payload: &mut MetadataWriter, functions: &[MirFunction]) {
    for function in functions {
        let Some(spec) = &function.extern_spec else {
            continue;
        };
        let mut parts = Vec::new();
        parts.push(format!("convention={}", spec.convention));
        parts.push(format!("binding={}", spec.binding));
        if let Some(library) = &spec.library {
            parts.push(format!("library={library}"));
        }
        if let Some(alias) = &spec.alias {
            parts.push(format!("alias={alias}"));
        }
        if spec.optional {
            parts.push("optional=true".to_string());
        }
        if let Some(charset) = &spec.charset {
            parts.push(format!("charset={charset}"));
        }
        let joined = parts.join(";");
        let _ = writeln!(payload, "extern:{}={}", function.name, joined);
    }
}

/// Append lending return metadata so tooling can surface borrow relationships.
pub(crate) fn append_lending_metadata(payload: &mut MetadataWriter, functions: &[MirFunction]) {
    for function in functions {
        if let Some(lenders) = &function.signature.lends_to_return {
            let joined = lenders.join(",");
            let _ = writeln!(payload, "lends_return:{}={}", function.name, joined);
        }
    }
}

/// Append default argument metadata entries for tooling/debugging.
pub(crate) fn append_default_argument_metadata(
    payload: &mut MetadataWriter,
    defaults: &[DefaultArgumentRecord],
) {
    for record in defaults {
        let descriptor = match &record.value {
            DefaultArgumentKind::Const(value) => format!("const:{value:?}"),
            DefaultArgumentKind::Thunk {
                symbol,
                metadata_count,
            } => format!("thunk:{symbol};meta={metadata_count}"),
        };
        let _ = writeln!(
            payload,
            "default_arg:{}#{}={}|{}|internal={}",
            record.function, record.param_index, record.param_name, descriptor, record.internal
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_tracks_hits_and_misses() {
        let mut cache = FunctionMetadataCache::default();
        let descriptor = FunctionMetadataDescriptor::new("My::Func".into(), "_ZNMyFunc".into());
        let entry = cache.ensure(descriptor.clone());
        assert_eq!(entry.name, "My::Func");
        assert_eq!(entry.symbol, "_ZNMyFunc");
        let telemetry = cache.telemetry();
        assert_eq!(telemetry.function_hits, 0);
        assert_eq!(telemetry.function_misses, 1);
        assert_eq!(telemetry.cached_entries, 1);

        let entry_again = cache.ensure(descriptor);
        assert_eq!(entry_again.name, "My::Func");
        let telemetry = cache.telemetry();
        assert_eq!(telemetry.function_hits, 1);
        assert_eq!(telemetry.function_misses, 1);
        assert_eq!(telemetry.cached_entries, 1);
    }
}
