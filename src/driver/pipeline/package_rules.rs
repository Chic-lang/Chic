use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::diagnostics::DiagnosticCode;
use crate::frontend::ast::Item;
use crate::frontend::diagnostics::{Diagnostic, Span, Suggestion};
use crate::manifest::{Manifest, SourceRoot, WorkspaceConfig};
use crate::package::resolver::ResolvedPackage;
use crate::unicode::identifier;

use super::{
    FrontendModuleState, PKG_FRIEND_DUPLICATE, PKG_FRIEND_INVALID_PREFIX, PKG_FRIEND_SELF_PREFIX,
    PKG_NAMESPACE_OUT_OF_SCOPE, PKG_PACKAGE_DIRECTIVES_DISALLOWED, PKG_PACKAGE_UNKNOWN,
    PKG_PACKAGE_UNRESOLVED, PKG_PACKAGE_VERSION_MISMATCH, package_error,
};

enum FriendSource {
    Manifest { manifest_path: Option<PathBuf> },
    Directive { span: Option<Span> },
}

impl FriendSource {
    fn span(&self) -> Option<Span> {
        match self {
            Self::Manifest { .. } => None,
            Self::Directive { span, .. } => *span,
        }
    }

    fn add_location_note(&self, diagnostic: &mut Diagnostic) {
        if let Self::Manifest { manifest_path } = self {
            if let Some(path) = manifest_path {
                diagnostic.add_note(format!("declared in manifest: {}", path.display()));
            }
        }
    }
}

pub(super) fn enforce_namespace_rules(
    manifest: &Manifest,
    workspace: Option<&WorkspaceConfig>,
    modules: &mut [FrontendModuleState],
) {
    let manifest_dir = manifest
        .path()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .or_else(|| workspace.map(|ws| ws.path.clone()));
    let package_prefix = manifest
        .package()
        .and_then(|pkg| pkg.namespace.clone().or_else(|| pkg.name.clone()));
    let source_roots = manifest.derived_source_roots();

    let mut allowed_prefixes = Vec::new();
    let mut seen_friends = HashSet::new();

    if let Some(prefix) = package_prefix.as_deref() {
        allowed_prefixes.push(prefix.to_string());
    }

    let mut manifest_diagnostics = Vec::new();
    if let Some(pkg) = manifest.package() {
        for prefix in &pkg.friends {
            if let Some(valid) = normalize_friend_prefix(
                prefix,
                package_prefix.as_deref(),
                FriendSource::Manifest {
                    manifest_path: manifest.path().map(PathBuf::from),
                },
                &mut manifest_diagnostics,
                &mut seen_friends,
            ) {
                allowed_prefixes.push(valid);
            }
        }
    }

    for module in modules.iter_mut() {
        if module.is_stdlib {
            continue;
        }
        for friend in &module.parse.module.friend_declarations {
            if let Some(valid) = normalize_friend_prefix(
                &friend.prefix,
                package_prefix.as_deref(),
                FriendSource::Directive { span: friend.span },
                &mut module.parse.diagnostics,
                &mut seen_friends,
            ) {
                allowed_prefixes.push(valid);
            }
        }
    }

    if let Some(index) = modules.iter().position(|module| !module.is_stdlib) {
        if let Some(target) = modules.get_mut(index) {
            target.parse.diagnostics.extend(manifest_diagnostics);
        }
    } else if let Some(first) = modules.first_mut() {
        first.parse.diagnostics.extend(manifest_diagnostics);
    }

    for module in modules.iter_mut() {
        if module.is_stdlib {
            continue;
        }

        let inferred_namespace = module.parse.module.namespace.is_none();
        if module.parse.module.namespace.is_none() {
            if let Some(inferred) = infer_namespace_for_module(
                module.input.as_path(),
                &source_roots,
                manifest_dir.as_deref(),
                package_prefix.as_deref(),
            ) {
                module.parse.module.namespace = Some(inferred);
                module
                    .parse
                    .module
                    .namespace_span
                    .get_or_insert_with(|| Span::in_file(module.parse.file_id, 0, 0));
            }
        }

        if let Some(ns) = module.parse.module.namespace.clone() {
            if !namespace_allowed(&ns, &allowed_prefixes) {
                report_out_of_scope_namespace(
                    &ns,
                    module.parse.module.namespace_span,
                    package_prefix.as_deref(),
                    manifest.path(),
                    &allowed_prefixes,
                    &mut module.parse.diagnostics,
                    inferred_namespace,
                );
            }
        }

        validate_namespace_items(
            &module.parse.module.items,
            &allowed_prefixes,
            module.parse.module.namespace_span,
            package_prefix.as_deref(),
            manifest.path(),
            &mut module.parse.diagnostics,
        );
    }
}

