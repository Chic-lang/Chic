use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SHIM_PATHS: &[&str] = &[
    "packages/runtime.native/shims.c",
    "packages/runtime.native/startup_trampoline.c",
];
const EXTRA_SYMBOLS: &[&str] = &["Std__Async__CancellationTokenSource__Create_byte_"];
const LEGACY_SHIM_SYMBOLS: &[&str] = &[
    "chic_thread_drop",
    "chic_thread_invoke",
    "chic_rt_host_thread_detach",
    "chic_rt_host_thread_join",
    "chic_rt_host_thread_sleep_ms",
    "chic_rt_host_thread_spawn",
    "chic_rt_host_thread_yield",
    "chic_rt_abort_unhandled_exception",
    "chic_rt_async_task_bool_result",
    "chic_rt_async_task_int_result",
    "chic_rt_async_task_result",
    "chic_rt_clear_pending_exception",
    "chic_rt_debug_mark",
    "chic_rt_decimal_add",
    "chic_rt_decimal_add_out",
    "chic_rt_decimal_clone",
    "chic_rt_decimal_div",
    "chic_rt_decimal_div_out",
    "chic_rt_decimal_dot",
    "chic_rt_decimal_dot_out",
    "chic_rt_decimal_fma",
    "chic_rt_decimal_fma_out",
    "chic_rt_decimal_matmul",
    "chic_rt_decimal_mul",
    "chic_rt_decimal_mul_out",
    "chic_rt_decimal_rem",
    "chic_rt_decimal_rem_out",
    "chic_rt_decimal_sub",
    "chic_rt_decimal_sub_out",
    "chic_rt_decimal_sum",
    "chic_rt_decimal_sum_out",
    "chic_rt_has_pending_exception",
    "chic_rt_peek_pending_exception",
    "chic_rt_startup_call_entry",
    "chic_rt_startup_call_entry_async",
    "chic_rt_startup_call_testcase",
    "chic_rt_startup_call_testcase_async",
    "chic_rt_startup_complete_entry_async",
    "chic_rt_startup_complete_testcase_async",
    "chic_rt_startup_cstr_to_string",
    "chic_rt_startup_descriptor_snapshot",
    "chic_rt_startup_exit",
    "chic_rt_startup_has_run_tests_flag",
    "chic_rt_startup_i32_to_string",
    "chic_rt_startup_ptr_at",
    "chic_rt_startup_raw_argc",
    "chic_rt_startup_raw_argv",
    "chic_rt_startup_raw_envp",
    "chic_rt_startup_slice_to_string",
    "chic_rt_startup_store_state",
    "chic_rt_startup_test_descriptor",
    "chic_rt_startup_usize_to_string",
    "chic_rt_string_as_slice",
    "chic_rt_string_borrow",
    "chic_rt_string_from_slice",
    "chic_rt_string_new",
    "chic_rt_take_pending_exception",
    "chic_rt_throw",
    "chic_rt_trace_enter",
    "chic_rt_trace_exit",
    "chic_rt_trace_flush",
    "chic_rt_math_",
    "chic_rt_ptr_",
    "__chic_startup_descriptor",
];
const STRICT_ENV: &str = "CHIC_SHIM_STRICT";
const GLOBAL_STRICT_ENV: &str = "CHIC_NATIVE_ONLY_STRICT";
const ALLOWED_NATIVE_SOURCES: &[&str] = &[
    "packages/runtime.native/src/support.ll",
    "packages/runtime.native/src/support.ch",
];
const SCAN_ROOTS: &[&str] = &[
    "src",
    "packages",
    "tests",
    "benches",
    "xtask",
    "runtime_adapter",
    "templates",
    "scripts",
];
const TEXT_EXTENSIONS: &[&str] = &[
    "rs", "ch", "c", "h", "ll", "toml", "yaml", "yml", "md", "json", "txt",
];

