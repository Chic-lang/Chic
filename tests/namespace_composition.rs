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

fn manifest(name: &str, namespace: &str, deps: &str) -> String {
    format!(
        r#"package:
  name: {name}
  namespace: {namespace}
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
{deps}
"#
    )
}

#[test]
fn namespaces_open_across_packages() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    // Package providing Std.IO
    let pkg_io = root.join("std.io");
    write_file(&pkg_io, "manifest.yaml", &manifest("std.io", "Std.IO", ""));
    write_file(
        &pkg_io,
        "src/lib.ch",
        r#"namespace Std.IO;

public class Stream { public int Read() { return 1; } }
"#,
    );

    // Package providing Std.IO.Compression in a different package
    let pkg_compress = root.join("std.compression");
    write_file(
        &pkg_compress,
        "manifest.yaml",
        &manifest(
            "std.compression",
            "Std.IO.Compression",
            "  std.io:\n    path: ../std.io\n",
        ),
    );
    write_file(
        &pkg_compress,
        "src/lib.ch",
        r#"namespace Std.IO.Compression;

import Std.IO;

public class GZipStream : Stream { public int Decode() { return Read(); } }
"#,
    );

    // Consumer package pulling both and using them together.
    let pkg_app = root.join("app");
    write_file(
        &pkg_app,
        "manifest.yaml",
        &manifest(
            "app",
            "App",
            r"  std.io:
    path: ../std.io
  std.compression:
    path: ../std.compression
",
        ),
    );
    write_file(
        &pkg_app,
        "src/main.ch",
        r#"namespace App;
import Std.IO;
import Std.IO.Compression;

public class Program
{
    public int Run()
    {
        var stream = new Stream();
        var gzip = new GZipStream();
        return stream.Read() + gzip.Decode();
    }
}
"#,
    );

    let mut cmd = cargo_bin_cmd!("chic");
    let assert = cmd
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_SKIP_MIR_VERIFY", "1")
        .arg("build")
        .arg(pkg_app.join("manifest.yaml"))
        .args(["--backend", "wasm"])
        .assert();
    assert.success();
}
