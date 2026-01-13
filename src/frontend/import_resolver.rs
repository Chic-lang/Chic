use std::collections::{HashMap, HashSet};

use crate::frontend::ast::{ImportDirective, ImportKind, Item, Module};

const PRELUDE_TYPE_ALIASES: &[(&str, &[&str])] = &[
    ("Task", &["Std", "Async", "Task"]),
    ("Future", &["Std", "Async", "Future"]),
];

#[derive(Clone, Default, Debug)]
struct ImportScope {
    namespace_imports: Vec<Vec<String>>,
    alias_imports: HashMap<String, Vec<String>>,
    static_imports: Vec<Vec<String>>,
}

impl ImportScope {
    fn add_import(&mut self, import: &ImportDirective) {
        match &import.kind {
            ImportKind::Namespace { path } => {
                let segments = split_segments(path);
                if !segments.is_empty() {
                    self.namespace_imports.push(segments);
                }
            }
            ImportKind::Alias { alias, target } => {
                let segments = split_segments(target);
                if !segments.is_empty() {
                    self.alias_imports.insert(alias.clone(), segments);
                }
            }
            ImportKind::Static { target } => {
                let segments = split_segments(target);
                if !segments.is_empty() {
                    self.static_imports.push(segments);
                }
            }
            ImportKind::CImport { .. } => {}
        }
    }

    fn is_empty(&self) -> bool {
        self.namespace_imports.is_empty()
            && self.alias_imports.is_empty()
            && self.static_imports.is_empty()
    }
}

#[derive(Clone, Default, Debug)]
pub struct CombinedImportScope {
    pub namespace_imports: Vec<Vec<String>>,
    pub alias_imports: HashMap<String, Vec<String>>,
    pub static_imports: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
pub enum Resolution {
    Found(String),
    Ambiguous(Vec<String>),
    NotFound,
}

pub type ImportResolution = Resolution;
pub type UsingResolution = Resolution;

#[derive(Clone, Default, Debug)]
pub struct ImportResolver {
    scopes: HashMap<String, ImportScope>,
    global_scope: ImportScope,
}

impl ImportResolver {
    #[must_use]
    pub fn build(module: &Module) -> Self {
        let mut resolver = Self {
            scopes: HashMap::new(),
            global_scope: ImportScope::default(),
        };
        resolver.collect_scope(&module.items, module.namespace.as_deref(), false);
        resolver
    }

    fn collect_scope(&mut self, items: &[Item], namespace: Option<&str>, nested: bool) {
        let key = namespace_key(namespace);
        let mut scope = ImportScope::default();
        for item in items {
            match item {
                Item::Import(import) => {
                    if import.is_global {
                        if nested {
                            continue;
                        }
                        self.global_scope.add_import(import);
                        continue;
                    }
                    scope.add_import(import);
                }
                _ => break,
            }
        }
        if !scope.is_empty() {
            self.scopes.insert(key.clone(), scope);
        }

        for item in items {
            if let Item::Namespace(ns) = item {
                let nested = qualify(namespace, &ns.name);
                self.collect_scope(&ns.items, Some(&nested), true);
            }
        }
    }

    #[must_use]
    pub fn combined_scope(&self, namespace: Option<&str>) -> CombinedImportScope {
        let mut combined = CombinedImportScope::default();
        combined.namespace_imports.push(vec!["Std".to_string()]);
        Self::merge_scope(&mut combined, &self.global_scope);
        for key in namespace_chain(namespace) {
            if let Some(scope) = self.scopes.get(&key) {
                Self::merge_scope(&mut combined, scope);
            }
        }
        combined
    }

    fn merge_scope(target: &mut CombinedImportScope, scope: &ImportScope) {
        if scope.is_empty() {
            return;
        }
        target
            .namespace_imports
            .extend(scope.namespace_imports.clone());
        target.static_imports.extend(scope.static_imports.clone());
        for (alias, import_target) in &scope.alias_imports {
            target
                .alias_imports
                .insert(alias.clone(), import_target.clone());
        }
    }

