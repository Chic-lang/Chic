//! Code generation drivers for the Chic bootstrap compiler.

mod cache;
pub mod cc1;
mod isa;
mod library;
pub mod llvm;
pub(crate) mod metadata;
mod text;
pub mod wasm;
pub(crate) use cache::compiler_cache_identity;

#[cfg(test)]
pub(crate) use wasm::{sample_loop_function, sample_match_function, test_emit_module};

pub use isa::{CpuIsaConfig, CpuIsaTier};
pub(crate) use library::FileRole;
pub use text::generate_text;
pub use text::stream::{TextStreamConfig, TextStreamMetrics, stream_module};

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::chic_kind::ChicKind;
use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::error::Error;
use crate::frontend::ast::Module;
use crate::hash_glue::SynthesisedHashGlue;
use crate::mir::MirModule;
use crate::perf::PerfMetadata;
use crate::runtime_package::{RuntimeKind, RuntimeMetadata};
use crate::target::{Target, TargetArch};
use crate::type_metadata::SynthesisedTypeMetadata;

#[derive(Debug, Clone, Default)]
pub struct FfiConfig {
    pub search_paths: Vec<String>,
    pub default_pattern: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NativeLinkOptions {
    pub search_paths: Vec<PathBuf>,
    pub libraries: Vec<String>,
    pub has_weak_imports: bool,
    pub runtime: Option<RuntimeMetadata>,
}

pub(crate) fn collect_native_link_options(
    mir: &MirModule,
    options: &CodegenOptions,
) -> NativeLinkOptions {
    let mut out = NativeLinkOptions::default();
    if let Some(config) = options.ffi_config.as_ref() {
        out.search_paths
            .extend(config.search_paths.iter().map(PathBuf::from));
    }
    out.libraries.extend(
        mir.attributes
            .link_libraries
            .iter()
            .map(|lib| lib.name.clone()),
    );
    out.has_weak_imports = mir.functions.iter().any(|function| {
        function.is_weak_import
            || function
                .extern_spec
                .as_ref()
                .map_or(false, |spec| spec.weak)
    });
    out.has_weak_imports |= mir.statics.iter().any(|var| var.is_weak_import);
    out.runtime = options.runtime.clone();
    if std::env::var_os("CHIC_DEBUG_LINK").is_some() {
        let weak_functions: Vec<_> = mir
            .functions
            .iter()
            .filter(|function| {
                function.is_weak_import
                    || function
                        .extern_spec
                        .as_ref()
                        .map_or(false, |spec| spec.weak)
            })
            .map(|function| function.name.clone())
            .collect();
        let weak_statics: Vec<_> = mir
            .statics
            .iter()
            .filter(|var| var.is_weak && var.initializer.is_none())
            .map(|var| var.qualified.clone())
            .collect();
        eprintln!(
            "[chic-debug link] weak imports present: {} (functions: {}, statics: {})",
            out.has_weak_imports,
            weak_functions.len(),
            weak_statics.len()
        );
        if !weak_functions.is_empty() {
            eprintln!("[chic-debug link] weak functions: {:?}", weak_functions);
        }
        if !weak_statics.is_empty() {
            eprintln!("[chic-debug link] weak statics: {:?}", weak_statics);
        }
        let debug_functions: Vec<_> = mir
            .functions
            .iter()
            .map(|function| {
                (
                    function.name.clone(),
                    function.is_weak_import,
                    function
                        .extern_spec
                        .as_ref()
                        .map(|spec| spec.weak)
                        .unwrap_or(false),
                )
            })
            .collect();
        eprintln!("[chic-debug link] functions: {:?}", debug_functions);
    }
    out
}

/// Available machine code backends for the bootstrap compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Llvm,
    Wasm,
    Cc1,
}

impl Backend {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Llvm => "llvm",
            Backend::Wasm => "wasm",
            Backend::Cc1 => "cc1",
        }
    }

    /// Return whether this backend is currently usable in the bootstrap toolchain.
    #[must_use]
    pub fn is_available(self) -> bool {
        match self {
            Backend::Llvm | Backend::Wasm | Backend::Cc1 => true,
        }
    }
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Optimisation level recognised by the codegen drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
    Oz,
}

