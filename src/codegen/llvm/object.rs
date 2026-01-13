use std::path::Path;
use std::process::Command;

use crate::codegen::{canonical_toolchain_triple, run_command};
use crate::error::Error;
use crate::target::Target;

use super::CodegenOptions;

pub(crate) fn emit_object(
    ir_path: &Path,
    object_path: &Path,
    target: &Target,
    options: &CodegenOptions,
) -> Result<(), Error> {
    let triple = canonical_toolchain_triple(target);
    let mut cmd = Command::new("clang");
    cmd.arg("-x")
        .arg("ir")
        .arg("-c")
        .arg(ir_path)
        .arg("-o")
        .arg(object_path)
        .arg(format!("-O{}", options.opt_level.as_flag()))
        .arg("-target")
        .arg(&triple)
        .arg("-Wno-override-module");

    if options.lto {
        cmd.arg("-flto");
    }
    if options.pgo_generate {
        cmd.arg("-fprofile-instr-generate");
    }
    if let Some(profile) = &options.pgo_use {
        cmd.arg(format!("-fprofile-instr-use={}", profile.display()));
    }

    run_command(cmd, "clang")
}
