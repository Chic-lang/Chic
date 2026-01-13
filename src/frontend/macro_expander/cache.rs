use crate::frontend::ast::Item;
use crate::frontend::diagnostics::Diagnostic;
use std::collections::HashMap;

use super::model::InvocationCacheKey;

#[derive(Default, Clone)]
pub struct CacheMetrics {
    pub hits: usize,
    pub misses: usize,
}

#[derive(Clone)]
pub struct CachedExpansion {
    pub items: Vec<Item>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CachedExpansion {
    #[must_use]
    pub fn new(items: Vec<Item>, diagnostics: Vec<Diagnostic>) -> Self {
        Self { items, diagnostics }
    }
}

#[derive(Default)]
pub struct ExpansionCache {
    entries: HashMap<InvocationCacheKey, CachedExpansion>,
    metrics: CacheMetrics,
}

impl ExpansionCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lookup(&mut self, key: &InvocationCacheKey) -> Option<CachedExpansion> {
        match self.entries.get(key) {
            Some(entry) => {
                self.metrics.hits += 1;
                Some(entry.clone())
            }
            None => {
                self.metrics.misses += 1;
                None
            }
        }
    }

    pub fn store(&mut self, key: InvocationCacheKey, expansion: CachedExpansion) {
        self.entries.insert(key, expansion);
    }

    #[must_use]
    pub fn metrics(&self) -> CacheMetrics {
        self.metrics.clone()
    }
}
