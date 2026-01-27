use super::*;
use crate::chic_kind::ChicKind;
use crate::codegen::Backend;
use crate::driver::{
    collect_alloc_files, collect_core_files, collect_foundation_files,
    collect_runtime_package_files, driver_stack_size, run_with_stack, stdlib_files_for,
};
use crate::frontend::conditional::ConditionalDefines;
use crate::manifest::Manifest;
use crate::runtime_package::{RuntimeKind, resolve_runtime};
use crate::target::Target;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::tempdir;

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn build_frontend_for_source(
    source: &str,
    load_stdlib: bool,
    runtime_kind: RuntimeKind,
) -> FrontendState {
    let dir = tempdir().expect("tempdir");
    let src_path = dir.path().join("main.ch");
    fs::write(&src_path, source).expect("write source");

    let target = Target::host();
    let core_files = collect_core_files().expect("core files");
    let foundation_files = collect_foundation_files().expect("foundation files");
    let alloc_files = collect_alloc_files().expect("alloc files");
    let runtime_resolution =
        resolve_runtime(None, runtime_kind, Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("runtime resolution");
    let runtime = runtime_resolution.resolved;
    let nostd_runtime_files = if runtime_kind == RuntimeKind::NoStd {
        collect_runtime_package_files(&runtime).expect("no_std runtime files")
    } else {
        Vec::new()
    };
    let std_files = stdlib_files_for(ChicKind::Executable, Backend::Llvm).expect("std files");

    run_with_stack(driver_stack_size(), move || {
        let mut defines = ConditionalDefines::default();
        if std::env::var("CHIC_ENABLE_ALLOC")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            defines.set_bool("ENABLE_ALLOC", true);
        }
        CompilerPipelineBuilder::new("test", &[src_path], &target, defines)
            .backend(Backend::Llvm)
            .kind(ChicKind::Executable)
            .load_stdlib(load_stdlib)
            .corelib_files(&core_files)
            .foundationlib_files(&foundation_files)
            .alloclib_files(&alloc_files)
            .nostd_runtime_files(&nostd_runtime_files)
            .stdlib_files(&std_files)
            .runtime(Some(runtime))
            .build()
            .execute()
    })
    .expect("pipeline execute")
}

fn module_inputs(frontend: &FrontendState) -> HashSet<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    frontend
        .modules
        .iter()
        .filter(|module| module.is_stdlib)
        .map(|module| {
            module
                .input
                .strip_prefix(&manifest_dir)
                .unwrap_or(&module.input)
                .to_path_buf()
        })
        .collect()
}

fn write_manifest(dir: &tempfile::TempDir, contents: &str) -> Manifest {
    let manifest_path = dir.path().join("manifest.yaml");
    fs::write(&manifest_path, contents).expect("write manifest");
    Manifest::discover(&manifest_path)
        .expect("manifest discovery should succeed")
        .expect("expected manifest to be found")
}

fn module_from_source(
    root: &Path,
    relative: &str,
    source: &str,
    files: &mut FileCache,
) -> FrontendModuleState {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    let owned_source = source.to_string();
    let file_id = files.add_file(&path, owned_source.clone());
    let parse = parse_module_in_file(&owned_source, file_id).expect("parse module");
    FrontendModuleState {
        input: path,
        source: owned_source,
        parse,
        manifest: None,
        is_stdlib: false,
        requires_codegen: true,
    }
}

