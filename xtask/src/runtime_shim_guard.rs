use std::error::Error;
use std::fs;
use std::path::PathBuf;

const BANNED_TOKENS: &[&str] = &["RustShim", "CHIC_ALLOW_RUST_RUNTIME"];

pub fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("xtask lives inside the workspace");
    let src_root = workspace_root.join("src");
    let mut stack = vec![src_root];
    let mut failures = Vec::new();

    while let Some(path) = stack.pop() {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            for entry in fs::read_dir(&path)? {
                stack.push(entry?.path());
            }
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        for token in BANNED_TOKENS {
            if contents.contains(token) {
                failures.push(format!(
                    "{} contains forbidden token `{}`",
                    path.display(),
                    token
                ));
            }
        }
    }

    if failures.is_empty() {
        println!("lint-runtime-shim: chic runtime is the only selectable backend.");
        Ok(())
    } else {
        eprintln!("lint-runtime-shim: Rust shim references detected:");
        for failure in failures {
            eprintln!("  - {failure}");
        }
        Err("lint-runtime-shim failed".into())
    }
}
