mod c_abi;

pub use c_abi::{
    CAbiError, CAbiParam, CAbiPass, CAbiReturn, CAbiSignature, classify_c_abi_signature,
};

use crate::mir::{Abi, MirFunction, MirModule, ParamMode};
use crate::target::Target;
use std::collections::HashMap;

/// Canonicalise a function name for ABI maps (replace `.` with `::`).
fn canonical_function_name(repr: &str) -> String {
    repr.replace('.', "::")
}

/// Extract parameter modes for a lowered function.
pub fn param_modes_for_function(function: &MirFunction) -> Vec<ParamMode> {
    let mut modes = vec![ParamMode::Value; function.signature.params.len()];
    for local in &function.body.locals {
        if let crate::mir::LocalKind::Arg(index) = local.kind {
            if let Some(mode) = local.param_mode {
                if let Some(slot) = modes.get_mut(index) {
                    *slot = mode;
                }
            }
        }
    }
    modes
}

/// Classify every `@extern("C")` function in a module for the provided target.
pub fn classify_module_functions(
    mir: &MirModule,
    target: &Target,
) -> Result<HashMap<String, CAbiSignature>, CAbiError> {
    let mut out = HashMap::new();
    for function in &mir.functions {
        let Abi::Extern(name) = &function.signature.abi else {
            continue;
        };
        if !name.eq_ignore_ascii_case("c") {
            continue;
        }
        let modes = param_modes_for_function(function);
        let canonical = canonical_function_name(&function.name);
        let signature =
            classify_c_abi_signature(&function.signature, &modes, &mir.type_layouts, target)
                .map_err(|err| {
                    CAbiError::new(format!(
                        "C ABI classification failed for `{}`: {err}",
                        function.name
                    ))
                })?;
        out.insert(canonical, signature);
    }
    Ok(out)
}
