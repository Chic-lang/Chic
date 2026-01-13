use std::collections::{BTreeSet, HashMap, HashSet};

use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::emit_function_with_async;
use crate::codegen::llvm::signatures::LlvmFunctionSignature;
use crate::error::Error;
use crate::mir::{
    Abi, ClassVTable, FunctionKind, MirFunction, StaticVar, StrId, TraitVTable, TypeLayoutTable,
};
use crate::target::{Target, TargetArch};
use std::fmt::Write;

use super::super::literals::StrLiteralInfo;
use super::super::metadata_pool::MetadataRegistry;

pub(crate) fn should_multiversion(function: &MirFunction) -> bool {
    matches!(
        function.kind,
        FunctionKind::Function | FunctionKind::Method | FunctionKind::Constructor
    ) && matches!(function.signature.abi, Abi::Chic)
        && !function.is_local()
        && !function.is_async
        && !function.is_generator
}

pub(crate) fn emit_multiversion_variants(
    out: &mut String,
    function: &MirFunction,
    sig: &LlvmFunctionSignature,
    signatures: &HashMap<String, LlvmFunctionSignature>,
    tracepoint: Option<&crate::perf::Tracepoint>,
    externals: &mut BTreeSet<&'static str>,
    vtable_symbols: &HashSet<String>,
    async_vtables: &HashMap<String, String>,
    trait_vtables: &[TraitVTable],
    class_vtables: &[ClassVTable],
    isa_tiers: &[CpuIsaTier],
    arch: TargetArch,
    target: &Target,
    statics: &[StaticVar],
    str_literals: &HashMap<StrId, StrLiteralInfo>,
    type_layouts: &TypeLayoutTable,
    metadata: &mut MetadataRegistry,
) -> Result<(), Error> {
    let mut variants = Vec::new();
    for tier in isa_tiers {
        let variant_symbol = format!("{}__{}", sig.symbol, tier.suffix());
        emit_function_with_async(
            out,
            function,
            sig,
            &variant_symbol,
            "internal dso_local",
            signatures,
            externals,
            vtable_symbols,
            async_vtables,
            trait_vtables,
            class_vtables,
            *tier,
            isa_tiers,
            arch,
            target,
            statics,
            str_literals,
            type_layouts,
            metadata,
            tracepoint,
        )?;
        writeln!(out).ok();
        variants.push((*tier, variant_symbol));
    }
    emit_dispatch_stub(out, function, sig, &variants)?;
    writeln!(out).ok();
    Ok(())
}

fn emit_dispatch_stub(
    out: &mut String,
    _function: &MirFunction,
    sig: &LlvmFunctionSignature,
    variants: &[(CpuIsaTier, String)],
) -> Result<(), Error> {
    if sig.variadic {
        return Err(Error::Codegen(
            "multiversion dispatch for variadic functions is not supported".into(),
        ));
    }
    if variants.is_empty() {
        return Err(Error::Codegen(
            "multiversion dispatch requested without variants".into(),
        ));
    }

    let ret_ty = sig.ret.clone().unwrap_or_else(|| "void".to_string());
    let params_repr = sig
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{ty} %arg{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let args_pass = sig
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| format!("{ty} %arg{index}"))
        .collect::<Vec<_>>()
        .join(", ");

    // Dispatch stubs can be emitted into multiple compilation units; use ODR linkage so the
    // linker can coalesce duplicates when building multi-module executables.
    //
    // NOTE: `linkonce_odr` stubs can be dropped by the toolchain if they are not referenced
    // within the same object file, which breaks cross-module calls (Mach-O in particular).
    // `weak_odr` keeps a definition available for external references while still allowing
    // duplicates to coalesce.
    let linkage = "weak_odr dso_local";
    writeln!(
        out,
        "define {linkage} {ret_ty} @{}({params_repr}) {{",
        sig.symbol
    )
    .ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  %tier = call i32 @chic_cpu_select_tier()").ok();

    let mut case_blocks = Vec::new();
    for (tier, _) in variants {
        case_blocks.push((tier.index(), format!("dispatch_{}", tier.suffix())));
    }
    let baseline_index = CpuIsaTier::Baseline.index();

    let default_block = case_blocks
        .iter()
        .find(|(index, _)| *index == baseline_index)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| case_blocks.first().map(|(_, label)| label.clone()).unwrap());

    let mut switch_line = format!("  switch i32 %tier, label %{default_block} [");
    for (index, label) in &case_blocks {
        if *index == baseline_index {
            continue;
        }
        write!(&mut switch_line, " i32 {index}, label %{label}").ok();
    }
    if !case_blocks.is_empty() {
        switch_line.push(' ');
    }
    switch_line.push(']');
    writeln!(out, "{switch_line}").ok();

    let mut result_values = Vec::new();
    for (tier, symbol) in variants {
        let block_label = format!("dispatch_{}", tier.suffix());
        writeln!(out, "{block_label}:").ok();
        if ret_ty == "void" {
            if args_pass.is_empty() {
                writeln!(out, "  call void @{symbol}()").ok();
            } else {
                writeln!(out, "  call void @{symbol}({args_pass})").ok();
            }
            writeln!(out, "  br label %dispatch_exit").ok();
        } else {
            let call_tmp = format!("%call_{}", tier.suffix());
            if args_pass.is_empty() {
                writeln!(out, "  {call_tmp} = call {ret_ty} @{symbol}()").ok();
            } else {
                writeln!(out, "  {call_tmp} = call {ret_ty} @{symbol}({args_pass})").ok();
            }
            writeln!(out, "  br label %dispatch_exit").ok();
            result_values.push((block_label.clone(), call_tmp));
        }
    }

    writeln!(out, "dispatch_exit:").ok();
    if ret_ty == "void" {
        writeln!(out, "  ret void").ok();
    } else {
        if result_values.is_empty() {
            return Err(Error::Codegen(
                "non-void dispatch requires at least one variant".into(),
            ));
        }
        let phi_entries = result_values
            .iter()
            .map(|(block, value)| format!("[ {value}, %{block} ]"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "  %dispatch_result = phi {ret_ty} {phi_entries}").ok();
        writeln!(out, "  ret {ret_ty} %dispatch_result").ok();
    }
    writeln!(out, "}}").ok();

    Ok(())
}
