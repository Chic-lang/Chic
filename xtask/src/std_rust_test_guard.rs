use std::error::Error;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn run() -> Result<(), Box<dyn Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("failed to resolve repository root")?;
    let packages_root = repo_root.join("packages");
    if !packages_root.exists() {
        println!(
            "lint-stdlib-rust-tests: packages directory missing at {}",
            packages_root.display()
        );
        return Ok(());
    }

    let mut offenders: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&packages_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if !name.starts_with("std") {
            continue;
        }
        for item in WalkDir::new(&path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|item| item.file_type().is_file())
        {
            if item.path().extension().and_then(|ext| ext.to_str()) == Some("rs") {
                offenders.push(item.path().to_path_buf());
            }
        }
    }

    if offenders.is_empty() {
        println!("lint-stdlib-rust-tests: no Rust sources under Std packages.");
        return Ok(());
    }

    eprintln!("lint-stdlib-rust-tests: Rust sources detected under Std packages:");
    for path in offenders {
        eprintln!("  - {}", path.display());
    }
    Err("lint-stdlib-rust-tests failed".into())
}
