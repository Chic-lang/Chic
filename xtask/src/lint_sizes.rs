use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const MAX_DEFAULT_LINES: usize = 1_000;

const ALLOWLIST: &[(&str, usize)] = &[];

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = repo_root()?;
    let allow_map: HashMap<&str, usize> = ALLOWLIST.iter().copied().collect();
    let mut failures = Vec::new();

    for entry in WalkDir::new(&repo_root)
        .into_iter()
        .filter_entry(|e| keep_entry(e.path()))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("warning: skipping entry due to error: {err}");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let rel_path = relative_path(&repo_root, path)?;
        let line_count = count_lines(path)?;

        if let Some(limit) = allow_map.get(rel_path.as_str()) {
            if line_count > *limit {
                failures.push((rel_path, line_count, *limit));
            }
            continue;
        }

        if line_count > MAX_DEFAULT_LINES {
            failures.push((rel_path, line_count, MAX_DEFAULT_LINES));
        }
    }

    if failures.is_empty() {
        println!(
            "lint-sizes: all files are within the configured limits ({} LOC default).",
            MAX_DEFAULT_LINES
        );
        return Ok(());
    }

    failures.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("lint-sizes: found files exceeding their permitted line counts:");
    for (path, lines, limit) in failures {
        eprintln!("  {path}: {lines} lines (limit {limit})");
    }
    eprintln!();
    eprintln!("Next steps when a file exceeds the limit:");
    eprintln!("  1. Split the module into focused submodules where practical.");
    eprintln!(
        "  2. Track the refactor plan in a linked issue/PR (or `todos.md` for small items) and keep the decomposition deletion-oriented."
    );
    eprintln!(
        "  3. If a temporary exception is absolutely necessary, adjust `ALLOWLIST` in `xtask/src/lint_sizes.rs` and document why."
    );
    eprintln!("     (Remember to remove the allowlist entry once the module is decomposed.)");
    eprintln!();
    eprintln!(
        "The default limit is {} LOC; adjust with care to keep the codebase maintainable.",
        MAX_DEFAULT_LINES
    );
    Err("lint-sizes failed".into())
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(manifest_dir
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf())
}

fn keep_entry(path: &Path) -> bool {
    path.components().all(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| !matches!(name, ".git" | "target" | "coverage"))
            .unwrap_or(true)
    })
}

fn relative_path(root: &Path, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let rel = path.strip_prefix(root)?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

fn count_lines(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(contents.lines().count())
}
