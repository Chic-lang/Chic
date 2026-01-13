use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::function::tests::helpers::{function_ir, test_target};
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::{LlvmFunctionSignature, build_signatures};
use crate::mir::{MirFunction, MirModule};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Emit the requested function and return its IR body or the formatted error message.
pub(super) fn emit_result(module: &MirModule, function_name: &str) -> Result<String, String> {
    let target = test_target();
    let (function, signature, signatures) = resolve_function(module, function_name, &target)?;
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        &signature,
        &signature.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &HashSet::new(),
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .map_err(|err| format!("{err}"))?;
    Ok(function_ir(&out, &signature.symbol).to_string())
}

fn resolve_function<'a>(
    module: &'a MirModule,
    function_name: &str,
    target: &crate::Target,
) -> Result<
    (
        &'a MirFunction,
        LlvmFunctionSignature,
        HashMap<String, LlvmFunctionSignature>,
    ),
    String,
> {
    let signatures = build_signatures(module, None, target).map_err(|err| format!("{err}"))?;
    let function = module
        .functions
        .iter()
        .find(|func| func.name == function_name)
        .ok_or_else(|| format!("function {function_name} not found in module"))?;
    let signature = signatures
        .get(&function.name)
        .cloned()
        .ok_or_else(|| format!("missing signature for {function_name}"))?;
    Ok((function, signature, signatures))
}
