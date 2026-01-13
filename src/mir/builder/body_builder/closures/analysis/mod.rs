use super::super::*;

mod collect;
mod pattern;

pub(crate) use collect::analyze_captures;

#[derive(Clone, Debug)]
pub(crate) struct CapturedLocal {
    pub(crate) name: String,
    pub(crate) local: LocalId,
    pub(crate) ty: Ty,
    pub(crate) is_nullable: bool,
    pub(crate) is_mutable: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum LambdaLoweringBody {
    Expression(Box<ExprNode>),
    Block(AstBlock),
}

use crate::frontend::ast::Block as AstBlock;
use crate::syntax::expr::ExprNode;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct CaptureCache {
    entries: HashMap<String, Vec<CapturedLocal>>,
    hits: u64,
    misses: u64,
}

#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct CaptureCacheMetrics {
    pub hits: u64,
    pub misses: u64,
}

impl CaptureCache {
    pub(crate) fn get(&mut self, key: &str) -> Option<Vec<CapturedLocal>> {
        if let Some(existing) = self.entries.get(key) {
            self.hits += 1;
            Some(existing.clone())
        } else {
            None
        }
    }

    pub(crate) fn insert(&mut self, key: String, captures: Vec<CapturedLocal>) {
        self.misses += 1;
        self.entries.insert(key, captures);
    }

    pub(crate) fn metrics(&self) -> CaptureCacheMetrics {
        CaptureCacheMetrics {
            hits: self.hits,
            misses: self.misses,
        }
    }
}

pub(crate) fn capture_cache_key(body: &LambdaLoweringBody) -> String {
    format!("{:?}", body)
}
