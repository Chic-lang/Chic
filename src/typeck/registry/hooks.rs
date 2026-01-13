use crate::frontend::diagnostics::Span;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegisteredItemKind {
    Struct,
    Class,
    Enum,
    Union,
    Interface,
    Trait,
    Delegate,
}

impl fmt::Display for RegisteredItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Struct => "struct",
            Self::Class => "class",
            Self::Enum => "enum",
            Self::Union => "union",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Delegate => "delegate",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegistryEventKind {
    Registered,
    Conflict,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(super) struct RegistryEvent {
    pub kind: RegistryEventKind,
    pub name: String,
    pub item_kind: RegisteredItemKind,
    pub span: Option<Span>,
    pub previous_span: Option<Span>,
}

#[derive(Default)]
pub(crate) struct RegistryHooks {
    callbacks: Vec<Box<dyn FnMut(&RegistryEvent)>>,
}

impl RegistryHooks {
    #[cfg(test)]
    pub(super) fn subscribe<F>(&mut self, hook: F)
    where
        F: FnMut(&RegistryEvent) + 'static,
    {
        self.callbacks.push(Box::new(hook));
    }

    pub(super) fn emit(&mut self, event: RegistryEvent) {
        if self.callbacks.is_empty() {
            return;
        }
        for callback in &mut self.callbacks {
            callback(&event);
        }
    }
}

#[derive(Default)]
pub(crate) struct RegistryIndex {
    seen: HashMap<String, Vec<RegistryEntry>>,
}

impl RegistryIndex {
    pub(super) fn record(
        &mut self,
        name: &str,
        kind: RegisteredItemKind,
        arity: usize,
        span: Option<Span>,
        hooks: &mut RegistryHooks,
    ) -> Result<(), RegistryConflict> {
        if let Some(entries) = self.seen.get(name) {
            if let Some(conflict) = entries
                .iter()
                .find(|entry| entry.kind == kind && entry.arity == arity)
            {
                let event = RegistryEvent {
                    kind: RegistryEventKind::Conflict,
                    name: name.to_string(),
                    item_kind: conflict.kind,
                    span,
                    previous_span: conflict.span,
                };
                hooks.emit(event);
                return Err(RegistryConflict {
                    kind: conflict.kind,
                    first_span: conflict.span,
                });
            }
        }

        self.seen
            .entry(name.to_string())
            .or_default()
            .push(RegistryEntry { kind, span, arity });
        let event = RegistryEvent {
            kind: RegistryEventKind::Registered,
            name: name.to_string(),
            item_kind: kind,
            span,
            previous_span: None,
        };
        hooks.emit(event);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) struct RegistryConflict {
    pub kind: RegisteredItemKind,
    pub first_span: Option<Span>,
}

#[derive(Debug, Clone, Copy)]
struct RegistryEntry {
    kind: RegisteredItemKind,
    span: Option<Span>,
    arity: usize,
}
