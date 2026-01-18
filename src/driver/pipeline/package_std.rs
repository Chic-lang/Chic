use crate::manifest::Manifest;

use super::{FrontendModuleState, PKG_PACKAGE_STD_MISSING, package_error};
use crate::frontend::diagnostics::Suggestion;

pub(super) fn is_std_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "std" || lower.starts_with("std.")
}

pub(super) fn declares_std_dependency(manifest: &Manifest) -> bool {
    manifest
        .dependencies()
        .iter()
        .any(|dep| is_std_name(&dep.name))
}

pub(super) fn enforce_std_dependency(
    manifest: &Manifest,
    load_stdlib: bool,
    modules: &mut [FrontendModuleState],
) {
    if !load_stdlib {
        return;
    }

    let is_std_package = manifest
        .package()
        .and_then(|pkg| pkg.name.as_deref())
        .map(is_std_name)
        .unwrap_or(false);
    if is_std_package {
        return;
    }

    let declares_std = declares_std_dependency(manifest);
    if declares_std {
        return;
    }

    if let Some(module) = modules.iter_mut().find(|m| !m.is_stdlib) {
        let mut diag = package_error(
            PKG_PACKAGE_STD_MISSING,
            "standard library must be declared under `dependencies` in manifest.yaml",
            None,
        );
        if let Some(path) = manifest.path() {
            diag.add_note(format!(
                "add `std` dependency in {} to enable stdlib for this package",
                path.display()
            ));
            diag.add_suggestion(Suggestion::new(
                "add std dependency",
                None,
                Some("std:\n  path: ../std".to_string()),
            ));
        }
        module.parse.diagnostics.push(diag);
    }
}
