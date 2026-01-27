use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
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

fn create_base_package(root: &Path, name: &str) -> PathBuf {
    let pkg_root = root.join(name);
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.base
  namespace: Access.Base
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/base.ch",
        r#"#![no_std]
namespace Access.Base;

public class AccessBase
{
    internal int InternalField;
    protected int ProtectedField;
    protected internal int ProtInternal;
    private protected int PrivProt;

    public init()
    {
        InternalField = 1;
        ProtectedField = 2;
        ProtInternal = 3;
        PrivProt = 4;
    }
}
"#,
    );
    pkg_root.join("manifest.yaml")
}

fn create_consumer_package(root: &Path, name: &str, source: &str, base_dep: &str) -> PathBuf {
    let pkg_root = root.join(name);
    write_file(
        &pkg_root,
        "manifest.yaml",
        format!(
            r#"package:
  name: pkg.consumer
  namespace: Access.Consumer
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src

dependencies:
  pkg.base:
    path: {base_dep}
"#
        )
        .as_str(),
    );
    write_file(&pkg_root, "src/lib.ch", source);
    pkg_root.join("manifest.yaml")
}

#[test]
fn cross_package_access_violations_are_reported() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let _base_manifest = create_base_package(root, "pkg.base");

    let consumer_manifest = create_consumer_package(
        root,
        "pkg.consumer",
        r#"#![no_std]
namespace Access.Consumer;

import Access.Base;

public class Derived : AccessBase
{
    public int Allow() => this.ProtInternal + this.ProtectedField; // ok
    public int BadInternal() => this.InternalField; // should be inaccessible (internal)
    public int BadPrivProt() => this.PrivProt; // should be inaccessible (private protected)
    public int BadReceiver(AccessBase other) => other.ProtectedField; // protected-instance rule
}

public class Snooper
{
    public int Bad(AccessBase b) => b.ProtInternal + b.InternalField + b.PrivProt;
}
"#,
        "../pkg.base",
    );

    build_manifest(&consumer_manifest)
        .failure()
        .stderr(predicate::str::contains("InternalField"))
        .stderr(predicate::str::contains("PrivProt"))
        .stderr(predicate::str::contains("ProtectedField"))
        .stderr(predicate::str::contains("ProtInternal"));
}

#[test]
fn protected_internal_allows_derived_across_packages() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let _base_manifest = create_base_package(root, "pkg.base");

    let consumer_manifest = create_consumer_package(
        root,
        "pkg.consumer.ok",
        r#"#![no_std]
namespace Access.Consumer;

import Access.Base;

public class Derived : AccessBase
{
    @allow(dead_code)
    public int Ok() => this.ProtInternal + this.ProtectedField;
}

public class Program
{
    public int Main() => new Derived().Ok();
}
"#,
        "../pkg.base",
    );

    build_manifest(&consumer_manifest).success();
}

#[test]
fn protected_instance_rule_blocks_base_receiver() {
    let temp = tempdir().expect("temp dir");
    let pkg_root = temp.path().join("pkg.instance");
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.instance
  namespace: Access.Instance
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/lib.ch",
        r#"#![no_std]
namespace Access.Instance;

public class Base { protected void Ping() { } }
public class Derived : Base
{
    public void Call(Base other) { other.Ping(); }
}
"#,
    );

    build_manifest(&pkg_root.join("manifest.yaml"))
        .failure()
        .stderr(predicate::str::contains("Ping"));
}

#[test]
fn abstract_types_cannot_be_constructed() {
    let temp = tempdir().expect("temp dir");
    let pkg_root = temp.path().join("pkg.abstract");
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.abstract
  namespace: Access.Abstract
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/lib.ch",
        r#"#![no_std]
namespace Access.Abstract;

public abstract class Ghost { }

public class User
{
    public Ghost Make() => new Ghost();
}
"#,
    );

    build_manifest(&pkg_root.join("manifest.yaml"))
        .failure()
        .stderr(predicate::str::contains("Ghost"));
}

#[test]
fn public_signatures_cannot_expose_internal_types() {
    let temp = tempdir().expect("temp dir");
    let pkg_root = temp.path().join("pkg.signatures");
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.signatures
  namespace: Access.Signatures
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/lib.ch",
        r#"#![no_std]
namespace Access.Signatures;

internal class Hidden { }

public class Api
{
    public Hidden Value => new Hidden();
    public Hidden Make() => new Hidden();
}
"#,
    );

    build_manifest(&pkg_root.join("manifest.yaml"))
        .failure()
        .stderr(predicate::str::contains("Hidden"));
}

#[test]
fn private_protected_allows_same_package_derived() {
    let temp = tempdir().expect("temp dir");
    let pkg_root = temp.path().join("pkg.privprot");
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.privprot
  namespace: Access.PrivProt
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/lib.ch",
        r#"#![no_std]
namespace Access.PrivProt;

public class Base
{
    private protected int Token;
    public init() { Token = 7; }
}

public class Derived : Base
{
    @allow(dead_code)
    public int Peek() => this.Token;
}

public class Program
{
    public int Main() => new Derived().Peek();
}
"#,
    );

    build_manifest(&pkg_root.join("manifest.yaml")).success();
}

#[test]
fn private_protected_blocks_non_derived_same_package() {
    let temp = tempdir().expect("temp dir");
    let pkg_root = temp.path().join("pkg.privprot.fail");
    write_file(
        &pkg_root,
        "manifest.yaml",
        r#"package:
  name: pkg.privprot.fail
  namespace: Access.PrivProt.Fail
  version: 0.0.1

build:
  kind: lib

sources:
  - path: ./src
"#,
    );
    write_file(
        &pkg_root,
        "src/lib.ch",
        r#"#![no_std]
namespace Access.PrivProt.Fail;

public class Base
{
    private protected int Token;
}

public class Snooper
{
    public int Peek(Base b) => b.Token;
}
"#,
    );

    build_manifest(&pkg_root.join("manifest.yaml"))
        .failure()
        .stderr(predicate::str::contains("Token"));
}