impl OptLevel {
    #[must_use]
    pub fn as_flag(self) -> &'static str {
        match self {
            OptLevel::O0 => "0",
            OptLevel::O1 => "1",
            OptLevel::O2 => "2",
            OptLevel::O3 => "3",
            OptLevel::Os => "s",
            OptLevel::Oz => "z",
        }
    }
}

/// Shared code generation options.
#[derive(Debug, Clone)]
pub struct CodegenOptions {
    pub backend: Backend,
    pub opt_level: OptLevel,
    pub keep_object: bool,
    pub lto: bool,
    pub coverage: bool,
    pub pgo_generate: bool,
    pub pgo_use: Option<PathBuf>,
    pub emit_wat_text: bool,
    pub cpu_isa: CpuIsaConfig,
    pub sve_vector_bits: Option<u32>,
    pub link_final_artifact: bool,
    pub cc1: Option<cc1::Cc1Options>,
    pub ffi_config: Option<FfiConfig>,
    pub ffi_packages: Vec<PathBuf>,
    pub runtime: Option<RuntimeMetadata>,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            backend: Backend::Llvm,
            opt_level: OptLevel::O2,
            keep_object: true,
            lto: false,
            coverage: false,
            pgo_generate: false,
            pgo_use: None,
            emit_wat_text: false,
            cpu_isa: CpuIsaConfig::default(),
            sve_vector_bits: None,
            link_final_artifact: true,
            cc1: None,
            ffi_config: None,
            ffi_packages: Vec::new(),
            runtime: None,
        }
    }
}

/// Result of running a backend.
#[derive(Debug)]
pub struct CodegenArtifact {
    pub textual_ir: String,
    pub object_path: PathBuf,
    pub artifact_path: PathBuf,
    pub library_pack: Option<PathBuf>,
    pub constant_folds: usize,
    pub inlined_functions: Vec<String>,
    pub metadata_path: Option<PathBuf>,
    pub metadata_telemetry: Option<metadata::MetadataTelemetry>,
    pub reflection_metadata_path: Option<PathBuf>,
}