#[test]
fn pipeline_skips_stdlib_for_no_std_crates() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var("CHIC_ENABLE_ALLOC");
    }
    let frontend =
        build_frontend_for_source("#![no_std]\nnamespace Kernel;", true, RuntimeKind::NoStd);
    let inputs = module_inputs(&frontend);
    let core_files: HashSet<_> = collect_core_files()
        .expect("core files")
        .into_iter()
        .collect();
    let foundation_files: HashSet<_> = collect_foundation_files()
        .expect("foundation files")
        .into_iter()
        .collect();
    let alloc_files: HashSet<_> = collect_alloc_files()
        .expect("alloc files")
        .into_iter()
        .collect();
    let std_files: HashSet<_> = stdlib_files_for(ChicKind::Executable, Backend::Llvm)
        .expect("std files")
        .into_iter()
        .collect();

    assert!(
        inputs.iter().any(|path| core_files.contains(path)),
        "core modules should be loaded for no_std crates (inputs={inputs:?})"
    );
    assert!(
        !inputs.iter().any(|path| alloc_files.contains(path)),
        "alloc should not load by default for #![no_std] (inputs={inputs:?})"
    );
    assert!(
        !inputs.iter().any(|path| foundation_files.contains(path)),
        "foundation should not load by default for #![no_std] (inputs={inputs:?})"
    );
    assert!(
        !inputs.iter().any(|path| std_files.contains(path)),
        "std should not load for #![no_std] (inputs={inputs:?})"
    );
    assert!(
        frontend
            .combined_ast
            .crate_attributes
            .std_setting
            .is_no_std()
    );
}

#[test]
fn pipeline_allows_alloc_opt_in_for_no_std() {
    let _lock = ENV_LOCK.lock().unwrap();
    let previous = std::env::var("CHIC_ENABLE_ALLOC").ok();
    unsafe {
        std::env::set_var("CHIC_ENABLE_ALLOC", "1");
    }
    let frontend =
        build_frontend_for_source("#![no_std]\nnamespace Kernel;", true, RuntimeKind::NoStd);
    let inputs = module_inputs(&frontend);
    let foundation_files: HashSet<_> = collect_foundation_files()
        .expect("foundation files")
        .into_iter()
        .collect();
    let alloc_files: HashSet<_> = collect_alloc_files()
        .expect("alloc files")
        .into_iter()
        .collect();
    let std_files: HashSet<_> = stdlib_files_for(ChicKind::Executable, Backend::Llvm)
        .expect("std files")
        .into_iter()
        .collect();

    assert!(
        inputs.iter().any(|path| alloc_files.contains(path)),
        "alloc should load when explicitly enabled (inputs={inputs:?})"
    );
    assert!(
        inputs.iter().any(|path| foundation_files.contains(path)),
        "foundation should load when alloc is enabled for #![no_std] (inputs={inputs:?})"
    );
    assert!(
        !inputs.iter().any(|path| std_files.contains(path)),
        "std should remain disabled in #![no_std] crates (inputs={inputs:?})"
    );

    unsafe {
        match previous {
            Some(value) => std::env::set_var("CHIC_ENABLE_ALLOC", value),
            None => std::env::remove_var("CHIC_ENABLE_ALLOC"),
        }
    }
}

#[test]
fn pipeline_loads_full_stdlib_for_std_crates() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var("CHIC_ENABLE_ALLOC");
    }
    let frontend = build_frontend_for_source(
        "
namespace App;

public int Main() { return 0; }
",
        true,
        RuntimeKind::Native,
    );
    let inputs = module_inputs(&frontend);
    let core_files: HashSet<_> = collect_core_files()
        .expect("core files")
        .into_iter()
        .collect();
    let alloc_files: HashSet<_> = collect_alloc_files()
        .expect("alloc files")
        .into_iter()
        .collect();
    let foundation_files: HashSet<_> = collect_foundation_files()
        .expect("foundation files")
        .into_iter()
        .collect();
    let std_files: HashSet<_> = stdlib_files_for(ChicKind::Executable, Backend::Llvm)
        .expect("std files")
        .into_iter()
        .collect();

    assert!(
        inputs.iter().any(|path| core_files.contains(path)),
        "core should load for std crates (inputs={inputs:?})"
    );
    assert!(
        inputs.iter().any(|path| alloc_files.contains(path)),
        "alloc should load for std crates (inputs={inputs:?})"
    );
    assert!(
        inputs.iter().any(|path| foundation_files.contains(path)),
        "foundation should load for std crates (inputs={inputs:?})"
    );
    assert!(
        inputs.iter().any(|path| std_files.contains(path)),
        "std should load for std crates (inputs={inputs:?})"
    );
    assert!(
        !frontend
            .combined_ast
            .crate_attributes
            .std_setting
            .is_no_std(),
        "std_setting should default to std when unspecified"
    );
}

#[test]
fn friend_directive_allows_out_of_prefix_namespace() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: CorePkg
  namespace: CorePkg
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/friend.ch",
        r#"