    #[must_use]
    pub fn resolve_type<F>(
        &self,
        base_segments: &[String],
        namespace: Option<&str>,
        context_type: Option<&str>,
        mut is_available: F,
    ) -> Resolution
    where
        F: FnMut(&str) -> bool,
    {
        if base_segments.is_empty() {
            return Resolution::NotFound;
        }

        let combined = self.combined_scope(namespace);

        let mut alias_applied = false;
        let mut primary_segments =
            if let Some(alias_target) = combined.alias_imports.get(&base_segments[0]) {
                alias_applied = true;
                let mut segments = alias_target.clone();
                segments.extend_from_slice(&base_segments[1..]);
                segments
            } else {
                base_segments.to_vec()
            };

        if !alias_applied {
            if base_segments[0] == "object" {
                let mut segments = vec!["Std".to_string(), "Object".to_string()];
                segments.extend_from_slice(&base_segments[1..]);
                primary_segments = segments;
                alias_applied = true;
            }
        }

        if !alias_applied {
            if let Some((_, target)) = PRELUDE_TYPE_ALIASES
                .iter()
                .find(|(alias, _)| alias.eq_ignore_ascii_case(&base_segments[0]))
            {
                let mut segments: Vec<String> =
                    target.iter().map(|segment| segment.to_string()).collect();
                segments.extend_from_slice(&base_segments[1..]);
                primary_segments = segments;
                alias_applied = true;
            }
        }

        if alias_applied {
            return resolve_candidates(vec![primary_segments], &mut is_available)
                .unwrap_or(Resolution::NotFound);
        } else {
            if let Some(ctx) = context_type {
                let mut candidates = Vec::new();
                let mut ctx_segments = split_segments(ctx);
                while !ctx_segments.is_empty() {
                    let mut combined = ctx_segments.clone();
                    combined.extend(primary_segments.clone());
                    candidates.push(combined);
                    ctx_segments.pop();
                }
                if let Some(resolution) = resolve_first_candidate(candidates, &mut is_available) {
                    return resolution;
                }
            }

            if let Some(ns) = namespace {
                let mut candidates = Vec::new();
                let mut ns_segments = split_segments(ns);
                while !ns_segments.is_empty() {
                    let mut combined = ns_segments.clone();
                    combined.extend(primary_segments.clone());
                    candidates.push(combined);
                    ns_segments.pop();
                }
                if let Some(resolution) = resolve_first_candidate(candidates, &mut is_available) {
                    return resolution;
                }
            }

            let mut import_candidates = Vec::new();
            for import in &combined.namespace_imports {
                let mut combined = import.clone();
                combined.extend(primary_segments.clone());
                import_candidates.push(combined);
            }
            if let Some(resolution) = resolve_candidates(import_candidates, &mut is_available) {
                return resolution;
            }

            return resolve_candidates(vec![primary_segments], &mut is_available)
                .unwrap_or(Resolution::NotFound);
        }
    }
}

fn resolve_candidates<F>(candidates: Vec<Vec<String>>, is_available: &mut F) -> Option<Resolution>
where
    F: FnMut(&str) -> bool,
{
    let mut checked = HashSet::new();
    let mut matches = Vec::new();
    for segments in candidates {
        if segments.is_empty() {
            continue;
        }
        let candidate = canonicalize_segments(&segments);
        if !checked.insert(candidate.clone()) {
            continue;
        }
        if is_available(&candidate) {
            matches.push(candidate);
        }
    }
    if matches.is_empty() {
        None
    } else if matches.len() == 1 {
        Some(Resolution::Found(matches.remove(0)))
    } else {
        Some(Resolution::Ambiguous(matches))
    }
}

fn resolve_first_candidate<F>(
    candidates: Vec<Vec<String>>,
    is_available: &mut F,
) -> Option<Resolution>
where
    F: FnMut(&str) -> bool,
{
    let mut checked = HashSet::new();
    for segments in candidates {
        if segments.is_empty() {
            continue;
        }
        let candidate = canonicalize_segments(&segments);
        if !checked.insert(candidate.clone()) {
            continue;
        }
        if is_available(&candidate) {
            return Some(Resolution::Found(candidate));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{ImportDirective, ImportKind, Item, NamespaceDecl};

    fn make_module(with_alias: bool) -> Module {
        let alias_using = if with_alias {
            vec![Item::Import(ImportDirective {
                doc: None,
                is_global: false,
                span: None,
                kind: ImportKind::Alias {
                    alias: "Alias".to_string(),
                    target: "Alpha.Beta".to_string(),
                },
            })]
        } else {
            Vec::new()
        };

        let mut items = Vec::new();
        items.push(Item::Import(ImportDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: ImportKind::Namespace {
                path: "Alpha.Beta".to_string(),
            },
        }));
        items.push(Item::Import(ImportDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: ImportKind::Namespace {
                path: "Gamma".to_string(),
            },
        }));
        items.extend(alias_using);
        items.push(Item::Namespace(NamespaceDecl {
            name: "Utilities".to_string(),
            items: Vec::new(),
            doc: None,
            attributes: Vec::new(),
            span: None,
        }));

        Module::with_items(None, items)
    }

    #[test]
    fn std_namespace_is_implicit() {
        let module = Module::with_items(None, Vec::new());
        let resolver = ImportResolver::build(&module);
        let available = |name: &str| name == "Std::Option";
        let resolution = resolver.resolve_type(&["Option".into()], None, None, available);
        match resolution {
            Resolution::Found(name) => assert_eq!(name, "Std::Option"),
            other => panic!("expected implicit Std import, found {other:?}"),
        }
    }

    #[test]
    fn unresolved_names_without_imports_do_not_clash() {
        let module = Module::with_items(None, Vec::new());
        let resolver = ImportResolver::build(&module);
        let available = |name: &str| name == "Utility::Widget";
        let resolution = resolver.resolve_type(&["Widget".into()], None, None, available);
        assert!(matches!(resolution, Resolution::NotFound));
    }

    #[test]
    fn resolves_namespace_import() {
        let module = make_module(false);
        let resolver = ImportResolver::build(&module);
        let available = |name: &str| name == "Alpha::Beta::Widget";
        let resolution = resolver.resolve_type(&["Widget".into()], None, None, available);
        match resolution {
            Resolution::Found(name) => assert_eq!(name, "Alpha::Beta::Widget"),
            other => panic!("unexpected resolution {other:?}"),
        }
    }

    #[test]
    fn resolves_alias_import() {
        let module = make_module(true);
        let resolver = ImportResolver::build(&module);
        let available = |name: &str| name == "Alpha::Beta::Widget";
        let resolution =
            resolver.resolve_type(&["Alias".into(), "Widget".into()], None, None, available);
        match resolution {
            Resolution::Found(name) => assert_eq!(name, "Alpha::Beta::Widget"),
            other => panic!("unexpected resolution {other:?}"),
        }
    }

    #[test]
    fn reports_ambiguous_resolution() {
        let module = make_module(false);
        let resolver = ImportResolver::build(&module);
        let available = |name: &str| name == "Alpha::Beta::Widget" || name == "Gamma::Widget";
        let resolution = resolver.resolve_type(&["Widget".into()], None, None, available);
        match resolution {
            Resolution::Ambiguous(mut candidates) => {
                candidates.sort();
                assert_eq!(candidates, vec!["Alpha::Beta::Widget", "Gamma::Widget"]);
            }
            other => panic!("unexpected resolution {other:?}"),
        }
    }

    #[test]
    fn global_import_inside_namespace_is_ignored() {
        let module = Module::with_items(
            None,
            vec![
                Item::Namespace(NamespaceDecl {
                    name: "Declaring".to_string(),
                    items: vec![Item::Import(ImportDirective {
                        doc: None,
                        is_global: true,
                        span: None,
                        kind: ImportKind::Namespace {
                            path: "Shared.Types".to_string(),
                        },
                    })],
                    doc: None,
                    attributes: Vec::new(),
                    span: None,
                }),
                Item::Namespace(NamespaceDecl {
                    name: "Consumer".to_string(),
                    items: Vec::new(),
                    doc: None,
                    attributes: Vec::new(),
                    span: None,
                }),
            ],
        );
        let resolver = ImportResolver::build(&module);
        let available = |name: &str| name == "Shared::Types::Widget";
        let resolution =
            resolver.resolve_type(&["Widget".into()], Some("Consumer"), None, available);
        assert!(
            matches!(resolution, Resolution::NotFound),
            "nested global import should be ignored, got {resolution:?}"
        );
    }

    #[test]
    fn conflicting_local_alias_prefers_local_resolution() {
        let module = Module::with_items(
            None,
            vec![
                Item::Import(ImportDirective {
                    doc: None,
                    is_global: true,
                    span: None,
                    kind: ImportKind::Alias {
                        alias: "Services".to_string(),
                        target: "Shared.Services".to_string(),
                    },
                }),
                Item::Namespace(NamespaceDecl {
                    name: "Consumer".to_string(),
                    items: vec![Item::Import(ImportDirective {
                        doc: None,
                        is_global: false,
                        span: None,
                        kind: ImportKind::Alias {
                            alias: "Services".to_string(),
                            target: "Local.Services".to_string(),
                        },
                    })],
                    doc: None,
                    attributes: Vec::new(),
                    span: None,
                }),
            ],
        );
        let resolver = ImportResolver::build(&module);
        let available =
            |name: &str| name == "Local::Services::Logger" || name == "Shared::Services::Logger";
        let resolution = resolver.resolve_type(
            &["Services".into(), "Logger".into()],
            Some("Consumer"),
            None,
            available,
        );
        match resolution {
            Resolution::Found(name) => assert_eq!(name, "Local::Services::Logger"),
            other => panic!("expected local alias to remain preferred, found {other:?}"),
        }
    }
}

fn split_segments(name: &str) -> Vec<String> {
    name.replace("::", ".")
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn canonicalize_segments(segments: &[String]) -> String {
    segments.join("::")
}

fn namespace_key(namespace: Option<&str>) -> String {
    namespace
        .map(|ns| ns.replace("::", "."))
        .unwrap_or_else(String::new)
}

fn namespace_chain(namespace: Option<&str>) -> Vec<String> {
    let mut chain = Vec::new();
    chain.push(String::new());
    if let Some(ns) = namespace {
        let segments = split_segments(ns);
        let mut prefix = Vec::new();
        for segment in segments {
            prefix.push(segment);
            chain.push(prefix.join("."));
        }
    }
    chain
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}