/// Compile a Chic module using the selected backend and return paths to the emitted files.
///
/// # Errors
///
/// Returns [`Error::Codegen`] when the selected backend is unavailable, when the backend fails
/// during compilation, or when caching/linking stages encounter I/O or encoding failures.
#[expect(
    clippy::too_many_arguments,
    reason = "compile_module mirrors the driver CLI surface; grouping arguments would obscure intent"
)]
pub fn compile_module(
    source: &str,
    ast: &Module,
    mir: &MirModule,
    extern_mir: Option<&MirModule>,
    perf_metadata: &PerfMetadata,
    target: &Target,
    kind: ChicKind,
    output: &Path,
    options: &CodegenOptions,
    drop_glue: &[SynthesisedDropGlue],
    hash_glue: &[SynthesisedHashGlue],
    eq_glue: &[SynthesisedEqGlue],
    type_metadata: &[SynthesisedTypeMetadata],
) -> Result<CodegenArtifact, Error> {
    if !options.backend.is_available() {
        return Err(Error::Codegen(format!(
            "backend '{}' is temporarily unavailable; see task list for replacement",
            options.backend
        )));
    }

    let textual_ir = text::generate_text(ast, target, kind);
    let startup_descriptor_fingerprint = if matches!(options.backend, Backend::Llvm)
        && matches!(kind, ChicKind::Executable)
        && !mir.attributes.suppress_startup_descriptor
    {
        let source_mir = extern_mir.unwrap_or(mir);
        let mut hasher = blake3::Hasher::new();
        for func in &source_mir.functions {
            if matches!(func.kind, crate::mir::FunctionKind::Testcase) {
                hasher.update(func.name.as_bytes());
                hasher.update(&[0]);
                hasher.update(&[u8::from(func.is_async)]);
                hasher.update(&[0]);
            }
        }
        Some(hasher.finalize().to_hex().to_string())
    } else {
        None
    };
    let mut extra_hasher = blake3::Hasher::new();
    extra_hasher.update(b"chic.codegen.cache.extra.v1\0");
    if let Some(startup) = startup_descriptor_fingerprint.as_ref() {
        extra_hasher.update(b"startup\0");
        extra_hasher.update(startup.as_bytes());
        extra_hasher.update(&[0]);
    }
    extra_hasher.update(b"drop_glue\0");
    for entry in drop_glue {
        extra_hasher.update(&entry.type_identity.to_le_bytes());
        extra_hasher.update(entry.symbol.as_bytes());
        extra_hasher.update(&[0]);
        extra_hasher.update(entry.type_name.as_bytes());
        extra_hasher.update(&[0]);
    }
    extra_hasher.update(b"hash_glue\0");
    for entry in hash_glue {
        extra_hasher.update(&entry.type_identity.to_le_bytes());
        extra_hasher.update(entry.symbol.as_bytes());
        extra_hasher.update(&[0]);
        extra_hasher.update(entry.type_name.as_bytes());
        extra_hasher.update(&[0]);
    }
    extra_hasher.update(b"eq_glue\0");
    for entry in eq_glue {
        extra_hasher.update(&entry.type_identity.to_le_bytes());
        extra_hasher.update(entry.symbol.as_bytes());
        extra_hasher.update(&[0]);
        extra_hasher.update(entry.type_name.as_bytes());
        extra_hasher.update(&[0]);
    }
    extra_hasher.update(b"type_metadata\0");
    for entry in type_metadata {
        extra_hasher.update(&entry.type_identity.to_le_bytes());
        extra_hasher.update(&entry.size.to_le_bytes());
        extra_hasher.update(&entry.align.to_le_bytes());
        extra_hasher.update(&entry.flags.bits().to_le_bytes());
        for variance in &entry.variances {
            extra_hasher.update(&[variance.encode()]);
        }
        if let Some(symbol) = entry.drop_symbol.as_ref() {
            extra_hasher.update(symbol.as_bytes());
        }
        extra_hasher.update(&[0]);
        extra_hasher.update(entry.type_name.as_bytes());
        extra_hasher.update(&[0]);
    }
    let extra_fingerprint = Some(extra_hasher.finalize().to_hex().to_string());
    let fingerprint_inputs = cache::FingerprintInputs {
        source,
        textual_ir: &textual_ir,
        target,
        kind,
        backend: options.backend,
        options,
        extra_fingerprint: extra_fingerprint.as_deref(),
    };
    let key = cache::compute_fingerprint(&fingerprint_inputs);
    let disable_cache = env::var("CHIC_DISABLE_CODEGEN_CACHE").is_ok();

    if !disable_cache {
        if let Some(hit) = cache::try_load(output, &key) {
            return Ok(CodegenArtifact {
                textual_ir,
                object_path: hit.object_path,
                artifact_path: hit.artifact_path,
                metadata_path: hit.metadata_path,
                library_pack: hit.library_pack,
                constant_folds: hit.constant_folds,
                inlined_functions: hit.inlined_functions,
                metadata_telemetry: hit.metadata_telemetry,
                reflection_metadata_path: hit.reflection_path,
            });
        }
    }

    let mut artifact = match options.backend {
        Backend::Llvm => llvm::compile(
            ast,
            mir,
            extern_mir,
            perf_metadata,
            target,
            kind,
            output,
            options,
            drop_glue,
            hash_glue,
            eq_glue,
            type_metadata,
        ),
        Backend::Wasm => wasm::compile(
            ast,
            mir,
            perf_metadata,
            target,
            kind,
            output,
            options,
            drop_glue,
            hash_glue,
            eq_glue,
            type_metadata,
        ),
        Backend::Cc1 => cc1::compile(mir, target, kind, output, options),
    }?;

    artifact.textual_ir = textual_ir;

    if artifact.reflection_metadata_path.is_none() {
        let manifest = metadata::write_reflection_manifest(ast, output)?;
        artifact.reflection_metadata_path = Some(manifest);
    }

    if options.keep_object {
        cache::store(output, &key, &artifact)?;
    }

    Ok(artifact)
}

pub(crate) fn package_clrlib_archive(
    module: &Module,
    mir: &MirModule,
    target: &Target,
    kind: ChicKind,
    output: &Path,
    object_files: &[(&str, &Path)],
    metadata_files: &[(&str, &Path)],
    extra_files: &[(&str, FileRole, &Path)],
) -> Result<PathBuf, Error> {
    let triple = canonical_toolchain_triple(target);
    library::write_clrlib_archive(
        module,
        &mir.class_vtables,
        &triple,
        kind,
        output,
        object_files,
        metadata_files,
        extra_files,
    )
}

