pub mod accelerator;
mod emitter;
pub mod function;
pub mod gpu;
pub mod graphs;
pub mod intrinsics;
pub mod linalg;
pub mod memory;
pub mod numerics;
mod object;
mod signatures;
mod types;

use std::fs;
use std::path::{Path, PathBuf};

use crate::chic_kind::ChicKind;
use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::error::Error;
use crate::frontend::ast::Module as AstModule;
use crate::hash_glue::SynthesisedHashGlue;
use crate::mir::MirModule;
use crate::perf::PerfMetadata;
use crate::target::Target;
use crate::type_metadata::SynthesisedTypeMetadata;

use super::library::{FileRole, write_clrlib_archive};
use super::metadata;
use super::text::generate_text;
use super::{
    Backend, CodegenArtifact, CodegenOptions, canonical_toolchain_triple, default_object_path,
    link_artifact,
};
use std::env;

pub(crate) use emitter::emit_module;
use object::emit_object;
pub(crate) use signatures::{
    build_signatures, ensure_drop_glue_signatures, ensure_eq_glue_signatures,
    ensure_hash_glue_signatures, find_entry_function,
};

/// Compile the provided MIR module into LLVM IR and then native code using `clang`.
#[expect(
    clippy::too_many_arguments,
    reason = "Codegen entrypoint requires explicit AST, MIR, target, Chic build kind, output path, and options."
)]
pub fn compile(
    ast: &AstModule,
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
    ensure_backend(options)?;
    ensure_entry_exists(ast, mir, kind)?;

    let textual_ir = generate_text(ast, target, kind);
    let mut entry_name = find_entry_function(ast);
    if mir.attributes.suppress_startup_descriptor {
        entry_name = None;
    }
    let mut signatures = build_signatures(mir, entry_name.as_deref(), target)?;
    if let Some(global) = extern_mir {
        if !std::ptr::eq(global, mir) {
            let global_sigs = build_signatures(global, None, target)?;
            for (name, sig) in global_sigs {
                signatures.entry(name).or_insert(sig);
            }
        }
    }
    ensure_drop_glue_signatures(&mut signatures, drop_glue);
    ensure_hash_glue_signatures(&mut signatures, hash_glue);
    ensure_eq_glue_signatures(&mut signatures, eq_glue);
    let target_triple = canonical_toolchain_triple(target);
    let llvm_ir = emit_module(
        mir,
        extern_mir,
        perf_metadata,
        &signatures,
        entry_name.as_deref(),
        kind,
        &target_triple,
        target,
        options,
        drop_glue,
        hash_glue,
        eq_glue,
        type_metadata,
    )?;

    if let Ok(dir) = env::var("CHIC_DUMP_LLVM_DIR") {
        let mut dump_path = PathBuf::from(dir);
        if dump_path.is_dir() || dump_path.extension().is_none() {
            let stem = output
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            let entry = entry_name.as_deref().unwrap_or("module");
            let filename = format!("{stem}-{entry}.ll");
            dump_path.push(filename);
        }
        if let Some(parent) = dump_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::write(&dump_path, &llvm_ir) {
            Ok(_) => {
                eprintln!("[chic-debug] dumped LLVM IR to {}", dump_path.display());
            }
            Err(err) => {
                eprintln!(
                    "[chic-debug] failed to write LLVM dump to {}: {err}",
                    dump_path.display()
                );
            }
        }
    }

    let ir_path = write_ir_file(output, &llvm_ir)?;
    let object_path = default_object_path(output);
    emit_object(&ir_path, &object_path, target, options)?;

    let (metadata_path, metadata_telemetry) = metadata::write_metadata_object(
        ast,
        mir,
        perf_metadata,
        target,
        &target_triple,
        kind,
        options.runtime.as_ref(),
        output,
    )?;
    let reflection_path = metadata::write_reflection_manifest(ast, output)?;

    let mut library_pack = None;
    if options.link_final_artifact {
        let link_options = crate::codegen::collect_native_link_options(mir, options);
        link_artifacts(
            target,
            kind,
            &object_path,
            &metadata_path,
            output,
            &link_options,
        )?;

        if kind.is_library() {
            let archive_path = output.with_extension("clrlib");
            let object_entry = archive_entry_name("objects", &object_path);
            let metadata_entry = archive_entry_name("objects", &metadata_path);
            let reflection_entry = archive_entry_name("metadata", &reflection_path);
            let ir_entry = archive_entry_name("ir", &ir_path);
            let object_files = [(object_entry.as_str(), object_path.as_path())];
            let mut extras = vec![(ir_entry, FileRole::LlvmIr, ir_path.clone())];
            for package in &options.ffi_packages {
                extras.push((
                    archive_entry_name("ffi", package),
                    FileRole::Other,
                    package.clone(),
                ));
            }
            let extra_refs: Vec<(&str, FileRole, &Path)> = extras
                .iter()
                .map(|(name, role, path)| (name.as_str(), role.clone(), path.as_path()))
                .collect();
            let metadata_files = [
                (metadata_entry.as_str(), metadata_path.as_path()),
                (reflection_entry.as_str(), reflection_path.as_path()),
            ];
            let pack = write_clrlib_archive(
                ast,
                &mir.class_vtables,
                &target_triple,
                kind,
                &archive_path,
                &object_files,
                &metadata_files,
                &extra_refs,
            )?;
            library_pack = Some(pack);
        }
    }

    if !options.keep_object {
        let _ = fs::remove_file(&object_path);
        let _ = fs::remove_file(&metadata_path);
    }

    let artifact_path = if options.link_final_artifact {
        output.to_path_buf()
    } else {
        object_path.clone()
    };

    Ok(CodegenArtifact {
        textual_ir,
        object_path,
        artifact_path,
        library_pack,
        constant_folds: 0,
        inlined_functions: Vec::new(),
        metadata_path: Some(metadata_path),
        metadata_telemetry: Some(metadata_telemetry),
        reflection_metadata_path: Some(reflection_path),
    })
}

fn ensure_backend(options: &CodegenOptions) -> Result<(), Error> {
    if options.backend != Backend::Llvm {
        return Err(Error::Codegen(
            "LLVM backend invoked with non-LLVM options state".into(),
        ));
    }
    Ok(())
}

fn ensure_entry_exists(ast: &AstModule, mir: &MirModule, kind: ChicKind) -> Result<(), Error> {
    let _ = (ast, mir, kind);
    Ok(())
}

fn write_ir_file(output: &Path, llvm_ir: &str) -> Result<PathBuf, Error> {
    let ir_path = output.with_extension("ll");
    if let Some(parent) = ir_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(&ir_path, llvm_ir.as_bytes())?;
    Ok(ir_path)
}

fn link_artifacts(
    target: &Target,
    kind: ChicKind,
    object_path: &Path,
    metadata_path: &Path,
    output: &Path,
    link: &crate::codegen::NativeLinkOptions,
) -> Result<(), Error> {
    let link_inputs: Vec<&Path> = vec![object_path, metadata_path];
    link_artifact(target, kind, &link_inputs, output, link)?;
    Ok(())
}

fn archive_entry_name(prefix: &str, path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    format!("{prefix}/{name}")
}
