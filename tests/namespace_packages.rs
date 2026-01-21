use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(&path, contents).expect("write file");
}

fn build_manifest(manifest: &Path) -> assert_cmd::assert::Assert {
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(manifest)
        .args(["--backend", "wasm"]);
    cmd.assert()
}

fn create_std_io_package(root: &Path, name: &str) {
    let pkg_root = root.join(name);
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.io
  namespace: Std.IO
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/stream.cl",
        r#"#![no_std]
namespace Std.IO;

public class Stream
{
    public init() { }
}
"#,
    );
}

fn create_std_compression_package(root: &Path, name: &str) {
    let pkg_root = root.join(name);
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.compression
  namespace: Std.IO.Compression
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
  pkg.io:
    path: ../pkg.io
"#,
    );
    write_file(
        &pkg_root,
        "src/gzip.cl",
        r#"#![no_std]
namespace Std.IO.Compression;

import Std.IO;

public class GZipStream : Std.IO.Stream
{
    public init() { }
}
"#,
    );
}

fn create_consumer_package(root: &Path, name: &str, source: &str) -> PathBuf {
    let pkg_root = root.join(name);
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.app
  namespace: Consumer.Tests
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
  pkg.io:
    path: ../pkg.io
  pkg.compression:
    path: ../pkg.compression
"#,
    );
    write_file(&pkg_root, "src/lib.cl", source);
    pkg_root.join("manifest.yaml")
}

#[test]
fn namespaces_compose_across_packages() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    create_std_io_package(root, "pkg.io");
    create_std_compression_package(root, "pkg.compression");

    let manifest = create_consumer_package(
        root,
        "pkg.app",
        r#"#![no_std]
namespace Consumer.Tests;

import Std.IO;
import Std.IO.Compression;

public class UsesBoth
{
    private Stream _stream;
    private GZipStream _gzip;

    public init(Stream stream, GZipStream gzip)
    {
        _stream = stream;
        _gzip = gzip;
    }
}
"#,
    );

    build_manifest(&manifest).success();
}

#[test]
fn parent_namespace_remains_visible_with_child_import() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    create_std_io_package(root, "pkg.io");
    create_std_compression_package(root, "pkg.compression");

    let manifest = create_consumer_package(
        root,
        "pkg.child_only",
        r#"#![no_std]
namespace Consumer.Tests;

import Std.IO.Compression;

public class ChildOnly
{
    private Std.IO.Stream _stream;
    private GZipStream _gzip;

    public init(Std.IO.Stream stream, GZipStream gzip)
    {
        _stream = stream;
        _gzip = gzip;
    }
}
"#,
    );

    build_manifest(&manifest).success();
}

#[test]
fn aliases_do_not_shadow_parent_or_child_namespaces() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    create_std_io_package(root, "pkg.io");
    create_std_compression_package(root, "pkg.compression");

    let manifest = create_consumer_package(
        root,
        "pkg.aliases",
        r#"#![no_std]
namespace Consumer.Tests;

import IO = Std.IO;
import Compression = Std.IO.Compression;

public class AliasUser
{
    private IO.Stream _stream;
    private Compression.GZipStream _gzip;

    public init(IO.Stream stream, Compression.GZipStream gzip)
    {
        _stream = stream;
        _gzip = gzip;
    }
}
"#,
    );

    build_manifest(&manifest).success();
}

#[test]
fn duplicate_types_across_packages_report_conflicts() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    // First Std.IO contributor
    let left_root = root.join("pkg.left");
    write_file(
        &left_root,
        "manifest.yaml",
        r#"package:
  name: pkg.left
  namespace: Std.IO
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &left_root,
        "src/stream.cl",
        r#"#![no_std]
namespace Std.IO;

public class Stream { }
"#,
    );

    // Second Std.IO contributor with the same type name.
    let right_root = root.join("pkg.right");
    write_file(
        &right_root,
        "manifest.yaml",
        r#"package:
  name: pkg.right
  namespace: Std.IO
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &right_root,
        "src/stream.cl",
        r#"#![no_std]
namespace Std.IO;

public class Stream { }
"#,
    );

    let consumer_root = root.join("pkg.conflict");
    write_file(
        &consumer_root,
        "manifest.yaml",
        r#"package:
  name: pkg.conflict
  namespace: Consumer.Conflicts
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
  pkg.left:
    path: ../pkg.left
  pkg.right:
    path: ../pkg.right
"#,
    );
    write_file(
        &consumer_root,
        "src/lib.cl",
        r#"#![no_std]
namespace Consumer.Conflicts;

public class ConflictConsumer { }
"#,
    );

    let mut cmd = cargo_bin_cmd!("chic");
    let output = cmd
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(consumer_root.join("manifest.yaml"))
        .args(["--backend", "wasm"])
        .output()
        .expect("run build");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("TCK400") && combined.contains("conflicts"),
        "expected conflict diagnostic, output={combined}"
    );
}

#[test]
fn dependency_order_does_not_affect_resolution() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    create_std_io_package(root, "pkg.io");
    create_std_compression_package(root, "pkg.compression");

    let manifest_a = create_consumer_package(
        root,
        "pkg.order_a",
        r#"#![no_std]
namespace Consumer.Tests;

import Std.IO;
import Std.IO.Compression;

public class OrderA { public init(Stream stream) { } }
"#,
    );

    // Recreate consumer manifest with reversed dependency order.
    let manifest_b_root = root.join("pkg.order_b");
    write_file(
        &manifest_b_root,
        "manifest.yaml",
        r#"package:
  name: pkg.order_b
  namespace: Consumer.Tests
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
  pkg.compression:
    path: ../pkg.compression
  pkg.io:
    path: ../pkg.io
"#,
    );
    write_file(
        &manifest_b_root,
        "src/lib.cl",
        r#"#![no_std]
namespace Consumer.Tests;

import Std.IO;
import Std.IO.Compression;

public class OrderB { public init(GZipStream stream) { } }
"#,
    );

    build_manifest(&manifest_a).success();
    build_manifest(&manifest_b_root.join("manifest.yaml")).success();
}
