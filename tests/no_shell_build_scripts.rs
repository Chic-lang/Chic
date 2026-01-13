use std::fs;
use std::path::Path;

#[test]
fn no_shell_build_scripts_remain() {
    let scripts_dir = Path::new("scripts");
    if !scripts_dir.exists() {
        return;
    }
    let mut offenders = Vec::new();
    if let Ok(entries) = fs::read_dir(scripts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("build_") && name.ends_with(".sh") {
                    offenders.push(path);
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "build shell scripts are deprecated; use `chic build` instead. Offenders: {:?}",
        offenders
    );
}
