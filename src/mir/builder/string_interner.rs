use std::collections::HashMap;

use crate::frontend::diagnostics::Span;
use crate::mir::data::{InternedStr, StrId, StrLifetime};

#[derive(Default)]
pub(crate) struct StringInterner {
    map: HashMap<String, StrId>,
    segments: Vec<InternedStr>,
}

impl StringInterner {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn intern(
        &mut self,
        value: &str,
        lifetime: StrLifetime,
        span: Option<Span>,
    ) -> StrId {
        if let Some(&id) = self.map.get(value) {
            return id;
        }
        let id = StrId::new(self.segments.len() as u32);
        let stored = value.to_string();
        let interned = InternedStr {
            id,
            value: stored.clone(),
            lifetime,
            span,
        };
        self.segments.push(interned);
        self.map.insert(stored, id);
        id
    }

    #[allow(dead_code)]
    pub(super) fn get(&self, id: StrId) -> Option<&InternedStr> {
        self.segments.get(id.index())
    }

    #[allow(dead_code)]
    pub(super) fn segments(&self) -> &[InternedStr] {
        &self.segments
    }

    pub(super) fn len(&self) -> usize {
        self.segments.len()
    }

    pub(super) fn install_snapshot(&mut self, snapshot: &[InternedStr]) {
        for entry in snapshot {
            let index = entry.id.index();
            if self.segments.len() > index {
                let existing = &self.segments[index];
                debug_assert_eq!(existing.value, entry.value);
                continue;
            }
            debug_assert_eq!(self.segments.len(), index);
            self.segments.push(entry.clone());
            self.map.insert(entry.value.clone(), entry.id);
        }
    }

    pub(super) fn drain(&mut self) -> Vec<InternedStr> {
        let drained = std::mem::take(&mut self.segments);
        self.map.clear();
        drained
    }
}