pub fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask lives inside the workspace");
    let strict = truthy(STRICT_ENV) || truthy(GLOBAL_STRICT_ENV);

    let sym_re = Regex::new(r"chic[0-9A-Za-z_]*")?;
    let mut files = Vec::new();
    // In non-strict mode, only symbols still present in shim files are scanned (ideally none).
    // Strict mode always scans the full banned list to prevent reintroduction.
    let mut symbol_set: HashSet<String> = HashSet::new();
    if strict {
        symbol_set.extend(EXTRA_SYMBOLS.iter().map(|s| s.to_string()));
        symbol_set.extend(LEGACY_SHIM_SYMBOLS.iter().map(|s| s.to_string()));
    }
    let mut any_present = false;

    for path in SHIM_PATHS {
        let full_path = workspace_root.join(path);
        let exists = full_path.exists();
        any_present |= exists;
        let contents = if exists {
            Some(fs::read_to_string(&full_path).map_err(|err| {
                format!(
                    "lint-shim-state: failed to read {}: {err}",
                    full_path.display()
                )
            })?)
        } else {
            None
        };
        let non_comment = contents.as_ref().map(|c| strip_comments(c));
        let mut symbols: Vec<String> = contents
            .as_deref()
            .map(|c| {
                sym_re
                    .captures_iter(c)
                    .filter_map(|cap| cap.get(0).map(|m| m.as_str().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        symbols.sort();
        symbols.dedup();
        symbol_set.extend(symbols.iter().cloned());

        files.push((full_path, exists, non_comment, symbols));
    }

    let mut symbols: Vec<String> = symbol_set.into_iter().collect();
    symbols.sort();

    if symbols.is_empty() {
        println!("lint-shim-state: no chic* shim symbols detected in native C sources.");
    } else {
        println!(
            "lint-shim-state: scanning {} banned shim symbols for references",
            symbols.len()
        );
    }

    let shim_paths: Vec<PathBuf> = files.iter().map(|(p, _, _, _)| p.clone()).collect();
    let references = scan_references(&symbols, &workspace_root, &shim_paths)?;
    if !references.is_empty() {
        eprintln!("lint-shim-state: shim symbols referenced elsewhere:");
        for (sym, paths) in references.iter() {
            let mut paths = paths.clone();
            paths.sort();
            eprintln!("  - {sym} ({})", paths.len());
            for path in paths.iter().take(5) {
                eprintln!("      {path}");
            }
            if paths.len() > 5 {
                eprintln!("      (+{} more)", paths.len() - 5);
            }
        }
        return Err("lint-shim-state failed: shim symbol references remain".into());
    }

    let native_sources = list_native_sources(&workspace_root);
    if !native_sources.is_empty() {
        let mut unexpected: Vec<_> = native_sources
            .iter()
            .filter(|p| {
                !ALLOWED_NATIVE_SOURCES
                    .iter()
                    .any(|allowed| Path::new(allowed) == p.as_path())
            })
            .cloned()
            .collect();
        unexpected.sort();
        unexpected.dedup();
        if !unexpected.is_empty() {
            for entry in unexpected {
                eprintln!(
                    "lint-shim-state: forbidden native runtime source {}",
                    entry.display()
                );
            }
            return Err("lint-shim-state failed: unexpected native runtime sources".into());
        }
    }

    if any_present {
        return Err("lint-shim-state failed: shim files must be deleted".into());
    }

    println!("lint-shim-state: shim files, symbols, and references absent.");

    Ok(())
}

fn scan_references(
    symbols: &[String],
    workspace_root: &Path,
    skip_paths: &[PathBuf],
) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    let mut results: HashMap<String, Vec<String>> = HashMap::new();
    if symbols.is_empty() {
        return Ok(results);
    }
    let skip: HashSet<PathBuf> = skip_paths.iter().cloned().collect();
    for root in SCAN_ROOTS {
        let root_path = workspace_root.join(root);
        if !root_path.exists() {
            continue;
        }
        for entry in WalkDir::new(&root_path) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }
            if entry.path().components().any(|c| c.as_os_str() == "obj")
                || entry.path().components().any(|c| c.as_os_str() == "target")
            {
                continue;
            }
            let canonical = entry.path();
            if canonical.starts_with(workspace_root.join("docs")) {
                continue;
            }
            if canonical
                == workspace_root
                    .join("xtask")
                    .join("src")
                    .join("shim_state_guard.rs")
            {
                continue;
            }
            if skip.contains(canonical) {
                continue;
            }
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if !TEXT_EXTENSIONS.contains(&ext) {
                    continue;
                }
            }
            let contents = match fs::read_to_string(entry.path()) {
                Ok(data) => data,
                Err(_) => continue,
            };
            for sym in symbols {
                if contents.contains(sym) {
                    let rel = entry
                        .path()
                        .strip_prefix(workspace_root)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .to_string();
                    results.entry(sym.clone()).or_default().push(rel);
                }
            }
        }
    }
    Ok(results)
}

fn list_native_sources(workspace_root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let native_root = workspace_root.join("src").join("native_runtime");
    if !native_root.exists() {
        return sources;
    }
    for entry in WalkDir::new(&native_root) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.components().any(|c| c.as_os_str() == "obj") {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if matches!(ext, "c" | "cc" | "cpp" | "ll" | "s" | "S") {
                sources.push(
                    path.strip_prefix(workspace_root)
                        .unwrap_or(path)
                        .to_path_buf(),
                );
            }
        }
    }
    sources.sort();
    sources.dedup();
    sources
}

fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '/' {
            match chars.peek() {
                Some('/') => {
                    while let Some(c) = chars.next() {
                        if c == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '*' {
                            if let Some('/') = chars.peek() {
                                chars.next();
                                break;
                            }
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(ch);
    }
    out
}

fn truthy(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}
