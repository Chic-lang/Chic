//! cc1-style compilation pipeline: translates MIR to preprocessed C and invokes `clang -cc1`.

mod runner;
mod translator;

use std::fs;
use std::path::{Path, PathBuf};

use crate::chic_kind::ChicKind;
use crate::codegen::{CodegenArtifact, CodegenOptions, OptLevel};
use crate::error::Error;
use crate::mir::MirModule;
use crate::target::Target;

use runner::run_cc1_stage;
use translator::{Cc1Error, Cc1Module};

/// Additional configuration for the cc1 backend.
#[derive(Debug, Clone, Default)]
pub struct Cc1Options {
    /// Extra flags forwarded to `clang -cc1`.
    pub extra_args: Vec<String>,
    /// When true, keep the generated `.i` file on disk instead of removing it after assembly.
    pub keep_preprocessed: bool,
}

pub fn compile(
    mir: &MirModule,
    target: &Target,
    kind: ChicKind,
    output: &Path,
    options: &CodegenOptions,
) -> Result<CodegenArtifact, Error> {
    let cc1 = options
        .cc1
        .as_ref()
        .ok_or_else(|| Error::Codegen("cc1 backend invoked without cc1 options".into()))?;

    let module = translate_module(mir, kind)?;
    let preprocessed_path = write_preprocessed(output, &module, cc1.keep_preprocessed)?;
    run_cc1_stage(
        &preprocessed_path,
        output,
        target,
        options.opt_level,
        &cc1.extra_args,
    )?;

    if !cc1.keep_preprocessed {
        let _ = fs::remove_file(&preprocessed_path);
    }

    Ok(CodegenArtifact {
        textual_ir: module.source.clone(),
        object_path: output.to_path_buf(),
        artifact_path: output.to_path_buf(),
        library_pack: None,
        constant_folds: 0,
        inlined_functions: Vec::new(),
        metadata_path: None,
        metadata_telemetry: None,
        reflection_metadata_path: None,
    })
}

pub fn translate_module(mir: &MirModule, kind: ChicKind) -> Result<Cc1Module, Error> {
    translator::translate(mir, kind).map_err(|err| Error::Codegen(err.to_string()))
}

pub fn run_preprocessed_stage(
    input: &Path,
    output: &Path,
    target: &Target,
    opt_level: OptLevel,
    extra_args: &[String],
) -> Result<(), Error> {
    run_cc1_stage(input, output, target, opt_level, extra_args)
}

fn write_preprocessed(output: &Path, module: &Cc1Module, keep: bool) -> Result<PathBuf, Error> {
    let preprocessed = output.with_extension("i");
    if let Some(parent) = preprocessed
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&preprocessed, module.source.as_bytes())?;
    if keep {
        Ok(preprocessed)
    } else {
        Ok(preprocessed)
    }
}

impl std::fmt::Display for Cc1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Cc1Error {}
