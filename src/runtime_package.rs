use std::fs;
use std::path::{Path, PathBuf};

use blake3::Hasher;

use crate::error::{Error, Result};
use crate::manifest::{
    Manifest, PROJECT_MANIFEST_BASENAME, RuntimeCompat, RuntimeProvides, RuntimeSelection,
};
use crate::package::version::{Version, VersionReq};

pub use crate::manifest::RuntimeKind;

pub const EXPECTED_RUNTIME_ABI: &str = "rt-abi-1";

#[derive(Debug, Clone)]
pub struct ResolvedRuntime {
    pub kind: RuntimeKind,
    pub package: String,
    pub version: Version,
    pub abi: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest_hash: String,
    pub manifest: Manifest,
}

impl ResolvedRuntime {
    #[must_use]
    pub fn identity(&self) -> String {
        format!("{}@{}", self.package, self.version)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResolution {
    pub selection: RuntimeSelection,
    pub resolved: ResolvedRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMetadata {
    pub identity: String,
    pub package: String,
    pub version: Version,
    pub abi: String,
    pub kind: RuntimeKind,
    pub manifest_hash: String,
}

pub fn resolve_runtime(
    selection: Option<&RuntimeSelection>,
    fallback_kind: RuntimeKind,
    workspace_root: &Path,
) -> Result<RuntimeResolution> {
    let mut selection = selection
        .cloned()
        .unwrap_or_else(|| default_selection(fallback_kind));
    let root = runtime_root(&selection, workspace_root);
    let manifest_path = root.join(PROJECT_MANIFEST_BASENAME);
    let manifest = Manifest::discover(&manifest_path)?.ok_or_else(|| {
        Error::internal(format!(
            "runtime manifest `{}` not found; expected runtime package at {}",
            PROJECT_MANIFEST_BASENAME,
            root.display()
        ))
    })?;
    let runtime_name = manifest
        .package()
        .and_then(|pkg| pkg.name.clone())
        .ok_or_else(|| {
            Error::internal(format!(
                "runtime manifest `{}` is missing package.name",
                manifest_path.display()
            ))
        })?;
    if runtime_name != selection.package {
        return Err(Error::internal(format!(
            "runtime selection requested package `{}` but manifest declares `{runtime_name}` ({})",
            selection.package,
            manifest_path.display()
        )));
    }
    let runtime_version = manifest.package_version().ok_or_else(|| {
        Error::internal(format!(
            "runtime manifest `{}` is missing package.version",
            manifest_path.display()
        ))
    })?;
    validate_version(selection.version.as_ref(), runtime_version)?;
    let compiler_version = compiler_version();
    enforce_compatibility(selection.compat, runtime_version, &compiler_version)?;

    let provides = manifest.runtime_provides().ok_or_else(|| {
        Error::internal(format!(
            "runtime manifest `{}` must declare runtime.provides",
            manifest_path.display()
        ))
    })?;
    validate_provider(&selection, provides)?;

    let abi = provides
        .abi
        .clone()
        .or_else(|| selection.abi.clone())
        .unwrap_or_else(|| EXPECTED_RUNTIME_ABI.to_string());
    if abi != EXPECTED_RUNTIME_ABI {
        return Err(Error::internal(format!(
            "runtime `{}` provides ABI `{abi}`, but compiler expects `{EXPECTED_RUNTIME_ABI}`; select a runtime package built for `{EXPECTED_RUNTIME_ABI}`",
            runtime_name
        )));
    }
    let manifest_hash = hash_runtime_package(&root, &manifest_path, &manifest)?;
    selection.path = Some(root.clone());
    Ok(RuntimeResolution {
        selection,
        resolved: ResolvedRuntime {
            kind: provides.kind,
            package: runtime_name,
            version: runtime_version.clone(),
            abi,
            root,
            manifest_path,
            manifest_hash,
            manifest,
        },
    })
}

fn runtime_root(selection: &RuntimeSelection, workspace_root: &Path) -> PathBuf {
    if let Some(path) = selection.path.as_ref() {
        return path.clone();
    }
    workspace_root
        .join("packages")
        .join(selection.package.as_str())
}

fn validate_version(requirement: Option<&VersionReq>, version: &Version) -> Result<()> {
    if let Some(req) = requirement {
        if !req.matches(version) {
            return Err(Error::internal(format!(
                "runtime version {} does not satisfy requirement `{}`",
                version, req
            )));
        }
    }
    Ok(())
}

fn enforce_compatibility(
    policy: RuntimeCompat,
    runtime_version: &Version,
    compiler_version: &Version,
) -> Result<()> {
    match policy {
        RuntimeCompat::Strict => {
            let same_major = runtime_version.major == compiler_version.major;
            let same_minor = runtime_version.minor == compiler_version.minor;
            if !same_major || !same_minor {
                return Err(Error::internal(format!(
                    "runtime version {} is not compatible with compiler {} under strict policy (requires matching major.minor)",
                    runtime_version, compiler_version
                )));
            }
        }
        RuntimeCompat::AllowMinor => {
            if runtime_version.major != compiler_version.major {
                return Err(Error::internal(format!(
                    "runtime version {} is not compatible with compiler {} (major version mismatch)",
                    runtime_version, compiler_version
                )));
            }
        }
    }
    Ok(())
}

fn validate_provider(selection: &RuntimeSelection, provides: &RuntimeProvides) -> Result<()> {
    if selection.kind != provides.kind {
        return Err(Error::internal(format!(
            "runtime `{}` provides `{}` but selection requested `{}`",
            selection.package,
            provides.kind.as_str(),
            selection.kind.as_str()
        )));
    }
    if let (Some(expected), Some(found)) = (&selection.abi, &provides.abi) {
        if expected.trim() != found.trim() {
            return Err(Error::internal(format!(
                "runtime `{}` declares ABI `{found}`, but manifest requested `{expected}`",
                selection.package
            )));
        }
    }
    if provides.abi.is_none() && selection.abi.is_none() {
        return Err(Error::internal(format!(
            "runtime `{}` must declare runtime.provides.abi",
            selection.package
        )));
    }
    Ok(())
}

fn hash_manifest(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn hash_runtime_package(root: &Path, manifest_path: &Path, manifest: &Manifest) -> Result<String> {
    let mut hasher = Hasher::new();
    hasher.update(&hash_manifest(manifest_path)?.as_bytes());

    let mut sources = Vec::<PathBuf>::new();
    for source_root in manifest.derived_source_roots() {
        let base = root.join(&source_root.path);
        collect_cl_sources(&base, &mut sources)?;
    }
    sources.sort();
    for path in sources {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update(&[0]);
        hasher.update(&fs::read(&path)?);
        hasher.update(&[0]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn collect_cl_sources(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(root)?;
    if metadata.is_file() {
        if root.extension().and_then(|ext| ext.to_str()) == Some("cl") {
            out.push(root.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_cl_sources(&path, out)?;
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("cl")
        {
            out.push(path);
        }
    }
    Ok(())
}

fn compiler_version() -> Version {
    let text = env!("CARGO_PKG_VERSION");
    Version::parse(text).unwrap_or_else(|_| Version::new(0, 0, 0))
}

fn default_selection(kind: RuntimeKind) -> RuntimeSelection {
    let package = match kind {
        RuntimeKind::Native => "runtime.native",
        RuntimeKind::NoStd => "runtime.no_std",
    };
    RuntimeSelection {
        kind,
        package: package.to_string(),
        version: None,
        path: None,
        compat: RuntimeCompat::AllowMinor,
        abi: Some(EXPECTED_RUNTIME_ABI.to_string()),
        require_native_runtime: None,
    }
}

impl From<&ResolvedRuntime> for RuntimeMetadata {
    fn from(runtime: &ResolvedRuntime) -> Self {
        Self {
            identity: runtime.identity(),
            package: runtime.package.clone(),
            version: runtime.version.clone(),
            abi: runtime.abi.clone(),
            kind: runtime.kind,
            manifest_hash: runtime.manifest_hash.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn compiler_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn write_runtime_manifest(root: &Path, abi: &str, version: &str, kind: &str) -> PathBuf {
        let runtime_root = root.join("packages").join("runtime.native");
        fs::create_dir_all(runtime_root.join("src")).expect("create runtime dir");
        let manifest = format!(
            "package:\n  name: runtime.native\n  namespace: Std.Runtime.Native\n  version: {version}\nruntime:\n  provides:\n    kind: {kind}\n    abi: {abi}\n"
        );
        fs::write(runtime_root.join(PROJECT_MANIFEST_BASENAME), manifest).expect("write manifest");
        runtime_root
    }

    fn selection_with_path(path: PathBuf, abi: Option<String>) -> RuntimeSelection {
        RuntimeSelection {
            kind: RuntimeKind::Native,
            package: "runtime.native".into(),
            version: None,
            path: Some(path),
            compat: RuntimeCompat::AllowMinor,
            abi,
            require_native_runtime: None,
        }
    }

    #[test]
    fn resolve_runtime_rejects_abi_mismatch() {
        let dir = tempdir().expect("tempdir");
        let version = compiler_version();
        let runtime_root =
            write_runtime_manifest(dir.path(), "rt-abi-mismatch", &version, "native");
        let selection = selection_with_path(runtime_root, Some(EXPECTED_RUNTIME_ABI.to_string()));
        let result = resolve_runtime(Some(&selection), RuntimeKind::Native, dir.path());
        assert!(result.is_err(), "expected ABI mismatch failure");
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains(EXPECTED_RUNTIME_ABI),
            "error message should mention expected ABI: {message}"
        );
    }

    #[test]
    fn resolve_runtime_loads_identity_and_hash() {
        let dir = tempdir().expect("tempdir");
        let version = compiler_version();
        let runtime_root =
            write_runtime_manifest(dir.path(), EXPECTED_RUNTIME_ABI, &version, "native");
        let selection = selection_with_path(runtime_root.clone(), None);
        let resolution = resolve_runtime(Some(&selection), RuntimeKind::Native, dir.path())
            .expect("resolve runtime");
        assert!(
            resolution.resolved.identity().contains(&version),
            "identity should include runtime version"
        );
        assert!(
            !resolution.resolved.manifest_hash.is_empty(),
            "manifest hash should be populated"
        );
        assert_eq!(resolution.resolved.root, runtime_root);
    }
}
