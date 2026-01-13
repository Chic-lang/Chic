#![allow(dead_code)]

use std::collections::BTreeMap;

/// Registry mapping graph IDs to lowered launch sequences.
#[derive(Default, Debug, Clone)]
pub struct GraphRegistry {
    graphs: BTreeMap<String, Vec<String>>,
}

impl GraphRegistry {
    pub fn register(&mut self, id: impl Into<String>, launches: Vec<String>) {
        self.graphs.insert(id.into(), launches);
    }

    #[must_use]
    pub fn lookup(&self, id: &str) -> Option<&[String]> {
        self.graphs.get(id).map(|v| v.as_slice())
    }
}
