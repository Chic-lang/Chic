use std::fs;
use std::path::Path;

// Some integration-test crates only need the file helpers; keep the clang probe available without
// forcing every crate to call it.
#[allow(dead_code)]
pub fn clang_available() -> bool {
    std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok()
}

// Many FFI-heavy tests pull this module for both single-file and batch writers; keep each helper
// local to avoid per-crate dead-code warnings when only one variant is exercised.
#[allow(dead_code)]
pub fn write_source(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap_or_else(|err| panic!("write source: {err}"));
}

#[allow(dead_code)]
pub fn write_sources(root: &Path, sources: &[(&str, &str)]) {
    for (relative, contents) in sources {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("create dir {}: {err}", parent.display()));
        }
        write_source(&path, contents);
    }
}
