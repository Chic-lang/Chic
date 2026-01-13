use crate::frontend::diagnostics::{Diagnostic, DiagnosticCode};
use crate::manifest::{Dependency, DependencySource, Manifest, PROJECT_MANIFEST_BASENAME};
use crate::package::version::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

const PKG_RESOLVE_CYCLE: &str = "PKG2001";
const PKG_RESOLVE_CONFLICT: &str = "PKG2002";
const PKG_RESOLVE_MISSING: &str = "PKG2003";
const PKG_RESOLVE_OFFLINE: &str = "PKG2004";
const PKG_RESOLVE_VERSION: &str = "PKG2005";

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: Version,
    pub manifest: Manifest,
    pub root: PathBuf,
    pub source: ResolvedSource,
}

#[derive(Debug, Clone)]
pub enum ResolvedSource {
    Path,
    Git {
        repo: String,
        rev: Option<String>,
        branch: Option<String>,
        tag: Option<String>,
        commit: String,
        subdir: Option<PathBuf>,
    },
    Registry {
        registry: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    pub offline: bool,
    pub cache_dir: Option<PathBuf>,
    pub lockfile: Option<PathBuf>,
}

impl ResolveOptions {
    #[must_use]
    pub fn from_env(lockfile: Option<PathBuf>) -> Self {
        Self {
            offline: env_flag_truthy("CHIC_OFFLINE")
                .or_else(|| env_flag_truthy("CHIC_PACKAGE_OFFLINE"))
                .unwrap_or(false),
            cache_dir: env::var_os("CHIC_PACKAGE_CACHE").map(PathBuf::from),
            lockfile,
        }
    }
}

#[derive(Debug, Default)]
pub struct ResolveOutcome {
    pub packages: Vec<ResolvedPackage>,
    pub diagnostics: Vec<Diagnostic>,
}

struct ResolveCtx<'a> {
    manifest_dir: &'a Path,
    options: &'a ResolveOptions,
    resolved: HashMap<String, ResolvedPackage>,
    resolving: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    cache_dir: PathBuf,
}

impl<'a> ResolveCtx<'a> {
    fn resolver_error(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(message, None).with_code(DiagnosticCode::new(
            code.to_string(),
            Some("package".into()),
        ))
    }
}

pub fn resolve_dependencies(
    manifest: &Manifest,
    manifest_path: &Path,
    options: &ResolveOptions,
) -> ResolveOutcome {
    let Some(manifest_dir) = manifest_path.parent() else {
        return ResolveOutcome {
            packages: Vec::new(),
            diagnostics: vec![
                Diagnostic::error("manifest path must have a parent directory", None).with_code(
                    DiagnosticCode::new(PKG_RESOLVE_MISSING.to_string(), Some("package".into())),
                ),
            ],
        };
    };

    let cache_dir = options.cache_dir.clone().unwrap_or_else(default_cache_dir);

    let mut ctx = ResolveCtx {
        manifest_dir,
        options,
        resolved: HashMap::new(),
        resolving: HashSet::new(),
        diagnostics: Vec::new(),
        cache_dir,
    };

    let mut root_name: Option<String> = None;
    if let Some(package) = manifest.package() {
        if let Some(name) = package.name.as_ref() {
            let version = package
                .version
                .clone()
                .unwrap_or_else(|| Version::new(0, 0, 0));
            root_name = Some(name.clone());
            ctx.resolved.insert(
                name.clone(),
                ResolvedPackage {
                    name: name.clone(),
                    version,
                    manifest: manifest.clone(),
                    root: manifest_dir.to_path_buf(),
                    source: ResolvedSource::Path,
                },
            );
        }
    }

    for dep in manifest.dependencies() {
        resolve_dependency(dep.clone(), None, &mut ctx);
    }

    if let Some(name) = &root_name {
        ctx.resolved.remove(name);
    }

    let mut packages: Vec<_> = ctx.resolved.into_values().collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    if let Some(lockfile) = options.lockfile.as_ref() {
        write_lockfile(lockfile, &packages);
    }

    ResolveOutcome {
        packages,
        diagnostics: ctx.diagnostics,
    }
}