pub(super) fn attach_manifest_issues(manifest: &Manifest, modules: &mut [FrontendModuleState]) {
    if manifest.issues().is_empty() {
        return;
    }

    let mut diagnostics: Vec<_> = manifest
        .issues()
        .iter()
        .map(|issue| package_error(issue.code, issue.message.clone(), None))
        .collect();

    if let Some(index) = modules.iter().position(|module| !module.is_stdlib) {
        if let Some(target) = modules.get_mut(index) {
            target.parse.diagnostics.append(&mut diagnostics);
            return;
        }
    }
    if let Some(first) = modules.first_mut() {
        first.parse.diagnostics.append(&mut diagnostics);
    }
}

pub(super) fn append_external_diagnostics(
    modules: &mut [FrontendModuleState],
    mut diagnostics: Vec<Diagnostic>,
) {
    if diagnostics.is_empty() {
        return;
    }

    if let Some(index) = modules.iter().position(|module| !module.is_stdlib) {
        if let Some(target) = modules.get_mut(index) {
            target.parse.diagnostics.append(&mut diagnostics);
            return;
        }
    }

    if let Some(first) = modules.first_mut() {
        first.parse.diagnostics.append(&mut diagnostics);
    }
}

pub(super) fn validate_package_imports(manifest: &Manifest, modules: &mut [FrontendModuleState]) {
    let declared: HashSet<_> = manifest
        .dependencies()
        .iter()
        .map(|dep| dep.name.as_str())
        .collect();

    for module in modules.iter_mut() {
        if module.is_stdlib {
            continue;
        }

        if !module.parse.module.package_imports.is_empty() {
            for import in &module.parse.module.package_imports {
                let mut diag = package_error(
                    PKG_PACKAGE_DIRECTIVES_DISALLOWED,
                    "package imports must be declared in manifest.yaml; `@package` directives are disallowed",
                    import.span.or(module.parse.module.namespace_span),
                );
                if let Some(path) = manifest.path() {
                    diag.add_note(format!(
                        "declare dependencies under `dependencies` in {} instead of using `@package`",
                        path.display()
                    ));
                }
                module.parse.diagnostics.push(diag);
            }
        }

        for import in &module.parse.module.package_imports {
            if declared.contains(import.name.as_str()) {
                continue;
            }
            let mut diag = package_error(
                PKG_PACKAGE_UNKNOWN,
                format!(
                    "package `{}` is not listed in `manifest.yaml` dependencies",
                    import.name
                ),
                import.span.or(module.parse.module.namespace_span),
            );
            if let Some(path) = manifest.path() {
                diag.add_note(format!(
                    "declare the dependency under `dependencies` in {}",
                    path.display()
                ));
                diag.add_suggestion(Suggestion::new(
                    "add dependency to manifest",
                    None,
                    Some(format!("{}: \"<version>\"", import.name)),
                ));
            }
            module.parse.diagnostics.push(diag);
        }
    }
}

