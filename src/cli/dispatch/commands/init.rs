use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::CliError;
use crate::cli::templates::{available_templates, render, resolve_template};
use crate::error::{Error, Result};

pub(super) fn run_init(
    template: &str,
    output: Option<PathBuf>,
    name: Option<String>,
) -> Result<()> {
    let Some(kind) = resolve_template(template) else {
        return Err(Error::Cli(CliError::new(unknown_template_message(
            template,
        ))));
    };
    let target_dir = output.unwrap_or_else(|| PathBuf::from("."));
    if target_dir.exists() && !target_dir.is_dir() {
        return Err(Error::Cli(CliError::new(format!(
            "destination {} is not a directory",
            target_dir.display()
        ))));
    }

    let project_name = resolve_project_name(&target_dir, name, kind.default_project_name());
    let rendered = render(kind, &project_name);

    let conflicts = detect_conflicts(&target_dir, &rendered)?;
    if !conflicts.is_empty() {
        return Err(Error::Cli(CliError::new(conflict_message(
            &target_dir,
            &conflicts,
        ))));
    }

    fs::create_dir_all(&target_dir)?;
    for asset in &rendered {
        let path = target_dir.join(asset.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &asset.contents)?;
        println!("  create {}", display_relative(&target_dir, &path));
    }

    println!(
        "Created {} project `{}` in {}",
        kind.display_name(),
        project_name,
        target_dir.display()
    );
    Ok(())
}

fn resolve_project_name(target: &Path, provided: Option<String>, fallback: &str) -> String {
    if let Some(raw) = provided {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(name) = target.file_name().and_then(|value| value.to_str()) {
        let trimmed = name.trim();
        if !trimmed.is_empty() && trimmed != "." {
            return trimmed.to_string();
        }
    }
    fallback.to_string()
}

fn detect_conflicts(
    base: &Path,
    assets: &[crate::cli::templates::RenderedAsset],
) -> Result<Vec<PathBuf>> {
    let mut conflicts = Vec::new();
    for asset in assets {
        let path = base.join(asset.relative_path);
        if path.exists() {
            conflicts.push(path);
        }
    }
    Ok(conflicts)
}

fn conflict_message(base: &Path, conflicts: &[PathBuf]) -> String {
    let joined = conflicts
        .iter()
        .map(|path| display_relative(base, path))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "refusing to overwrite existing files in {}: {}",
        base.display(),
        joined
    )
}

fn display_relative(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|rel| rel.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn unknown_template_message(requested: &str) -> String {
    let available = available_templates();
    format!(
        "unknown template '{requested}'; available templates: {}",
        available.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_unknown_template() {
        let err = run_init("unknown", None, None).expect_err("expected failure");
        match err {
            Error::Cli(cli) => assert!(cli.to_string().contains("available templates")),
            other => panic!("expected CLI error, found {other:?}"),
        }
    }

    #[test]
    fn refuses_to_overwrite_existing_files() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("app");
        fs::create_dir_all(&target).expect("mkdir");
        let manifest = target.join("manifest.yaml");
        fs::write(&manifest, "existing").expect("write manifest");
        let err = run_init("app", Some(target.clone()), None).expect_err("expected failure");
        match err {
            Error::Cli(cli) => {
                let message = cli.to_string();
                assert!(message.contains("manifest.yaml"), "message: {message}");
            }
            other => panic!("expected CLI error, found {other:?}"),
        }
    }

    #[test]
    fn derives_project_name_from_directory() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("demo-app");
        let project = resolve_project_name(&target, None, "fallback");
        assert_eq!(project, "demo-app");
    }
}