@friend("Compat.Legacy")
namespace Compat.Legacy.Widget;

struct Foo {}
"#,
        &mut files,
    )];

    enforce_namespace_rules(&manifest, None, &mut modules);
    assert!(
        modules[0].parse.diagnostics.is_empty(),
        "expected friend directive to allow Compat.* namespace: {:?}",
        modules[0].parse.diagnostics
    );
}

#[test]
fn namespace_outside_prefix_without_friend_reports_error() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: Package.Core
  namespace: Package.Core
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/other.ch",
        r#"
namespace Other.External;

struct Widget {}
"#,
        &mut files,
    )];

    enforce_namespace_rules(&manifest, None, &mut modules);
    let diag = modules[0]
        .parse
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code.as_ref().map(|code| code.code.as_str()) == Some(PKG_NAMESPACE_OUT_OF_SCOPE)
        })
        .unwrap_or_else(|| {
            panic!(
                "expected PKG namespace diagnostic: {:?}",
                modules[0].parse.diagnostics
            )
        });
    assert!(
        diag.message.contains("Package.Core"),
        "diagnostic should mention package prefix: {}",
        diag.message
    );
    let span = diag
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("diagnostic should carry span");
    let snippet = &modules[0].source[span.start..span.end];
    assert!(
        snippet.contains("namespace Other.External"),
        "span should highlight offending namespace, got {snippet:?}"
    );
}

#[test]
fn block_namespace_outside_prefix_reports_friend_hint() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: Package.Core
  namespace: Package.Core
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/block.ch",
        r#"
namespace Compat { namespace Legacy { struct Widget {} } }
"#,
        &mut files,
    )];

    enforce_namespace_rules(&manifest, None, &mut modules);
    let diag = modules[0]
        .parse
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code.as_ref().map(|code| code.code.as_str()) == Some(PKG_NAMESPACE_OUT_OF_SCOPE)
                && diag.message.contains("Compat.Legacy")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected PKG namespace diagnostic for Compat.Legacy: {:?}",
                modules[0].parse.diagnostics
            )
        });
    assert!(
        diag.message.contains("missing an `@friend` grant"),
        "diagnostic should explain missing friend: {}",
        diag.message
    );
    assert!(
        diag.notes
            .iter()
            .any(|note| note.contains("package.friends") && note.contains("Compat.Legacy")),
        "friend note should mention Compat.Legacy: {:?}",
        diag.notes
    );
}

#[test]
fn manifest_friend_mismatch_reports_missing_grant() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: Package.Core
  namespace: Package.Core
  friends:
    - Compat.Leg
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/main.ch",
        r#"
namespace Compat.Legacy.Services;

struct Widget {}
"#,
        &mut files,
    )];

    enforce_namespace_rules(&manifest, None, &mut modules);
    let diag = modules[0]
        .parse
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code.as_ref().map(|code| code.code.as_str()) == Some(PKG_NAMESPACE_OUT_OF_SCOPE)
        })
        .unwrap_or_else(|| {
            panic!(
                "expected PKG namespace diagnostic: {:?}",
                modules[0].parse.diagnostics
            )
        });
    assert!(
        diag.notes
            .iter()
            .any(|note| note.contains("allowed prefixes")),
        "diagnostic should list allowed prefixes when a friend prefix exists"
    );
    assert!(
        diag.notes.iter().any(|note| note.contains("Compat.Legacy")),
        "diagnostic should mention missing Compat.Legacy friend grant: {:?}",
        diag.notes
    );
}

#[test]
fn manifest_friend_rejects_self_prefix() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: Package.Core
  namespace: Package.Core
  friends:
    - Package.Core
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/main.ch",
        r#"
namespace Package.Core;

struct Widget {}
"#,
        &mut files,
    )];

    enforce_namespace_rules(&manifest, None, &mut modules);
    let diag = modules[0]
        .parse
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code.as_ref().map(|code| code.code.as_str()) == Some(PKG_FRIEND_SELF_PREFIX)
        })
        .unwrap_or_else(|| {
            panic!(
                "expected self-prefix friend diagnostic: {:?}",
                modules[0].parse.diagnostics
            )
        });
    assert!(
        diag.message.contains("does not need to be listed"),
        "expected self-prefix diagnostic message, got {}",
        diag.message
    );
}

