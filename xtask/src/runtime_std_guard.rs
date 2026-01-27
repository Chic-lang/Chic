use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const BASELINE: &str = "docs/runtime/rust_runtime_std_symbols.txt";
const SEARCH_ROOTS: &[&str] = &[
    "packages/std/src",
    "packages/std.core/src",
    "packages/std.alloc/src",
    "packages/std.foundation/src",
    "packages/std.net/src",
    "packages/std.security/src",
    "packages/std.text/src",
    "packages/runtime.native/src",
    "packages/runtime.no_std/src",
];
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
    let (current, usage) = collect_symbols(&workspace_root)?;

    let mut additions: Vec<String> = if strict {
        current.iter().cloned().collect()
    } else {
        current.difference(&baseline).cloned().collect()
    };
    additions.sort();

    if !additions.is_empty() {
        eprintln!(
            "lint-runtime-stdlib: new chic_rt_* references detected in Std/Core/native runtime sources:"
        );
        for sym in additions {
            eprintln!("  - {sym}");
            if let Some(paths) = usage.get(&sym) {
                let mut paths = paths.clone();
                paths.sort();
                paths.dedup();
                for path in paths.iter().take(5) {
                    eprintln!("      used in {path}");
                }
                if paths.len() > 5 {
                    eprintln!("      (+{} more)", paths.len() - 5);
                }
            }
        }
        eprintln!(
            "Route language-visible behaviour through Std.* and the Chic-native runtime; \
             update the spec and baseline if a new runtime ABI symbol is truly required."
        );
        return Err("lint-runtime-stdlib failed".into());
    }

    if strict {
        println!(
            "lint-runtime-stdlib: strict mode enabled; no chic_rt_* references remain in Std/Core/native runtime sources."
        );
    } else {
        println!(
            "lint-runtime-stdlib: runtime symbol references in Std/Core/native runtime are unchanged ({} symbols).",
            current.len()
        );
    }
    Ok(())
}

fn load_baseline(path: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
    let data = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read runtime stdlib baseline {}: {err}",
            path.display()
        )
    })?;
    let set = data
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect();
    Ok(set)
}

fn collect_symbols(
    workspace_root: &Path,
) -> Result<(HashSet<String>, HashMap<String, Vec<String>>), Box<dyn Error>> {
    let mut set = HashSet::new();
    let mut usage: HashMap<String, Vec<String>> = HashMap::new();
    let re = Regex::new(r"chic_rt_[A-Za-z0-9][A-Za-z0-9_]*")?;

    for root in SEARCH_ROOTS {
        let root_path = workspace_root.join(root);
        if !root_path.exists() {
            continue;
        }
        for entry in WalkDir::new(&root_path) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().and_then(|s| s.to_str()) != Some("ch") {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(workspace_root)
                .unwrap_or(entry.path());
            let rel_str = rel.to_string_lossy();
            let contents = fs::read_to_string(entry.path())?;
            for cap in re.captures_iter(&contents) {
                let sym = cap.get(0).unwrap().as_str().to_string();
                set.insert(sym.clone());
                let paths = usage.entry(sym).or_default();
                let rel_owned = rel_str.to_string();
                if !paths.contains(&rel_owned) {
                    paths.push(rel_owned);
                }
            }
        }
    }

    Ok((set, usage))
}

fn truthy(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}
