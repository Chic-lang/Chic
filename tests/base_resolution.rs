use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(&path, contents).expect("write file");
}

#[test]
fn internal_base_type_cannot_be_inherited_across_packages() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    let pkg_a = root.join("pkg.a");
    write_file(
        &pkg_a,
        "manifest.yaml",
        r#"package:
  name: pkg.a
  namespace: Shared
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_a,
        "src/lib.ch",
        r#"namespace Shared;

internal class Hidden { }
public class Anchor { }
"#,
    );

    let pkg_b = root.join("pkg.b");
    write_file(
        &pkg_b,
        "manifest.yaml",
        r#"package:
  name: pkg.b
  namespace: Consumer
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
  pkg.a:
    path: ../pkg.a
"#,
    );
    write_file(
        &pkg_b,
        "src/lib.ch",
        r#"namespace Consumer;
import Shared;

public class Derived : Hidden { }
"#,
    );

    let mut cmd = cargo_bin_cmd!("chic");
    let assert = cmd
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(pkg_b.join("manifest.yaml"))
        .args(["--backend", "wasm"])
        .assert();
    assert.failure().stderr(predicates::str::contains("Hidden"));
}
