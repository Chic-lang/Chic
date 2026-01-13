use std::path::Path;
use std::process::Command;

use crate::codegen::canonical_toolchain_triple;
use crate::codegen::{OptLevel, run_command};
use crate::error::Error;
use crate::target::Target;

pub(crate) fn run_cc1_stage(
    input: &Path,
    output: &Path,
    target: &Target,
    opt_level: OptLevel,
    extra_args: &[String],
) -> Result<(), Error> {
    let triple = canonical_toolchain_triple(target);
    let mut cmd = Command::new("clang");
    cmd.arg("-cc1")
        .arg("-triple")
        .arg(&triple)
        .arg(format!("-O{}", opt_level.as_flag()))
        .arg("-S")
        .arg("-o")
        .arg(output)
        .arg("-x")
        .arg("c")
        .arg(input);

    for arg in extra_args {
        cmd.arg(arg);
    }

    run_command(cmd, "clang -cc1")
}