#[test]
fn empty_friend_prefix_reports_error() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: Package.Core
  namespace: Package.Core
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/main.ch",
        r#"
@friend("")
namespace Package.Core;

struct Widget {}
"#,
        &mut files,
    )];

    enforce_namespace_rules(&manifest, None, &mut modules);
    let diag = modules[0]
        .parse
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code.as_ref().map(|code| code.code.as_str()) == Some(PKG_FRIEND_INVALID_PREFIX)
        })
        .unwrap_or_else(|| {
            panic!(
                "expected invalid friend diagnostic: {:?}",
                modules[0].parse.diagnostics
            )
        });
    let span = diag
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("diagnostic should carry span");
    let snippet = &modules[0].source[span.start..span.end];
    assert!(
        snippet.contains("@friend"),
        "span should reference friend directive, got {snippet:?}"
    );
}

#[test]
fn package_imports_require_manifest_entry() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(
        &dir,
        r#"
package:
  name: Package.Core
  namespace: Package.Core
sources:
  - path: src
"#,
    );

    let mut files = FileCache::default();
    let mut modules = vec![module_from_source(
        dir.path(),
        "src/main.ch",
        r#"
@package("Other.Core")
namespace Package.Core;

struct Widget {}
"#,
        &mut files,
    )];

    validate_package_imports(&manifest, &mut modules);
    let codes: Vec<_> = modules[0]
        .parse
        .diagnostics
        .iter()
        .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
        .collect();
    assert!(
        codes.contains(&PKG_PACKAGE_UNKNOWN.to_string()),
        "expected package dependency diagnostic; codes={codes:?}"
    );
    assert!(
        codes.contains(&PKG_PACKAGE_DIRECTIVES_DISALLOWED.to_string()),
        "expected @package directive diagnostic; codes={codes:?}"
    );
}

