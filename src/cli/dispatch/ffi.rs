use std::fs;

use glob::glob;

use crate::Target;
use crate::cli::CliFfiOptions;
use crate::driver::BuildFfiOptions;
use crate::error::{Error, Result};

pub(super) fn resolve_cli_ffi_options(
    cli: &CliFfiOptions,
    target: &Target,
) -> Result<BuildFfiOptions> {
    let mut search_paths = cli.search_paths.clone();
    let platform = canonical_target_key(target);
    let mut default_pattern = cli
        .default_patterns
        .iter()
        .find(|entry| entry.target == platform)
        .map(|entry| entry.pattern.clone());
    if default_pattern.is_none() {
        if let Some(entry) = cli
            .default_patterns
            .iter()
            .find(|entry| entry.target == "any")
        {
            default_pattern = Some(entry.pattern.clone());
        }
    }

    let mut packages = Vec::new();
    for pattern in &cli.package_globs {
        let mut matched = false;
        let walker = glob(pattern).map_err(|err| {
            Error::internal(format!("invalid ffi package glob '{pattern}': {err}"))
        })?;
        for entry in walker {
            let path = entry.map_err(|err| {
                Error::internal(format!(
                    "failed to evaluate ffi package glob '{pattern}': {err}"
                ))
            })?;
            if path.is_dir() {
                return Err(Error::internal(format!(
                    "ffi package glob '{pattern}' matched directory {}; only files are supported",
                    path.display()
                )));
            }
            matched = true;
            packages.push(path);
        }
        if !matched {
            return Err(Error::internal(format!(
                "ffi package glob '{pattern}' did not match any files"
            )));
        }
    }
    for path in &mut search_paths {
        if let Ok(absolute) = fs::canonicalize(&*path) {
            *path = absolute;
        }
    }

    Ok(BuildFfiOptions {
        search_paths,
        default_pattern,
        packages,
    })
}

pub(super) fn canonical_target_key(target: &Target) -> String {
    let triple = target.triple().to_ascii_lowercase();
    if triple.contains("windows") {
        "windows".into()
    } else if triple.contains("darwin") || triple.contains("apple") || triple.contains("macos") {
        "macos".into()
    } else if triple.contains("linux") {
        "linux".into()
    } else if triple.contains("wasi") {
        "wasi".into()
    } else {
        "other".into()
    }
}
