//! Arena for sharing AST modules between frontend stages.

use std::cell::{Ref, RefCell, RefMut};

use super::items::{Attribute, CrateAttributes, Item, Module};
use crate::frontend::ast::{FriendDirective, PackageImport};
use crate::frontend::diagnostics::Span;

/// Identifier for a module stored in the [`AstArena`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(usize);

/// Shared storage for parsed AST modules.
#[derive(Default, Debug)]
pub struct AstArena {
    modules: ArenaStore<Module>,
}

impl AstArena {
    #[must_use]
    pub fn new() -> Self {
        Self {
            modules: ArenaStore::default(),
        }
    }

    #[must_use]
    pub fn module_builder(&self, namespace: Option<String>) -> ModuleBuilder<'_> {
        ModuleBuilder::new(self, namespace)
    }

    #[must_use]
    pub fn module(&self, id: ModuleId) -> Ref<'_, Module> {
        self.modules.get(id.0)
    }

    #[must_use]
    pub fn module_mut(&self, id: ModuleId) -> RefMut<'_, Module> {
        self.modules.get_mut(id.0)
    }

    #[must_use]
    pub fn module_owned(&self, id: ModuleId) -> Module {
        self.module(id).clone()
    }

    fn alloc_module(&self, module: Module) -> ModuleId {
        ModuleId(self.modules.alloc(module))
    }
}

/// Builder used to prepare a module before storing it in the arena.
pub struct ModuleBuilder<'arena> {
    arena: &'arena AstArena,
    namespace: Option<String>,
    namespace_span: Option<Span>,
    namespace_attributes: Vec<Attribute>,
    crate_attributes: CrateAttributes,
    friend_declarations: Vec<FriendDirective>,
    package_imports: Vec<PackageImport>,
    items: Vec<Item>,
}

impl<'arena> ModuleBuilder<'arena> {
    fn new(arena: &'arena AstArena, namespace: Option<String>) -> Self {
        Self {
            arena,
            namespace,
            namespace_span: None,
            namespace_attributes: Vec::new(),
            crate_attributes: CrateAttributes::default(),
            friend_declarations: Vec::new(),
            package_imports: Vec::new(),
            items: Vec::new(),
        }
    }

    pub fn with_namespace_span(mut self, span: Option<Span>) -> Self {
        self.namespace_span = span;
        self
    }

    pub fn with_namespace_attributes(mut self, attrs: Vec<Attribute>) -> Self {
        self.namespace_attributes = attrs;
        self
    }

    pub fn with_crate_attributes(mut self, attrs: CrateAttributes) -> Self {
        self.crate_attributes = attrs;
        self
    }

    pub fn push_item(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn with_items(mut self, items: Vec<Item>) -> Self {
        self.items = items;
        self
    }

    pub fn with_friend_declarations(mut self, friends: Vec<FriendDirective>) -> Self {
        self.friend_declarations = friends;
        self
    }

    pub fn with_package_imports(mut self, imports: Vec<PackageImport>) -> Self {
        self.package_imports = imports;
        self
    }

    #[must_use]
    pub fn finish_owned(self) -> Module {
        let ModuleBuilder {
            arena: _,
            namespace,
            namespace_span,
            namespace_attributes,
            crate_attributes,
            friend_declarations,
            package_imports,
            items,
        } = self;
        let mut module = Module::with_namespace_items(
            namespace,
            namespace_span,
            namespace_attributes,
            friend_declarations,
            items,
        );
        module.package_imports = package_imports;
        module.crate_attributes = crate_attributes;
        module
    }

    #[must_use]
    pub fn finish_in(self) -> ModuleId {
        let ModuleBuilder {
            arena,
            namespace,
            namespace_span,
            namespace_attributes,
            crate_attributes,
            friend_declarations,
            package_imports,
            items,
        } = self;
        let mut module = Module::with_namespace_items(
            namespace,
            namespace_span,
            namespace_attributes,
            friend_declarations,
            items,
        );
        module.package_imports = package_imports;
        module.crate_attributes = crate_attributes;
        arena.alloc_module(module)
    }
}

#[derive(Debug)]
struct ArenaStore<T> {
    entries: RefCell<Vec<T>>,
}

impl<T> Default for ArenaStore<T> {
    fn default() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
        }
    }
}

impl<T> ArenaStore<T> {
    fn alloc(&self, value: T) -> usize {
        let mut entries = self.entries.borrow_mut();
        let index = entries.len();
        entries.push(value);
        index
    }

    fn get(&self, index: usize) -> Ref<'_, T> {
        Ref::map(self.entries.borrow(), |entries| &entries[index])
    }

    fn get_mut(&self, index: usize) -> RefMut<'_, T> {
        RefMut::map(self.entries.borrow_mut(), |entries| &mut entries[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::items::{UsingDirective, UsingKind};

    #[test]
    fn stores_module_in_arena() {
        let arena = AstArena::new();
        let mut builder = arena.module_builder(Some("Core".into()));
        builder.push_item(Item::Import(UsingDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: UsingKind::Namespace {
                path: "Core.Utils".into(),
            },
        }));
        let id = builder.finish_in();
        let module = arena.module(id);
        assert_eq!(module.namespace.as_deref(), Some("Core"));
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn finish_owned_matches_builder() {
        let arena = AstArena::new();
        let builder = arena.module_builder(None);
        let module = builder.finish_owned();
        assert!(module.namespace.is_none());
        assert!(module.items.is_empty());
    }
}
