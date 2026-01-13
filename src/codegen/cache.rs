//! Incremental compilation cache for Chic code generation.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::chic_kind::ChicKind;
use crate::codegen::metadata::MetadataTelemetry;
use crate::codegen::{Backend, CodegenArtifact, CodegenOptions};
use crate::error::Error;
use crate::target::Target;

/// Represents a cache lookup key computed from the compilation inputs.
pub(crate) struct CacheKey {
    pub fingerprint: String,
    pub backend: Backend,
}

/// Result of a cache lookup.
pub(crate) struct CacheHit {
    pub object_path: PathBuf,
    pub artifact_path: PathBuf,
    pub metadata_path: Option<PathBuf>,
    pub library_pack: Option<PathBuf>,
    pub constant_folds: usize,
    pub inlined_functions: Vec<String>,
    pub metadata_telemetry: Option<MetadataTelemetry>,
    pub reflection_path: Option<PathBuf>,
}

/// Inputs required to compute a fingerprint.
pub(crate) struct FingerprintInputs<'a> {
    pub source: &'a str,
    pub textual_ir: &'a str,
    pub target: &'a Target,
    pub kind: ChicKind,
    pub backend: Backend,
    pub options: &'a CodegenOptions,
    pub extra_fingerprint: Option<&'a str>,
}

#[derive(Serialize, Deserialize)]
struct CacheMetadata {
    fingerprint: String,
    backend: String,
    compiler_version: String,
    object_path: String,
    artifact_path: String,
    metadata_object: Option<String>,
    library_pack: Option<String>,
    constant_folds: usize,
    inlined_functions: Vec<String>,
    last_built: u64,
    #[serde(default)]
    reflection_manifest: Option<String>,
}

/// Compute a deterministic fingerprint for the given compilation inputs.
pub(crate) fn compute_fingerprint(inputs: &FingerprintInputs<'_>) -> CacheKey {
    let mut hasher = Hasher::new();
    hasher.update(inputs.source.as_bytes());
    hasher.update(inputs.textual_ir.as_bytes());
    hasher.update(inputs.target.triple().as_bytes());
    hasher.update(inputs.kind.as_str().as_bytes());
    hasher.update(inputs.backend.as_str().as_bytes());
    hasher.update(inputs.options.opt_level.as_flag().as_bytes());
    hasher.update(&[u8::from(inputs.options.keep_object)]);
    hasher.update(&[u8::from(inputs.options.link_final_artifact)]);
    hasher.update(&[u8::from(inputs.options.lto)]);
    hasher.update(&[u8::from(inputs.options.coverage)]);
    hasher.update(&[u8::from(inputs.options.pgo_generate)]);
    hasher.update(&[u8::from(inputs.options.emit_wat_text)]);
    let isa_bytes = inputs.options.cpu_isa.fingerprint_bytes();
    if !isa_bytes.is_empty() {
        hasher.update(&isa_bytes);
    }
    if let Some(bits) = inputs.options.sve_vector_bits {
        hasher.update(&bits.to_le_bytes());
    }
    if let Some(profile) = &inputs.options.pgo_use {
        hasher.update(profile.to_string_lossy().as_bytes());
    }
    if let Some(cc1) = &inputs.options.cc1 {
        hasher.update(&[u8::from(cc1.keep_preprocessed)]);
        for arg in &cc1.extra_args {
            hasher.update(arg.as_bytes());
        }
    }
    if let Some(runtime) = inputs.options.runtime.as_ref() {
        hasher.update(runtime.identity.as_bytes());
        hasher.update(runtime.abi.as_bytes());
        hasher.update(runtime.manifest_hash.as_bytes());
    }
    if let Some(extra) = inputs.extra_fingerprint {
        hasher.update(extra.as_bytes());
    }
    hasher.update(compiler_cache_identity().as_bytes());
    let fingerprint = hasher.finalize().to_hex().to_string();
    CacheKey {
        fingerprint,
        backend: inputs.backend,
    }
}