#[test]
fn resolves_path_dependencies_with_package_imports() {
    let dir = tempdir().expect("tempdir");

    let shared_dir = dir.path().join("shared");
    fs::create_dir_all(shared_dir.join("src")).expect("create shared src");
    fs::write(
        shared_dir.join("manifest.yaml"),
        r#"
package:
  name: Shared
  namespace: Shared
  version: 1.0.0
sources:
  - path: src
"#,
    )
    .expect("write shared manifest");
    fs::write(
        shared_dir.join("src/lib.ch"),
        r#"
namespace Shared;

public struct SharedValue { }
"#,
    )
    .expect("write shared source");

    let left_dir = dir.path().join("left");
    fs::create_dir_all(left_dir.join("src")).expect("create left src");
    fs::write(
        left_dir.join("manifest.yaml"),
        r#"
package:
  name: Left
  namespace: Left
  version: 1.0.0
sources:
  - path: src
dependencies:
  Shared: { path: "../shared", version: "1.0.0" }
"#,
    )
    .expect("write left manifest");
    fs::write(
        left_dir.join("src/lib.ch"),
        r#"
@package("Shared")
namespace Left;

public struct LeftValue { public Shared.SharedValue value; }
"#,
    )
    .expect("write left source");

    let right_dir = dir.path().join("right");
    fs::create_dir_all(right_dir.join("src")).expect("create right src");
    fs::write(
        right_dir.join("manifest.yaml"),
        r#"
package:
  name: Right
  namespace: Right
  version: 1.0.0
sources:
  - path: src
dependencies:
  Shared: { path: "../shared", version: "1.0.0" }
"#,
    )
    .expect("write right manifest");
    fs::write(
        right_dir.join("src/lib.ch"),
        r#"
@package("Shared")
namespace Right;

public struct RightValue { public Shared.SharedValue value; }
"#,
    )
    .expect("write right source");

    let root_dir = dir.path().join("root");
    fs::create_dir_all(root_dir.join("src")).expect("create root src");
    fs::write(
        root_dir.join("manifest.yaml"),
        r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
dependencies:
  Left: { path: "../left", version: "1.0.0" }
  Right: { path: "../right", version: "1.0.0" }
"#,
    )
    .expect("write root manifest");
    fs::write(
        root_dir.join("src/main.ch"),
        r#"
@package("Left")
@package("Right")
namespace Root;

public struct UsePackages {
public Left.LeftValue left;
public Right.RightValue right;
}
"#,
    )
    .expect("write root source");

    let root_manifest = Manifest::discover(&root_dir.join("manifest.yaml"))
        .expect("discover root manifest")
        .expect("root manifest missing");
    let target = Target::host();
    let root_src = root_dir.join("src/main.ch");
    let inputs = [root_src.clone()];
    let pipeline =
        CompilerPipelineBuilder::new("test", &inputs, &target, ConditionalDefines::default())
            .manifest(Some(root_manifest))
            .load_stdlib(false)
            .build();
    let frontend = pipeline.execute().expect("pipeline execute");
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        let inputs: Vec<_> = frontend
            .modules
            .iter()
            .map(|module| module.input.clone())
            .collect();
        eprintln!("[chic-debug] loaded modules: {:?}", inputs);
    }

    let diags: Vec<_> = frontend
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .collect();
    let codes: Vec<_> = diags
        .iter()
        .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
        .collect();
    let left_src = fs::canonicalize(left_dir.join("src/lib.ch"))
        .unwrap_or_else(|_| left_dir.join("src/lib.ch"));
    let right_src = fs::canonicalize(right_dir.join("src/lib.ch"))
        .unwrap_or_else(|_| right_dir.join("src/lib.ch"));
    let shared_src = fs::canonicalize(shared_dir.join("src/lib.ch"))
        .unwrap_or_else(|_| shared_dir.join("src/lib.ch"));
    assert!(
        codes
            .iter()
            .all(|code| code == PKG_PACKAGE_DIRECTIVES_DISALLOWED),
        "expected only @package diagnostics, got {:?}",
        codes
    );
    assert!(
        frontend
            .modules
            .iter()
            .any(|module| module.input == left_src),
        "left package should be loaded"
    );
    assert!(
        frontend
            .modules
            .iter()
            .any(|module| module.input == right_src),
        "right package should be loaded"
    );
    assert!(
        frontend
            .modules
            .iter()
            .any(|module| module.input == shared_src),
        "shared package should be loaded"
    );
}

#[test]
fn package_directive_reports_unresolved_dependency() {
    let dir = tempdir().expect("tempdir");
    let root_dir = dir.path().join("root");
    fs::create_dir_all(root_dir.join("src")).expect("create root src");
    fs::write(
        root_dir.join("manifest.yaml"),
        r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
dependencies:
  Missing: { path: "../missing", version: "1.0.0" }
"#,
    )
    .expect("write root manifest");
    fs::write(
        root_dir.join("src/main.ch"),
        r#"
@package("Missing")
namespace Root;

public struct UsesMissing { }
"#,
    )
    .expect("write root source");

    let root_manifest = Manifest::discover(&root_dir.join("manifest.yaml"))
        .expect("discover root manifest")
        .expect("root manifest missing");
    let target = Target::host();
    let inputs = [root_dir.join("src/main.ch")];
    let pipeline =
        CompilerPipelineBuilder::new("test", &inputs, &target, ConditionalDefines::default())
            .manifest(Some(root_manifest))
            .load_stdlib(false)
            .build();
    let frontend = pipeline.execute().expect("pipeline execute");
    let codes: Vec<_> = frontend
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
        .collect();
    assert!(
        codes.iter().any(|code| code == PKG_PACKAGE_UNRESOLVED),
        "expected unresolved package diagnostics, got {codes:?}"
    );
    assert!(
        codes
            .iter()
            .any(|code| code == PKG_PACKAGE_DIRECTIVES_DISALLOWED),
        "expected @package directive diagnostic, got {codes:?}"
    );
}