pub(super) fn attach_package_resolution_status(
    default_manifest: Option<&Manifest>,
    resolved: &HashMap<String, ResolvedPackage>,
    modules: &mut [FrontendModuleState],
) {
    let mut requirements_cache: HashMap<
        Option<PathBuf>,
        HashMap<String, crate::package::version::VersionReq>,
    > = HashMap::new();

    for module in modules.iter_mut() {
        if module.is_stdlib {
            continue;
        }

        let manifest = module.manifest.as_ref().or(default_manifest);
        let Some(manifest) = manifest else { continue };
        let manifest_key = manifest.path().map(PathBuf::from);
        let requirements = requirements_cache
            .entry(manifest_key.clone())
            .or_insert_with(|| {
                manifest
                    .dependencies()
                    .iter()
                    .filter_map(|dep| {
                        dep.requirement
                            .as_ref()
                            .map(|req| (dep.name.clone(), req.clone()))
                    })
                    .collect()
            });

        let mut seen = HashSet::new();
        for import in &module.parse.module.package_imports {
            if !seen.insert(import.name.clone()) {
                continue;
            }
            let Some(pkg) = resolved.get(&import.name) else {
                let mut diag = package_error(
                    PKG_PACKAGE_UNRESOLVED,
                    format!(
                        "package `{}` could not be resolved; ensure dependencies are restored",
                        import.name
                    ),
                    import.span.or(module.parse.module.namespace_span),
                );
                if let Some(path) = manifest.path() {
                    diag.add_note(format!("declared in manifest: {}", path.display()));
                }
                module.parse.diagnostics.push(diag);
                continue;
            };
            if let Some(req) = requirements.get(&import.name) {
                if !req.matches(&pkg.version) {
                    let mut diag = package_error(
                        PKG_PACKAGE_VERSION_MISMATCH,
                        format!(
                            "package `{}` resolved to version {}, which does not satisfy manifest constraint `{}`",
                            import.name, pkg.version, req
                        ),
                        import.span.or(module.parse.module.namespace_span),
                    );
                    if let Some(path) = manifest.path() {
                        diag.add_note(format!("declared in manifest: {}", path.display()));
                    }
                    module.parse.diagnostics.push(diag);
                }
            }
        }
    }
}

fn normalize_friend_prefix(
    prefix: &str,
    package_prefix: Option<&str>,
    source: FriendSource,
    diagnostics: &mut Vec<Diagnostic>,
    seen: &mut HashSet<String>,
) -> Option<String> {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        let mut diag = package_error(
            PKG_FRIEND_INVALID_PREFIX,
            "`@friend` prefix must not be empty",
            source.span(),
        );
        source.add_location_note(&mut diag);
        diagnostics.push(diag);
        return None;
    }

    if let Some(pkg_prefix) = package_prefix {
        if trimmed == pkg_prefix {
            let mut diag = package_error(
                PKG_FRIEND_SELF_PREFIX,
                "package namespace prefix does not need to be listed as a friend",
                source.span(),
            );
            source.add_location_note(&mut diag);
            diag.add_suggestion(Suggestion::new(
                "remove redundant @friend entry",
                source.span(),
                Some(String::new()),
            ));
            diagnostics.push(diag);
            return None;
        }
    }

    if !is_valid_namespace_prefix(trimmed) {
        let mut diag = package_error(
            PKG_FRIEND_INVALID_PREFIX,
            format!("`@friend` prefix `{trimmed}` is not a valid namespace prefix"),
            source.span(),
        );
        source.add_location_note(&mut diag);
        diagnostics.push(diag);
        return None;
    }

    let lowered = trimmed.to_string();
    if !seen.insert(lowered.clone()) {
        let mut diag = Diagnostic::warning(
            format!("duplicate `@friend` prefix `{trimmed}` is ignored"),
            source.span(),
        )
        .with_code(DiagnosticCode::new(
            PKG_FRIEND_DUPLICATE.to_string(),
            Some("package".into()),
        ));
        source.add_location_note(&mut diag);
        diagnostics.push(diag);
        return None;
    }

    Some(lowered)
}

fn namespace_allowed(namespace: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|prefix| {
        namespace == prefix
            || namespace.starts_with(prefix.as_str())
                && namespace
                    .as_bytes()
                    .get(prefix.len())
                    .is_some_and(|b| *b == b'.')
    })
}

fn validate_namespace_items(
    items: &[Item],
    allowed: &[String],
    fallback_span: Option<Span>,
    package_prefix: Option<&str>,
    manifest_path: Option<&Path>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        if let Item::Namespace(ns) = item {
            if !namespace_allowed(&ns.name, allowed) {
                report_out_of_scope_namespace(
                    &ns.name,
                    ns.span.or(fallback_span),
                    package_prefix,
                    manifest_path,
                    allowed,
                    diagnostics,
                    false,
                );
            }
            validate_namespace_items(
                &ns.items,
                allowed,
                ns.span.or(fallback_span),
                package_prefix,
                manifest_path,
                diagnostics,
            );
        }
    }
}