/// Compute the default object-file path adjacent to the output artifact.
pub(crate) fn default_object_path(output: &Path) -> PathBuf {
    let mut object_path = output.to_path_buf();
    if let Some(ext) = object_path.extension().and_then(|ext| ext.to_str())
        && ext == "o"
    {
        return object_path;
    }
    object_path.set_extension("o");
    object_path
}

/// Link an object file into the requested artifact form.
pub(crate) fn link_artifact(
    target: &Target,
    kind: ChicKind,
    objects: &[&Path],
    output: &Path,
    link: &NativeLinkOptions,
) -> Result<(), Error> {
    match kind {
        ChicKind::Executable => link_executable(target, objects, output, link),
        ChicKind::StaticLibrary => link_static_library(objects, output),
        ChicKind::DynamicLibrary => link_dynamic_library(target, objects, output, link),
    }
}

fn link_executable(
    target: &Target,
    objects: &[&Path],
    output: &Path,
    link: &NativeLinkOptions,
) -> Result<(), Error> {
    if std::env::var_os("CHIC_DEBUG_LINK").is_some() {
        eprintln!(
            "[chic-debug link] linking executable, weak_imports={}, search_paths={:?}, libraries={:?}",
            link.has_weak_imports, link.search_paths, link.libraries
        );
    }
    let mut cmd = default_linker_command(target);
    add_target_flags(&mut cmd, target);
    for object in objects {
        cmd.arg(object);
    }
    if should_link_native_runtime(link.runtime.as_ref()) {
        let runtime_path = runtime_archive_path(link.runtime.as_ref());
        let runtime_staticlib = ensure_runtime_staticlib(&runtime_path)?;
        cmd.arg(&runtime_staticlib);
    }
    append_native_link_options(&mut cmd, link, target);
    cmd.arg("-o").arg(output);
    append_default_libraries(&mut cmd, target, false);
    run_command(cmd, "link")
}

fn link_static_library(objects: &[&Path], output: &Path) -> Result<(), Error> {
    let mut cmd = Command::new("ar");
    cmd.arg("rcs").arg(output);
    for object in objects {
        cmd.arg(object);
    }
    run_command(cmd, "archive")?;

    if let Ok(status) = Command::new("ranlib").arg(output).status()
        && !status.success()
    {
        eprintln!("warning: ranlib exited with status {status}");
    }
    Ok(())
}

fn link_dynamic_library(
    target: &Target,
    objects: &[&Path],
    output: &Path,
    link: &NativeLinkOptions,
) -> Result<(), Error> {
    let mut cmd = default_linker_command(target);
    add_target_flags(&mut cmd, target);

    let triple = canonical_toolchain_triple(target);
    if triple.contains("apple") {
        cmd.arg("-dynamiclib");
        if let Some(name) = output.file_name() {
            cmd.arg("-install_name");
            let install = format!("@rpath/{}", name.to_string_lossy());
            cmd.arg(install);
        }
    } else {
        cmd.arg("-shared");
    }

    for object in objects {
        cmd.arg(object);
    }
    if should_link_native_runtime(link.runtime.as_ref()) {
        let runtime_path = runtime_archive_path(link.runtime.as_ref());
        let runtime_staticlib = ensure_runtime_staticlib(&runtime_path)?;
        cmd.arg(&runtime_staticlib);
    }
    append_native_link_options(&mut cmd, link, target);

    cmd.arg("-o").arg(output);
    append_default_libraries(&mut cmd, target, true);
    run_command(cmd, "link")
}

pub(crate) fn run_command(mut cmd: Command, action: &str) -> Result<(), Error> {
    if std::env::var_os("CHIC_DEBUG_LINK").is_some() {
        eprintln!("[chic-debug {action}] {:?}", cmd);
    }
    let status = cmd
        .status()
        .map_err(|err| Error::Codegen(format!("failed to spawn {action}: {err}")))?;
    if !status.success() {
        return Err(Error::Codegen(format!(
            "{action} command exited with status {status}"
        )));
    }
    Ok(())
}

