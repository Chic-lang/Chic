use std::collections::HashMap;

use super::super::{MirFunction, TypeConstraint};
use super::driver::LoweringDiagnostic;
use crate::mir::data::InternedStr;

#[derive(Default)]
pub(crate) struct LoweringCache {
    entries: HashMap<String, CachedLowering>,
    metrics: LoweringCacheMetrics,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LoweringCacheMetrics {
    pub hits: usize,
    pub misses: usize,
}

impl LoweringCacheMetrics {
    #[must_use]
    pub fn total(&self) -> usize {
        self.hits + self.misses
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            1.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[derive(Clone)]
pub(super) struct CachedLowering {
    pub functions: Vec<MirFunction>,
    pub diagnostics: Vec<LoweringDiagnostic>,
    pub constraints: Vec<TypeConstraint>,
    pub interned: Vec<InternedStr>,
}

impl LoweringCache {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn reset_run(&mut self) {
        self.metrics = LoweringCacheMetrics::default();
    }

    pub(super) fn metrics(&self) -> LoweringCacheMetrics {
        self.metrics
    }

    pub(super) fn lookup(&mut self, key: &str) -> Option<CachedLowering> {
        let entry = self.entries.get(key).cloned();
        if entry.is_some() {
            self.metrics.hits += 1;
        } else {
            self.metrics.misses += 1;
        }
        entry
    }

    pub(super) fn insert(&mut self, key: String, entry: CachedLowering) {
        self.entries.insert(key, entry);
    }
}
