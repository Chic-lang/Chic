use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::codegen::compiler_cache_identity;
use crate::error::{Error, Result};

pub(crate) const MANIFEST_VERSION: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FileState {
    pub path: String,
    pub size: u64,
    pub mtime: u128,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ArtifactState {
    #[serde(default)]
    pub objects: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct BuildManifest {
    pub schema_version: u32,
    pub toolchain: String,
    pub target: String,
    pub backend: String,
    pub profile: String,
    pub options_fingerprint: String,
    #[serde(default)]
    pub runtime_identity: String,
    #[serde(default)]
    pub runtime_abi: String,
    #[serde(default)]
    pub runtime_manifest_hash: String,
    pub files: Vec<FileState>,
    pub artifacts: ArtifactState,
}

pub(crate) fn load_manifest(path: &Path) -> Option<BuildManifest> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub(crate) fn write_manifest(path: &Path, manifest: &BuildManifest) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(manifest)
        .map_err(|err| Error::internal(format!("failed to serialise cache manifest: {err}")))?;
    fs::write(path, data)?;
    Ok(())
}

pub(crate) fn snapshot_file(
    display_path: String,
    fs_path: &Path,
    _previous: Option<&FileState>,
) -> Result<FileState> {
    let metadata = fs::metadata(fs_path)?;
    let size = metadata.len();
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_nanos())
        .unwrap_or(0);

    let hash = hash_file(fs_path)?;

    Ok(FileState {
        path: display_path,
        size,
        mtime,
        hash,
        object: None,
    })
}

pub(crate) fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

pub(crate) fn options_fingerprint(parts: &[impl AsRef<str>]) -> String {
    let mut hasher = Hasher::new();
    for part in parts {
        hasher.update(part.as_ref().as_bytes());
    }
    hasher.update(compiler_cache_identity().as_bytes());
    hasher.finalize().to_hex().to_string()
}