fn default_linker_command(target: &Target) -> Command {
    if let Ok(linker) = env::var("CHIC_LINKER") {
        return Command::new(linker);
    }

    let triple = canonical_toolchain_triple(target);
    if triple.contains("apple") {
        Command::new("clang")
    } else if triple.contains("linux") {
        Command::new("gcc")
    } else {
        Command::new("cc")
    }
}

fn add_target_flags(cmd: &mut Command, target: &Target) {
    let triple = canonical_toolchain_triple(target);
    cmd.arg("-target").arg(triple);
}

fn append_default_libraries(cmd: &mut Command, target: &Target, include_threading: bool) {
    let triple = canonical_toolchain_triple(target);
    if triple.contains("apple") {
        cmd.arg("-lc");
        if include_threading {
            cmd.arg("-lSystem");
        }
    } else if triple.contains("linux") {
        cmd.arg("-lc");
        cmd.arg("-lm");
        cmd.arg("-ldl");
        if include_threading {
            cmd.arg("-lpthread");
        }
        cmd.arg("-Wl,--as-needed");
    } else {
        cmd.arg("-lc");
    }
}

fn append_native_link_options(cmd: &mut Command, link: &NativeLinkOptions, target: &Target) {
    for path in &link.search_paths {
        cmd.arg("-L").arg(path);
    }
    for lib in &link.libraries {
        cmd.arg(format!("-l{lib}"));
    }
    if link.has_weak_imports && canonical_toolchain_triple(target).contains("apple") {
        if std::env::var_os("CHIC_DEBUG_LINK").is_some() {
            eprintln!("[chic-debug link] adding -undefined dynamic_lookup for weak imports");
        }
        cmd.arg("-Wl,-undefined,dynamic_lookup");
    }
}
pub(crate) fn canonical_toolchain_triple(target: &Target) -> String {
    let triple = target.triple();
    if triple.ends_with("-unknown-none") {
        host_default_triple(target.arch())
    } else if triple.contains("-apple-macos") {
        // Normalise to the Darwin form understood by LLVM/target-lexicon.
        host_default_triple(target.arch())
    } else {
        triple.to_string()
    }
}

fn host_default_triple(arch: TargetArch) -> String {
    let os = std::env::consts::OS;
    match (arch, os) {
        (TargetArch::X86_64, "macos") => "x86_64-apple-darwin".into(),
        (TargetArch::Aarch64, "macos") => "arm64-apple-darwin".into(),
        (TargetArch::X86_64, "linux") => "x86_64-unknown-linux-gnu".into(),
        (TargetArch::Aarch64, "linux") => "aarch64-unknown-linux-gnu".into(),
        _ => match arch {
            TargetArch::X86_64 => "x86_64-unknown-linux-gnu".into(),
            TargetArch::Aarch64 => "aarch64-unknown-linux-gnu".into(),
        },
    }
}

fn ensure_runtime_staticlib(path: &Path) -> Result<PathBuf, Error> {
    if path.exists() {
        return Ok(path.to_path_buf());
    }
    Err(Error::Codegen(format!(
        "native runtime archive missing at {}; rebuild the native runtime with `chic build packages/runtime.native --backend llvm --crate-type staticlib --artifacts-path target/runtime/native --output target/runtime/native/<identity>/libchic_rt_native.a`",
        path.display()
    )))
}

fn runtime_archive_path(runtime: Option<&RuntimeMetadata>) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(runtime) = runtime {
        let kind = match runtime.kind {
            RuntimeKind::Native => "native",
            RuntimeKind::NoStd => "no_std",
        };
        return manifest_dir
            .join("target")
            .join("runtime")
            .join(kind)
            .join(&runtime.identity)
            .join("libchic_rt_native.a");
    }
    manifest_dir
        .join("target")
        .join("native")
        .join("libchic_rt_native.a")
}

fn should_link_native_runtime(runtime: Option<&RuntimeMetadata>) -> bool {
    let Some(runtime) = runtime else {
        return false;
    };
    if runtime.kind == RuntimeKind::NoStd {
        return false;
    }
    match std::env::var("CHIC_LINK_NATIVE_RUNTIME") {
        Ok(value) => value != "0",
        Err(_) => true,
    }
}

#[cfg(test)]
mod pipeline_smoke_tests;