fn resolve_dependency(
    dep: Dependency,
    _parent: Option<&str>,
    ctx: &mut ResolveCtx<'_>,
) -> Option<ResolvedPackage> {
    if let Some(existing) = ctx.resolved.get(&dep.name) {
        if let Some(req) = &dep.requirement {
            if !req.matches(&existing.version) {
                ctx.diagnostics.push(ctx.resolver_error(
                    PKG_RESOLVE_CONFLICT,
                    format!(
                        "dependency `{}` resolved to {}, which does not satisfy requirement `{}`",
                        dep.name,
                        existing.version,
                        fmt_req(req)
                    ),
                ));
            }
        }
        return Some(existing.clone());
    }

    if !ctx.resolving.insert(dep.name.clone()) {
        ctx.diagnostics.push(ctx.resolver_error(
            PKG_RESOLVE_CYCLE,
            format!("detected dependency cycle involving `{}`", dep.name),
        ));
        return None;
    }

    let resolved = match &dep.source {
        DependencySource::Path(path) => resolve_path_dependency(path, dep.clone(), ctx),
        DependencySource::Git {
            repo,
            rev,
            branch,
            tag,
            subdir,
        } => resolve_git_dependency(
            dep.clone(),
            repo,
            rev.as_deref(),
            branch.as_deref(),
            tag.as_deref(),
            subdir.as_ref(),
            ctx,
        ),
        DependencySource::Registry { registry } => {
            resolve_registry_dependency(dep.clone(), registry.clone(), ctx)
        }
    };

    let Some(package) = resolved else {
        ctx.resolving.remove(&dep.name);
        return None;
    };

    if let Some(req) = &dep.requirement {
        if !req.matches(&package.version) {
            ctx.diagnostics.push(ctx.resolver_error(
                PKG_RESOLVE_VERSION,
                format!(
                    "dependency `{}` resolved to {}, which does not satisfy requirement `{}`",
                    dep.name,
                    package.version,
                    fmt_req(req)
                ),
            ));
        }
    }

    for child in package.manifest.dependencies().iter().cloned() {
        resolve_dependency(child, Some(&package.name), ctx);
    }

    ctx.resolving.remove(&dep.name);
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        let manifest_path = package
            .manifest
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        eprintln!(
            "[chic-debug] resolved dependency {} -> {} (root {})",
            dep.name,
            manifest_path,
            package.root.display()
        );
    }
    ctx.resolved.insert(dep.name.clone(), package.clone());
    Some(package)
}

fn resolve_path_dependency(
    path: &Path,
    dep: Dependency,
    ctx: &mut ResolveCtx<'_>,
) -> Option<ResolvedPackage> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        ctx.manifest_dir.join(path)
    };
    let root = joined.canonicalize().unwrap_or_else(|_| joined.clone());
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        eprintln!(
            "[chic-debug] resolve_path_dependency {} joined={} canonical={} exists={}",
            dep.name,
            joined.display(),
            root.display(),
            root.exists()
        );
    }
    let manifest = load_manifest(&root, ctx)?;
    build_resolved_package(dep, root, manifest, ResolvedSource::Path)
}