#[test]
fn manifest_requires_std_dependency_when_stdlib_enabled() {
    let dir = tempdir().expect("tempdir");
    let root_dir = dir.path().join("root");
    fs::create_dir_all(root_dir.join("src")).expect("create root src");
    fs::write(
        root_dir.join("manifest.yaml"),
        r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
"#,
    )
    .expect("write root manifest");
    fs::write(
        root_dir.join("src/main.ch"),
        r#"
namespace Root;

public struct UsesStd { public int Value; }
"#,
    )
    .expect("write root source");

    let root_manifest = Manifest::discover(&root_dir.join("manifest.yaml"))
        .expect("discover root manifest")
        .expect("root manifest missing");
    let target = Target::host();
    let inputs = [root_dir.join("src/main.ch")];
    let pipeline =
        CompilerPipelineBuilder::new("test", &inputs, &target, ConditionalDefines::default())
            .manifest(Some(root_manifest))
            .load_stdlib(true)
            .build();
    let frontend = pipeline.execute().expect("pipeline execute");
    let codes: Vec<_> = frontend
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
        .collect();
    assert!(
        codes.iter().any(|code| code == PKG_PACKAGE_STD_MISSING),
        "expected std dependency diagnostic, got {codes:?}"
    );
}

#[test]
fn package_directive_reports_version_mismatch() {
    let dir = tempdir().expect("tempdir");

    let shared_dir = dir.path().join("shared");
    fs::create_dir_all(shared_dir.join("src")).expect("create shared src");
    fs::write(
        shared_dir.join("manifest.yaml"),
        r#"
package:
  name: Shared
  namespace: Shared
  version: 1.0.0
sources:
  - path: src
"#,
    )
    .expect("write shared manifest");
    fs::write(
        shared_dir.join("src/lib.ch"),
        r#"
namespace Shared;

public struct SharedValue { }
"#,
    )
    .expect("write shared source");

    let root_dir = dir.path().join("root");
    fs::create_dir_all(root_dir.join("src")).expect("create root src");
    fs::write(
        root_dir.join("manifest.yaml"),
        r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
dependencies:
  Shared: { path: "../shared", version: "2.0.0" }
"#,
    )
    .expect("write root manifest");
    fs::write(
        root_dir.join("src/main.ch"),
        r#"
@package("Shared")
namespace Root;

public struct UsePackages { public Shared.SharedValue value; }
"#,
    )
    .expect("write root source");

    let root_manifest = Manifest::discover(&root_dir.join("manifest.yaml"))
        .expect("discover root manifest")
        .expect("root manifest missing");
    let target = Target::host();
    let inputs = [root_dir.join("src/main.ch")];
    let pipeline =
        CompilerPipelineBuilder::new("test", &inputs, &target, ConditionalDefines::default())
            .manifest(Some(root_manifest))
            .load_stdlib(false)
            .build();
    let frontend = pipeline.execute().expect("pipeline execute");
    let codes: Vec<_> = frontend
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
        .collect();
    assert!(
        codes
            .iter()
            .any(|code| code == PKG_PACKAGE_VERSION_MISMATCH),
        "expected version mismatch diagnostics, got {codes:?}"
    );
    assert!(
        codes
            .iter()
            .any(|code| code == PKG_PACKAGE_DIRECTIVES_DISALLOWED),
        "expected @package directive diagnostic, got {codes:?}"
    );
}