pub(crate) fn compiler_cache_identity() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let version = env!("CARGO_PKG_VERSION");
        let commit = option_env!("CHIC_GIT_HASH").unwrap_or("unknown");
        let exe_hash = current_exe_hash().unwrap_or_else(|| "unknown".to_string());
        format!("{version}+{commit}+{exe_hash}")
    })
}

fn current_exe_hash() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    let bytes = fs::read(&path).ok()?;
    let digest = blake3::hash(&bytes).to_hex().to_string();
    Some(digest.chars().take(16).collect())
}

/// Attempt to load a cached artifact for the provided key.
pub(crate) fn try_load(output: &Path, key: &CacheKey) -> Option<CacheHit> {
    let meta_path = metadata_path(output);
    let data = fs::read_to_string(&meta_path).ok()?;
    let metadata: CacheMetadata = serde_json::from_str(&data).ok()?;
    if metadata.fingerprint != key.fingerprint {
        return None;
    }
    if metadata.backend != key.backend.as_str() {
        return None;
    }
    if metadata.compiler_version != compiler_cache_identity() {
        return None;
    }

    let base = meta_path.parent().unwrap_or(Path::new("."));
    let object_path = resolve_path(base, &metadata.object_path);
    let artifact_path = resolve_path(base, &metadata.artifact_path);
    let metadata_object = metadata
        .metadata_object
        .as_ref()
        .map(|stored| resolve_path(base, stored));
    let library_pack = metadata
        .library_pack
        .as_ref()
        .map(|stored| resolve_path(base, stored));
    let reflection_manifest = metadata
        .reflection_manifest
        .as_ref()
        .map(|stored| resolve_path(base, stored));

    if !object_path.exists() || !artifact_path.exists() {
        return None;
    }
    if matches!(metadata_object.as_ref(), Some(meta) if !meta.exists()) {
        return None;
    }
    if matches!(library_pack.as_ref(), Some(pack) if !pack.exists()) {
        return None;
    }
    if matches!(reflection_manifest.as_ref(), Some(manifest) if !manifest.exists()) {
        return None;
    }

    Some(CacheHit {
        object_path,
        artifact_path,
        metadata_path: metadata_object,
        library_pack,
        constant_folds: metadata.constant_folds,
        inlined_functions: metadata.inlined_functions,
        metadata_telemetry: None,
        reflection_path: reflection_manifest,
    })
}

/// Record the successful code generation result in the cache metadata.
pub(crate) fn store(
    output: &Path,
    key: &CacheKey,
    artifact: &CodegenArtifact,
) -> Result<(), Error> {
    let meta_path = metadata_path(output);
    if let Some(parent) = meta_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let base = meta_path.parent().unwrap_or(Path::new("."));
    let metadata = CacheMetadata {
        fingerprint: key.fingerprint.clone(),
        backend: key.backend.as_str().to_string(),
        compiler_version: compiler_cache_identity().to_string(),
        object_path: relativise(base, &artifact.object_path),
        artifact_path: relativise(base, &artifact.artifact_path),
        metadata_object: artifact
            .metadata_path
            .as_ref()
            .map(|path| relativise(base, path)),
        library_pack: artifact
            .library_pack
            .as_ref()
            .map(|path| relativise(base, path)),
        constant_folds: artifact.constant_folds,
        inlined_functions: artifact.inlined_functions.clone(),
        last_built: unix_timestamp(),
        reflection_manifest: artifact
            .reflection_metadata_path
            .as_ref()
            .map(|path| relativise(base, path)),
    };

    let json = serde_json::to_string_pretty(&metadata)
        .map_err(|err| Error::Codegen(format!("failed to serialise cache metadata: {err}")))?;
    fs::write(&meta_path, json)?;
    Ok(())
}

fn metadata_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let meta_name = format!("{file_name}.cache.json");
    output.with_file_name(meta_name)
}

fn relativise(base: &Path, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(base) {
        relative.to_string_lossy().into_owned()
    } else {
        path.to_string_lossy().into_owned()
    }
}

fn resolve_path(base: &Path, stored: &str) -> PathBuf {
    let candidate = PathBuf::from(stored);
    if candidate.is_relative() {
        base.join(candidate)
    } else {
        candidate
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
