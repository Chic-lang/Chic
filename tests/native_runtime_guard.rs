use std::path::Path;

#[test]
fn native_runtime_linked_by_default() {
    assert_eq!(
        option_env!("CHIC_NATIVE_RUNTIME_LINKED"),
        Some("1"),
        "Native runtime must be linked by default"
    );
}

#[test]
fn native_runtime_defines_decimal_drop_symbols() {
    let archive = std::env::var("CHIC_NATIVE_RUNTIME_ARCHIVE")
        .expect("CHIC_NATIVE_RUNTIME_ARCHIVE unset; build.rs should provide it");
    let bytes =
        std::fs::read(&archive).expect("native runtime archive missing; rebuild before testing");

    for sym in [
        "__cl_drop__DecimalBinary",
        "__cl_drop__DecimalTernary",
        "__cl_drop__Std__Runtime__Native__DecimalBinary",
        "__cl_drop__Std__Runtime__Native__DecimalTernary",
        "__cl_drop__fn_decimal__decimal_____decimal",
        "__cl_drop__fn_decimal__decimal__decimal_____decimal",
    ] {
        let needle = sym.as_bytes();
        let found = bytes.windows(needle.len()).any(|window| window == needle);
        assert!(
            found,
            "native runtime archive `{archive}` is missing symbol `{sym}`; drop stubs must be defined"
        );
    }
}

#[test]
fn native_runtime_has_no_math_shim_exports() {
    let archive = std::env::var("CHIC_NATIVE_RUNTIME_ARCHIVE")
        .expect("CHIC_NATIVE_RUNTIME_ARCHIVE unset; build.rs should provide it");
    let bytes =
        std::fs::read(&archive).expect("native runtime archive missing; rebuild before testing");
    let needle = ["chic_rt_", "math_"].join("");
    let missing = !bytes
        .windows(needle.len())
        .any(|window| window == needle.as_bytes());
    assert!(
        missing,
        "native runtime archive `{archive}` still exposes math shim exports; Std.Math must call libm directly"
    );
}

#[test]
fn no_legacy_runtime_paths_in_code() {
    let roots = ["src", "scripts", "runtime_adapter", "build.rs", "xtask"];
    let forbidden = ["src/native_runtime", "src/no_std_runtime"];
    let mut hits = Vec::new();
    for root in roots {
        let path = Path::new(root);
        if path.is_dir() {
            scan_dir(path, &forbidden, &mut hits);
        } else if path.is_file() {
            scan_file(path, &forbidden, &mut hits);
        }
    }
    if !hits.is_empty() {
        panic!(
            "forbidden runtime path references found:\n{}",
            hits.join("\n")
        );
    }
}

fn scan_dir(dir: &Path, forbidden: &[&str], hits: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, forbidden, hits);
        } else if path.is_file() {
            scan_file(&path, forbidden, hits);
        }
    }
}

fn scan_file(path: &Path, forbidden: &[&str], hits: &mut Vec<String>) {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    for needle in forbidden {
        if contents.contains(needle) {
            hits.push(format!(
                "{} references forbidden path `{}`",
                path.display(),
                needle
            ));
        }
    }
}

#[test]
fn runtime_adapter_folder_removed() {
    assert!(
        !Path::new("runtime_adapter").exists(),
        "runtime_adapter folder must be deleted; migrate semantics to Chic"
    );
}