#[test]
fn trims_unreferenced_dependency_exports() {
    let dir = tempdir().expect("tempdir");

    let shared_dir = dir.path().join("shared");
    fs::create_dir_all(shared_dir.join("src")).expect("create shared src");
    fs::write(
        shared_dir.join("manifest.yaml"),
        r#"
package:
  name: Shared
  namespace: Shared
  version: 1.0.0
sources:
  - path: src
"#,
    )
    .expect("write shared manifest");
    fs::write(
        shared_dir.join("src/lib.ch"),
        r#"
namespace Shared;

@export("shared_one")
public int One() { return 1; }

@export("shared_two")
public int Two() { return 2; }
"#,
    )
    .expect("write shared source");

    let left_dir = dir.path().join("left");
    fs::create_dir_all(left_dir.join("src")).expect("create left src");
    fs::write(
        left_dir.join("manifest.yaml"),
        r#"
package:
  name: Left
  namespace: Left
  version: 1.0.0
sources:
  - path: src
dependencies:
  Shared: { path: "../shared", version: "1.0.0" }
"#,
    )
    .expect("write left manifest");
    fs::write(
        left_dir.join("src/lib.ch"),
        r#"
@package("Shared")
namespace Left;

public int UseOne() { return Shared.One(); }
public int Unused() { return Shared.Two(); }
"#,
    )
    .expect("write left source");

    let root_dir = dir.path().join("root");
    fs::create_dir_all(root_dir.join("src")).expect("create root src");
    fs::write(
        root_dir.join("manifest.yaml"),
        r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
dependencies:
  Left: { path: "../left", version: "1.0.0" }
"#,
    )
    .expect("write root manifest");
    fs::write(
        root_dir.join("src/main.ch"),
        r#"
@package("Left")
namespace Root;

public int Main() { return Left.UseOne(); }
"#,
    )
    .expect("write root source");

    let root_manifest = Manifest::discover(&root_dir.join("manifest.yaml"))
        .expect("discover root manifest")
        .expect("root manifest missing");
    let target = Target::host();
    let inputs = [root_dir.join("src/main.ch")];
    let pipeline =
        CompilerPipelineBuilder::new("test", &inputs, &target, ConditionalDefines::default())
            .manifest(Some(root_manifest))
            .load_stdlib(false)
            .build();
    let frontend = pipeline.execute().expect("pipeline execute");

    let left_path = fs::canonicalize(left_dir.join("src/lib.ch")).unwrap();
    let shared_path = fs::canonicalize(shared_dir.join("src/lib.ch")).unwrap();
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        for module in &frontend.modules {
            if !module.parse.diagnostics.is_empty() {
                eprintln!(
                    "[chic-debug] diagnostics for {}: {:?}",
                    module.input.display(),
                    module
                        .parse
                        .diagnostics
                        .iter()
                        .map(|d| d.code.as_ref().map(|c| c.code.clone()).unwrap_or_default())
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    let _left_idx = frontend
        .modules
        .iter()
        .position(|module| {
            fs::canonicalize(&module.input)
                .map(|path| path == left_path)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            panic!(
                "left module not loaded; modules: {:?}",
                frontend
                    .modules
                    .iter()
                    .map(|m| m.input.clone())
                    .collect::<Vec<_>>()
            )
        });
    let _shared_idx = frontend
        .modules
        .iter()
        .position(|module| {
            fs::canonicalize(&module.input)
                .map(|path| path == shared_path)
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            panic!(
                "shared module not loaded; modules: {:?}",
                frontend
                    .modules
                    .iter()
                    .map(|m| m.input.clone())
                    .collect::<Vec<_>>()
            )
        });

    // Validate exports trimming instead of per-function lists to avoid fragile symbol shapes.
    let exports: Vec<_> = frontend
        .mir_module
        .exports
        .iter()
        .map(|export| export.function.clone())
        .collect();
    assert!(
        exports.iter().any(|name| name.ends_with("Shared::One")),
        "expected Shared::One export to remain: {exports:?}"
    );
    assert!(
        !exports.iter().any(|name| name.ends_with("Shared::Two")),
        "Shared::Two export should be trimmed when unreachable: {exports:?}"
    );
    assert!(
        !frontend
            .mir_module
            .exports
            .iter()
            .any(|export| export.function.ends_with("Shared::Two")),
        "exports should exclude unreachable functions"
    );
}

#[test]
fn reports_dependency_version_conflicts() {
    let dir = tempdir().expect("tempdir");

    let shared_v1_dir = dir.path().join("shared_v1");
    fs::create_dir_all(shared_v1_dir.join("src")).expect("create shared v1 src");
    fs::write(
        shared_v1_dir.join("manifest.yaml"),
        r#"
package:
  name: Shared
  namespace: Shared
  version: 1.0.0
sources:
  - path: src
"#,
    )
    .expect("write shared v1 manifest");
    fs::write(
        shared_v1_dir.join("src/lib.ch"),
        "namespace Shared; public int Value() { return 1; }",
    )
    .expect("write shared v1 source");

    let shared_v2_dir = dir.path().join("shared_v2");
    fs::create_dir_all(shared_v2_dir.join("src")).expect("create shared v2 src");
    fs::write(
        shared_v2_dir.join("manifest.yaml"),
        r#"
package:
  name: Shared
  namespace: Shared
  version: 2.0.0
sources:
  - path: src
"#,
    )
    .expect("write shared v2 manifest");
    fs::write(
        shared_v2_dir.join("src/lib.ch"),
        "namespace Shared; public int Value() { return 2; }",
    )
    .expect("write shared v2 source");

    let left_dir = dir.path().join("left");
    fs::create_dir_all(left_dir.join("src")).expect("create left src");
    fs::write(
        left_dir.join("manifest.yaml"),
        r#"
package:
  name: Left
  namespace: Left
  version: 1.0.0
sources:
  - path: src
dependencies:
  Shared: { path: "../shared_v1", version: ">=1.0.0 <2.0.0" }
"#,
    )
    .expect("write left manifest");
    fs::write(
        left_dir.join("src/lib.ch"),
        r#"
@package("Shared")
namespace Left;

public int UseShared() { return Shared::Value(); }
"#,
    )
    .expect("write left source");

    let right_dir = dir.path().join("right");
    fs::create_dir_all(right_dir.join("src")).expect("create right src");
    fs::write(
        right_dir.join("manifest.yaml"),
        r#"
package:
  name: Right
  namespace: Right
  version: 1.0.0
sources:
  - path: src
dependencies:
  Shared: { path: "../shared_v2", version: "2.0.0" }
"#,
    )
    .expect("write right manifest");
    fs::write(
        right_dir.join("src/lib.ch"),
        r#"
@package("Shared")
namespace Right;

public int UseShared() { return Shared::Value(); }
"#,
    )
    .expect("write right source");

    let root_dir = dir.path().join("root");
    fs::create_dir_all(root_dir.join("src")).expect("create root src");
    fs::write(
        root_dir.join("manifest.yaml"),
        r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
dependencies:
  Left: { path: "../left", version: "1.0.0" }
  Right: { path: "../right", version: "1.0.0" }
"#,
    )
    .expect("write root manifest");
    fs::write(
        root_dir.join("src/main.ch"),
        r#"
@package("Left")
@package("Right")
namespace Root;

public int Main() { return Left::UseShared() + Right::UseShared(); }
"#,
    )
    .expect("write root source");

    let root_manifest = Manifest::discover(&root_dir.join("manifest.yaml"))
        .expect("discover root manifest")
        .expect("root manifest missing");
    let target = Target::host();
    let inputs = [root_dir.join("src/main.ch")];
    let pipeline =
        CompilerPipelineBuilder::new("test", &inputs, &target, ConditionalDefines::default())
            .manifest(Some(root_manifest))
            .load_stdlib(false)
            .build();
    let frontend = pipeline.execute().expect("pipeline execute");

    let codes: Vec<_> = frontend
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
        .collect();
    assert!(
        codes.iter().any(|code| code == "PKG2002"),
        "expected PKG2002 conflict diagnostic, got {codes:?}"
    );
}

#[test]
fn pipeline_sets_no_main_crate_attribute() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var("CHIC_ENABLE_ALLOC");
    }
    let frontend =
        build_frontend_for_source("#![no_main]\nnamespace Kernel;", true, RuntimeKind::Native);
    assert!(
        frontend
            .combined_ast
            .crate_attributes
            .main_setting
            .is_no_main(),
        "expected crate_attributes.main_setting to record no_main"
    );
    assert!(
        frontend.mir_module.attributes.is_no_main(),
        "mir attributes should capture no_main flag"
    );
}

#[test]
fn pipeline_loads_no_std_runtime_shim() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::remove_var("CHIC_ENABLE_ALLOC");
    }
    let frontend =
        build_frontend_for_source("#![no_std]\nnamespace Kernel;", true, RuntimeKind::NoStd);
    let inputs = module_inputs(&frontend);
    let runtime = resolve_runtime(
        None,
        RuntimeKind::NoStd,
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .expect("resolve runtime")
    .resolved;
    let nostd_files: HashSet<_> = collect_runtime_package_files(&runtime)
        .expect("no_std runtime files")
        .into_iter()
        .collect();
    assert!(
        inputs.iter().any(|path| nostd_files.contains(path)),
        "no_std runtime shim should load for #![no_std] crates (inputs={inputs:?})"
    );
}