fn resolve_git_dependency(
    dep: Dependency,
    repo: &str,
    rev: Option<&str>,
    branch: Option<&str>,
    tag: Option<&str>,
    subdir: Option<&PathBuf>,
    ctx: &mut ResolveCtx<'_>,
) -> Option<ResolvedPackage> {
    let git_dir = ctx.cache_dir.join("git");
    if let Err(err) = fs::create_dir_all(&git_dir) {
        ctx.diagnostics.push(ctx.resolver_error(
            PKG_RESOLVE_MISSING,
            format!(
                "failed to prepare git cache dir {}: {err}",
                git_dir.display()
            ),
        ));
        return None;
    }

    let key = format!(
        "{}-{}",
        hash_string(repo),
        rev.or(branch).or(tag).unwrap_or("head")
    );
    let checkout_dir = git_dir.join(sanitize_component(&key));
    if !checkout_dir.exists() {
        if ctx.options.offline {
            ctx.diagnostics.push(ctx.resolver_error(
                PKG_RESOLVE_OFFLINE,
                format!(
                    "offline mode is enabled; cached checkout for `{repo}` not found at {}",
                    checkout_dir.display()
                ),
            ));
            return None;
        }
        if let Err(err) = fs::create_dir_all(&git_dir) {
            ctx.diagnostics.push(ctx.resolver_error(
                PKG_RESOLVE_MISSING,
                format!(
                    "failed to prepare git cache dir {}: {err}",
                    git_dir.display()
                ),
            ));
            return None;
        }
        let status = Command::new("git")
            .args(["clone", repo, checkout_dir.to_string_lossy().as_ref()])
            .status();
        if !status.map(|s| s.success()).unwrap_or(false) {
            ctx.diagnostics.push(ctx.resolver_error(
                PKG_RESOLVE_MISSING,
                format!("failed to clone git repo `{repo}`"),
            ));
            return None;
        }
    } else if !ctx.options.offline {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&checkout_dir)
            .args(["fetch", "--all", "--tags"])
            .status();
    }

    if let Some(rev) = rev {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&checkout_dir)
            .args(["checkout", rev])
            .status();
    } else if let Some(branch) = branch {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&checkout_dir)
            .args(["checkout", branch])
            .status();
    } else if let Some(tag) = tag {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&checkout_dir)
            .args(["checkout", tag])
            .status();
    }

    let commit = Command::new("git")
        .arg("-C")
        .arg(&checkout_dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    let resolved_root = if let Some(subdir) = subdir {
        checkout_dir.join(subdir)
    } else {
        checkout_dir.clone()
    };

    let manifest = load_manifest(&resolved_root, ctx)?;
    let source = ResolvedSource::Git {
        repo: repo.to_string(),
        rev: rev.map(str::to_string),
        branch: branch.map(str::to_string),
        tag: tag.map(str::to_string),
        commit,
        subdir: subdir.cloned(),
    };
    build_resolved_package(dep, resolved_root, manifest, source)
}

fn resolve_registry_dependency(
    dep: Dependency,
    registry: Option<String>,
    ctx: &mut ResolveCtx<'_>,
) -> Option<ResolvedPackage> {
    let base = ctx.cache_dir.join("registry").join(&dep.name);
    if dep.requirement.is_none() {
        ctx.diagnostics.push(ctx.resolver_error(
            PKG_RESOLVE_VERSION,
            format!(
                "dependency `{}` must specify a version when using registry sources",
                dep.name
            ),
        ));
        return None;
    }

    let requirement = dep.requirement.clone().unwrap();
    let versions = find_cached_versions(&base);
    let candidate = versions
        .iter()
        .filter(|version| requirement.matches(version))
        .max()
        .cloned();

    let Some(version) = candidate else {
        ctx.diagnostics.push(ctx.resolver_error(
            PKG_RESOLVE_MISSING,
            format!(
                "no cached registry package found for `{}` matching `{}`",
                dep.name,
                fmt_req(&requirement)
            ),
        ));
        return None;
    };

    let root = base.join(version.to_string());
    let manifest = load_manifest(&root, ctx)?;
    let source = ResolvedSource::Registry { registry };
    build_resolved_package(dep, root, manifest, source)
}

fn build_resolved_package(
    dep: Dependency,
    root: PathBuf,
    manifest: Manifest,
    source: ResolvedSource,
) -> Option<ResolvedPackage> {
    let name = manifest
        .package()
        .and_then(|pkg| pkg.name.clone())
        .unwrap_or(dep.name.clone());
    let version = manifest
        .package_version()
        .cloned()
        .unwrap_or_else(|| Version::new(0, 0, 0));
    Some(ResolvedPackage {
        name,
        version,
        manifest,
        root,
        source,
    })
}

