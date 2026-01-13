//! Type metadata caching primitives shared across backends.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// Identifier for a cached type metadata entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TypeMetadataFingerprint(pub(crate) String);

impl TypeMetadataFingerprint {
    /// Create a new fingerprint from the provided string payload.
    pub(crate) fn new<S: Into<String>>(fingerprint: S) -> Self {
        Self(fingerprint.into())
    }

    /// View the fingerprint contents.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cached descriptor for a type metadata payload (placeholder for future detail).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeMetadataEntry {
    pub(crate) qualified_name: String,
}

impl TypeMetadataEntry {
    pub(crate) fn new(name: String) -> Self {
        Self {
            qualified_name: name,
        }
    }
}

/// Cache that tracks entries by fingerprint while exposing hit/miss telemetry.
#[derive(Debug, Default)]
pub(crate) struct TypeMetadataCache {
    entries: HashMap<TypeMetadataFingerprint, TypeMetadataEntry>,
    hit_count: usize,
    miss_count: usize,
}

impl TypeMetadataCache {
    /// Fetch an entry if present, otherwise insert using the provided builder.
    pub(crate) fn ensure_with<F>(
        &mut self,
        fingerprint: TypeMetadataFingerprint,
        builder: F,
    ) -> &TypeMetadataEntry
    where
        F: FnOnce(&TypeMetadataFingerprint) -> TypeMetadataEntry,
    {
        match self.entries.entry(fingerprint) {
            Entry::Occupied(entry) => {
                self.hit_count += 1;
                entry.into_mut()
            }
            Entry::Vacant(slot) => {
                self.miss_count += 1;
                let fingerprint = slot.key().clone();
                slot.insert(builder(&fingerprint))
            }
        }
    }

    /// Summarise cache activity for telemetry/CI reporting.
    pub(crate) fn telemetry(&self) -> TypeMetadataTelemetry {
        TypeMetadataTelemetry {
            type_hits: self.hit_count,
            type_misses: self.miss_count,
            cached_entries: self.entries.len(),
        }
    }
}

/// Snapshot of cache statistics.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeMetadataTelemetry {
    pub(crate) type_hits: usize,
    pub(crate) type_misses: usize,
    pub(crate) cached_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_with_tracks_hits_and_misses() {
        let mut cache = TypeMetadataCache::default();
        let fp = TypeMetadataFingerprint::new("MyType");
        let entry = cache.ensure_with(fp.clone(), |fingerprint| {
            TypeMetadataEntry::new(fingerprint.as_str().to_string())
        });
        assert_eq!(entry.qualified_name, "MyType");
        let telemetry = cache.telemetry();
        assert_eq!(telemetry.type_hits, 0);
        assert_eq!(telemetry.type_misses, 1);
        assert_eq!(telemetry.cached_entries, 1);

        let entry_again = cache.ensure_with(fp, |fingerprint| {
            TypeMetadataEntry::new(format!("{}_second", fingerprint.as_str()))
        });
        assert_eq!(entry_again.qualified_name, "MyType");
        let telemetry = cache.telemetry();
        assert_eq!(telemetry.type_hits, 1);
        assert_eq!(telemetry.type_misses, 1);
        assert_eq!(telemetry.cached_entries, 1);
    }
}
