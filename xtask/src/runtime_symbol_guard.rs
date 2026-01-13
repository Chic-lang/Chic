use regex::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const BASELINE: &str = "docs/runtime/rust_runtime_symbols.txt";
const STRICT_ENV: &str = "CHIC_RUNTIME_STRICT";
const GLOBAL_STRICT_ENV: &str = "CHIC_NATIVE_ONLY_STRICT";

pub fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf();

    let strict = truthy(STRICT_ENV) || truthy(GLOBAL_STRICT_ENV);
    let baseline = if strict {
        HashSet::new()
    } else {
        load_baseline(&workspace_root.join(BASELINE))?
    };

    let runtime_root = workspace_root.join("src").join("runtime");
    check_native_only_runtime(&runtime_root)?;

    let current = collect_symbols(&runtime_root)?;
    let adapter_root = workspace_root.join("runtime_adapter");
    if adapter_root.exists() {
        return Err(
            "runtime_adapter folder must be removed; implement runtime semantics in Chic".into(),
        );
    }

    let mut additions: Vec<String> = if strict {
        current.iter().cloned().collect()
    } else {
        current.difference(&baseline).cloned().collect()
    };
    additions.sort();

    if !additions.is_empty() {
        eprintln!(
            "lint-runtime-symbols: new Rust runtime symbols detected (src/runtime or runtime_adapter):"
        );
        for entry in additions {
            eprintln!("  - {entry}");
        }
        eprintln!(
            "If a symbol is truly required, migrate the semantics to Std.* and the native runtime; \
             do not grow the Rust runtime surface."
        );
        return Err("lint-runtime-symbols failed".into());
    }

    if strict {
        println!(
            "lint-runtime-symbols: strict mode enabled; no chic_rt_* symbols remain in Rust runtime sources."
        );
    } else {
        println!(
            "lint-runtime-symbols: no new chic_rt_* symbols in Rust runtime sources (src/runtime + runtime_adapter)."
        );
    }
    Ok(())
}

fn load_baseline(path: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
    let data = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read runtime symbol baseline {}: {err}",
            path.display()
        )
    })?;
    let set = data
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect();
    Ok(set)
}

fn collect_symbols(root: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut set = HashSet::new();
    let re = Regex::new(r"chic_rt_[A-Za-z0-9][A-Za-z0-9_]*")?;
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let rel = entry
            .path()
            .strip_prefix(
                root.parent()
                    .expect("runtime folder has a parent")
                    .parent()
                    .expect("workspace root"),
            )
            .unwrap_or(entry.path());
        for cap in re.captures_iter(&contents) {
            let sym = cap.get(0).unwrap().as_str();
            set.insert(format!("{}:{}", rel.display(), sym));
        }
    }
    Ok(set)
}

fn check_native_only_runtime(root: &Path) -> Result<(), Box<dyn Error>> {
    // Reject any paths that attempt to compile when `chic_native_runtime` is *not* enabled.
    // Positive `cfg!(chic_native_runtime)` assertions are allowed.
    let disallowed = Regex::new(
        r"cfg!\s*\(\s*not\s*\(\s*chic_native_runtime\s*\)\s*\)|cfg\s*\(\s*not\s*\(\s*chic_native_runtime",
    )?;
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        if let Some(mat) = disallowed.find(&contents) {
            let rel = entry
                .path()
                .strip_prefix(root.parent().and_then(|p| p.parent()).unwrap_or(root))
                .unwrap_or(entry.path());
            eprintln!(
                "lint-runtime-symbols: native runtime is required; found disallowed cfg in {} at byte {}",
                rel.display(),
                mat.start()
            );
            return Err(
                "lint-runtime-symbols failed: chic_native_runtime fallbacks are forbidden".into(),
            );
        }
    }
    Ok(())
}

fn truthy(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}