fn infer_namespace_for_module(
    path: &Path,
    roots: &[SourceRoot],
    manifest_dir: Option<&Path>,
    package_prefix: Option<&str>,
) -> Option<String> {
    let manifest_dir = manifest_dir?;
    let mut best_root: Option<(usize, PathBuf, &SourceRoot)> = None;

    for root in roots {
        let abs_root = manifest_dir.join(&root.path);
        if path.starts_with(&abs_root) {
            let depth = abs_root.components().count();
            if best_root
                .as_ref()
                .map_or(true, |(best_depth, _, _)| depth > *best_depth)
            {
                best_root = Some((depth, abs_root, root));
            }
        }
    }

    let (_, abs_root, root) = best_root?;
    let rel = path.strip_prefix(&abs_root).ok()?;

    let mut segments: Vec<String> = Vec::new();
    if let Some(prefix) = root.namespace_prefix.as_deref().or(package_prefix) {
        segments.extend(
            prefix
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_owned),
        );
    }

    let components: Vec<_> = rel.components().collect();
    for (index, component) in components.iter().enumerate() {
        let mut text = component.as_os_str().to_string_lossy().to_string();
        if index + 1 == components.len() {
            if let Some(stem) = Path::new(&text).file_stem() {
                text = stem.to_string_lossy().to_string();
            }
        }
        if text.is_empty() {
            continue;
        }
        let sanitized = sanitize_namespace_segment(text.as_ref());
        if sanitized.is_empty() {
            continue;
        }
        segments.push(sanitized);
    }

    if segments.is_empty() {
        return None;
    }

    Some(segments.join("."))
}

fn sanitize_namespace_segment(raw: &str) -> String {
    let mut output = String::new();
    let mut chars = raw.chars();
    if let Some(first) = chars.next() {
        if identifier::is_identifier_start(first) || first == '_' {
            output.extend(first.to_uppercase());
        } else {
            output.push('_');
        }
    }
    for ch in chars {
        if identifier::is_identifier_continue(ch) || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    output
}

fn is_valid_namespace_prefix(prefix: &str) -> bool {
    let mut seen_segment = false;
    for segment in prefix.split('.') {
        if segment.is_empty() {
            return false;
        }
        seen_segment = true;
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !identifier::is_identifier_start(first) && first != '_' {
            return false;
        }
        if chars.any(|ch| !identifier::is_identifier_continue(ch) && ch != '_') {
            return false;
        }
    }
    seen_segment
}

fn report_out_of_scope_namespace(
    namespace: &str,
    span: Option<Span>,
    package_prefix: Option<&str>,
    manifest_path: Option<&Path>,
    allowed_prefixes: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    inferred: bool,
) {
    let mut diag = package_error(
        PKG_NAMESPACE_OUT_OF_SCOPE,
        match package_prefix {
            Some(prefix) => format!(
                "namespace `{namespace}` is outside the package prefix `{prefix}` and is missing an `@friend` grant"
            ),
            None => format!(
                "namespace `{namespace}` is outside the allowed prefixes and is missing an `@friend` grant"
            ),
        },
        span,
    );
    diag.add_note(format!("actual namespace: `{namespace}`"));
    if let Some(prefix) = package_prefix {
        diag.add_note(format!("expected prefix: `{prefix}`"));
    }
    if let Some(path) = manifest_path {
        diag.add_note(format!("manifest: {}", path.display()));
    }
    if !allowed_prefixes.is_empty() {
        let mut prefixes: Vec<_> = allowed_prefixes
            .iter()
            .map(String::as_str)
            .map(|p| format!("`{p}`"))
            .collect();
        prefixes.sort();
        prefixes.dedup();
        diag.add_note(format!("allowed prefixes: {}", prefixes.join(", ")));
    }
    let friend_prefix = suggest_friend_prefix(namespace);
    diag.add_note(format!(
        "add `{friend_prefix}` to `package.friends` in the manifest (preferred) or add `@friend(\"{friend_prefix}\")` in the source file to authorise this namespace"
    ));
    if let (Some(prefix), Some(span)) = (package_prefix, span) {
        let replacement = if prefix.is_empty() {
            namespace.to_string()
        } else {
            format!("{prefix}.{namespace}")
        };
        diag.add_suggestion(Suggestion::new(
            "align with the package namespace prefix",
            Some(span),
            Some(replacement),
        ));
    }
    if inferred {
        diag.add_note("namespace was inferred from the file path; add an explicit `namespace` directive or friend grant to override inference");
    }
    diagnostics.push(diag);
}

fn suggest_friend_prefix(namespace: &str) -> String {
    let mut segments: Vec<_> = namespace.split('.').collect();
    if segments.len() > 2 {
        segments.pop();
        return segments.join(".");
    }
    namespace.to_string()
}