fn load_manifest(root: &Path, ctx: &mut ResolveCtx<'_>) -> Option<Manifest> {
    let manifest_path = root.join(PROJECT_MANIFEST_BASENAME);
    let discovered = Manifest::discover(&manifest_path);
    match discovered {
        Ok(Some(manifest)) => Some(manifest),
        Ok(None) => {
            ctx.diagnostics.push(ctx.resolver_error(
                PKG_RESOLVE_MISSING,
                format!("failed to find manifest under `{}`", root.display()),
            ));
            None
        }
        Err(err) => {
            ctx.diagnostics.push(ctx.resolver_error(
                PKG_RESOLVE_MISSING,
                format!(
                    "failed to load manifest `{}`: {err}",
                    manifest_path.display()
                ),
            ));
            None
        }
    }
}

fn find_cached_versions(base: &Path) -> Vec<Version> {
    let mut versions = Vec::new();
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if let Ok(version) = Version::parse(name) {
                    versions.push(version);
                }
            }
        }
    }
    versions
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn default_cache_dir() -> PathBuf {
    if let Some(path) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(path).join("chic");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join("chic");
    }
    env::temp_dir().join("chic_cache")
}

fn hash_string(input: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn sanitize_component(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn fmt_req(req: &VersionReq) -> String {
    req.to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct Lockfile {
    packages: Vec<LockedPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
    source: LockedSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum LockedSource {
    Path {
        path: String,
    },
    Git {
        repo: String,
        rev: Option<String>,
        branch: Option<String>,
        tag: Option<String>,
        commit: String,
        subdir: Option<String>,
    },
    Registry {
        registry: Option<String>,
    },
}

impl From<&ResolvedPackage> for LockedPackage {
    fn from(pkg: &ResolvedPackage) -> Self {
        let source = match &pkg.source {
            ResolvedSource::Path => LockedSource::Path {
                path: pkg.root.to_string_lossy().into_owned(),
            },
            other => LockedSource::from(other),
        };
        Self {
            name: pkg.name.clone(),
            version: pkg.version.to_string(),
            source,
        }
    }
}

impl From<&ResolvedSource> for LockedSource {
    fn from(source: &ResolvedSource) -> Self {
        match source {
            ResolvedSource::Path => LockedSource::Path {
                path: String::from("."),
            },
            ResolvedSource::Git {
                repo,
                rev,
                branch,
                tag,
                commit,
                subdir,
            } => LockedSource::Git {
                repo: repo.clone(),
                rev: rev.clone(),
                branch: branch.clone(),
                tag: tag.clone(),
                commit: commit.clone(),
                subdir: subdir
                    .as_ref()
                    .and_then(|path| path.to_str().map(str::to_string)),
            },
            ResolvedSource::Registry { registry } => LockedSource::Registry {
                registry: registry.clone(),
            },
        }
    }
}

fn write_lockfile(path: &Path, packages: &[ResolvedPackage]) {
    let lockfile = Lockfile {
        packages: packages.iter().map(LockedPackage::from).collect(),
    };
    if let Ok(serialized) = serde_yaml::to_string(&lockfile) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, serialized);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn detects_version_conflicts_across_dependencies() {
        let dir = tempdir().expect("tempdir");

        let shared_v1 = dir.path().join("shared_v1");
        fs::create_dir_all(&shared_v1).expect("create shared_v1");
        fs::write(
            shared_v1.join("manifest.yaml"),
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

        let shared_v2 = dir.path().join("shared_v2");
        fs::create_dir_all(&shared_v2).expect("create shared_v2");
        fs::write(
            shared_v2.join("manifest.yaml"),
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

        let left_dir = dir.path().join("left");
        fs::create_dir_all(&left_dir).expect("create left dir");
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
  Shared: { path: "../shared_v1", version: "1.0.0" }
"#,
        )
        .expect("write left manifest");

        let right_dir = dir.path().join("right");
        fs::create_dir_all(&right_dir).expect("create right dir");
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

        let root_dir = dir.path().join("root");
        fs::create_dir_all(&root_dir).expect("create root dir");
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

        let root_manifest_path = root_dir.join("manifest.yaml");
        let root_manifest = Manifest::discover(&root_manifest_path)
            .expect("discover root manifest")
            .expect("root manifest missing");
        let options = ResolveOptions::from_env(Some(root_dir.join("manifest.lock")));
        let outcome = resolve_dependencies(&root_manifest, &root_manifest_path, &options);
        let codes: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter_map(|diag| diag.code.as_ref().map(|code| code.code.clone()))
            .collect();
        assert!(
            codes
                .iter()
                .any(|code| code == PKG_RESOLVE_CONFLICT || code == PKG_RESOLVE_VERSION),
            "expected a version conflict diagnostic, got {codes:?}"
        );
    }

    #[test]
    fn resolves_git_dependency_and_reuses_cache_offline() {
        let dir = tempdir().expect("tempdir");
        let repo_dir = dir.path().join("repo");
        fs::create_dir_all(&repo_dir).expect("create repo dir");
        fs::write(
            repo_dir.join("manifest.yaml"),
            r#"
package:
  name: RepoDep
  namespace: RepoDep
  version: 0.1.0
sources:
  - path: src
"#,
        )
        .expect("write repo manifest");

        let git_init = Command::new("git")
            .arg("init")
            .arg(&repo_dir)
            .status()
            .expect("run git init");
        assert!(git_init.success(), "git init failed");
        let git_add = Command::new("git")
            .args([
                "-C",
                repo_dir.to_string_lossy().as_ref(),
                "add",
                "manifest.yaml",
            ])
            .status()
            .expect("run git add");
        assert!(git_add.success(), "git add failed");
        let git_commit = Command::new("git")
            .args([
                "-C",
                repo_dir.to_string_lossy().as_ref(),
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=test",
                "commit",
                "-m",
                "init",
            ])
            .status()
            .expect("run git commit");
        assert!(git_commit.success(), "git commit failed");

        let root_dir = dir.path().join("root");
        fs::create_dir_all(&root_dir).expect("create root dir");
        fs::write(
            root_dir.join("manifest.yaml"),
            format!(
                r#"
package:
  name: Root
  namespace: Root
  version: 1.0.0
sources:
  - path: src
dependencies:
  RepoDep: {{ git: "{}", rev: "HEAD" }}
"#,
                repo_dir.display()
            ),
        )
        .expect("write root manifest");

        let root_manifest_path = root_dir.join("manifest.yaml");
        let root_manifest = Manifest::discover(&root_manifest_path)
            .expect("discover root manifest")
            .expect("root manifest missing");
        let cache_dir = dir.path().join("cache");
        let options = ResolveOptions {
            offline: false,
            cache_dir: Some(cache_dir.clone()),
            lockfile: Some(root_dir.join("manifest.lock")),
        };
        let outcome = resolve_dependencies(&root_manifest, &root_manifest_path, &options);
        assert!(
            outcome.packages.iter().any(|pkg| pkg.name == "RepoDep"),
            "expected git package to resolve"
        );
        assert!(
            outcome.diagnostics.is_empty(),
            "unexpected diagnostics resolving git package: {:?}",
            outcome.diagnostics
        );

        let offline_opts = ResolveOptions {
            offline: true,
            cache_dir: Some(cache_dir.clone()),
            lockfile: Some(root_dir.join("manifest.lock")),
        };
        let offline_outcome =
            resolve_dependencies(&root_manifest, &root_manifest_path, &offline_opts);
        assert!(
            offline_outcome
                .packages
                .iter()
                .any(|pkg| pkg.name == "RepoDep"),
            "offline resolve should reuse cached git checkout"
        );
        assert!(
            offline_outcome.diagnostics.is_empty(),
            "unexpected offline diagnostics: {:?}",
            offline_outcome.diagnostics
        );
    }
}
