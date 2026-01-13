use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf();
    let mut failures = Vec::new();
    let mut stack = vec![workspace_root.clone()];

    while let Some(path) = stack.pop() {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            if is_ignored_dir(&path) {
                continue;
            }
            for entry in fs::read_dir(&path)? {
                stack.push(entry?.path());
            }
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "manifest.yaml") {
            if let Some(message) = violation(&path, &workspace_root) {
                failures.push(message);
            }
        }
    }

    if failures.is_empty() {
        println!("lint-manifest-location: all manifests are at package roots.");
        Ok(())
    } else {
        eprintln!("lint-manifest-location: invalid manifest locations detected:");
        for failure in failures {
            eprintln!("  - {failure}");
        }
        Err("lint-manifest-location failed".into())
    }
}

fn is_ignored_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".git" | "target" | "obj" | "bin" | ".cargo" | "coverage"
    )
}

fn violation(path: &Path, workspace_root: &Path) -> Option<String> {
    let rel = path.strip_prefix(workspace_root).unwrap_or(path);
    let components: Vec<_> = rel.components().collect();
    let mut packages_index = None;
    for (idx, component) in components.iter().enumerate() {
        if component.as_os_str() == "packages" {
            packages_index = Some(idx);
            break;
        }
    }
    if let Some(idx) = packages_index {
        if let Some(pkg) = components.get(idx + 1) {
            let expected = workspace_root
                .join("packages")
                .join(pkg.as_os_str())
                .join("manifest.yaml");
            if path != expected {
                return Some(format!(
                    "{} should live at {}",
                    path.display(),
                    expected.display()
                ));
            }
        }
    }
    if components
        .iter()
        .any(|component| component.as_os_str() == "src")
    {
        return Some(format!(
            "{} is nested under src; move manifest.yaml to packages/<name>/manifest.yaml",
            path.display()
        ));
    }
    None
}
