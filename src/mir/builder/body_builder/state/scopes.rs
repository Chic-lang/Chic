use super::super::*;
use std::collections::HashMap;

body_builder_impl! {
    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(ScopeFrame::default());
    }

    pub(crate) fn pop_scope(&mut self) {
        if let Some(frame) = self.scopes.pop() {
            let fallback_span = self
                .blocks
                .get(self.current_block.0)
                .and_then(|block| block.span);
            let mut pending: Vec<(LocalId, Option<Span>)> = frame
                .locals
                .into_iter()
                .rev()
                .filter(|entry| entry.live)
                .map(|entry| (entry.local, entry.span.or(fallback_span)))
                .collect();
            if self.blocks[self.current_block.0].terminator.is_none() {
                for (local, span) in pending.drain(..) {
                    self.emit_storage_dead(local, span);
                }
            }
        }
    }

    pub(crate) fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    pub(crate) fn bind_name(&mut self, name: &str, id: LocalId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), id);
        }
    }

    pub(crate) fn bind_const(&mut self, name: &str, value: ConstValue) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.consts.insert(name.to_string(), value);
        }
    }

    pub(crate) fn register_match_binding(&mut self, local: LocalId) -> String {
        let name = format!("__match{}", self.match_binding_counter);
        self.match_binding_counter += 1;
        self.bind_name(&name, local);
        name
    }

    pub(crate) fn lookup_name(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.bindings.get(name) {
                return Some(*id);
            }
        }
        None
    }

    pub(crate) fn lookup_const(&self, name: &str) -> Option<ConstValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.consts.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    pub(crate) fn const_environment(&self) -> HashMap<String, ConstValue> {
        let mut map = HashMap::new();
        for scope in &self.scopes {
            for (name, value) in &scope.consts {
                map.insert(name.clone(), value.clone());
            }
        }
        map
    }
}
