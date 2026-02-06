use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

use crate::chic_kind::ChicKind;
use crate::codegen::llvm::types::{const_repr, map_type_owned};
use crate::codegen::{CodegenOptions, CpuIsaTier};
use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::error::Error;
use crate::frontend::ast::{ExternBinding, Visibility};
use crate::hash_glue::SynthesisedHashGlue;
use crate::mir::{MirFunction, MirModule};
use crate::runtime::startup;
use crate::target::Target;
use crate::type_metadata::SynthesisedTypeMetadata;

use crate::codegen::llvm::signatures::{
    DynamicExternSignature, LlvmFunctionSignature, sanitise_symbol,
};

use super::context::LlvmEmitContext;
use super::dispatch::{
    emit_cpu_dispatch_helpers, emit_external_declarations, emit_multiversion_variants,
    should_multiversion,
};
use super::drop_table::emit_drop_table;
use super::eq_table::emit_eq_table;
use super::function::emit_function_with_async;
use super::hash_table::emit_hash_table;
use super::literals::{emit_string_literals, encode_llvm_bytes};
use super::metadata::emit_startup_metadata;
use super::metadata_pool::MetadataRegistry;
use super::type_metadata::emit_type_metadata_table;

fn sanitize_async_symbol(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_sep = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
            last_sep = false;
        } else if !last_sep {
            out.push('_');
            last_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn collect_async_vtable_symbols(
    mir: &MirModule,
    signatures: &HashMap<String, LlvmFunctionSignature>,
) -> Result<HashMap<String, String>, Error> {
    let mut symbols = HashMap::new();
    for plan in &mir.async_plans {
        let poll_sig = signatures.get(&plan.poll_fn).ok_or_else(|| {
            Error::Codegen(format!(
                "async poll stub `{}` missing LLVM signature",
                plan.poll_fn
            ))
        })?;
        let drop_sig = signatures.get(&plan.drop_fn).ok_or_else(|| {
            Error::Codegen(format!(
                "async drop stub `{}` missing LLVM signature",
                plan.drop_fn
            ))
        })?;
        let name = format!(
            "__async_vtable_{}",
            sanitize_async_symbol(&plan.function_name)
        );
        symbols.insert(plan.function_name.clone(), name);
        // Ensure referenced functions are also declared by touching the symbols.
        let _ = (&poll_sig.symbol, &drop_sig.symbol);
    }
    Ok(symbols)
}

pub(crate) fn emit_module(
    mir: &MirModule,
    global_mir: Option<&MirModule>,
    perf_metadata: &crate::perf::PerfMetadata,
    signatures: &HashMap<String, LlvmFunctionSignature>,
    entry: Option<&str>,
    kind: ChicKind,
    target_triple: &str,
    target: &Target,
    options: &CodegenOptions,
    drop_glue: &[SynthesisedDropGlue],
    hash_glue: &[SynthesisedHashGlue],
    eq_glue: &[SynthesisedEqGlue],
    type_metadata: &[SynthesisedTypeMetadata],
) -> Result<String, Error> {
    let async_vtables = collect_async_vtable_symbols(mir, signatures)?;
    let mut metadata = MetadataRegistry::new();
    let ctx = LlvmEmitContext::new(
        mir,
        global_mir,
        perf_metadata,
        signatures,
        async_vtables,
        entry,
        kind,
        target_triple,
        target,
        options,
        drop_glue,
        hash_glue,
        eq_glue,
        type_metadata,
    );
    let vtable_symbols: HashSet<String> = ctx
        .mir
        .trait_vtables
        .iter()
        .map(|table| table.symbol.clone())
        .chain(
            ctx.mir
                .class_vtables
                .iter()
                .map(|table| table.symbol.clone()),
        )
        .collect();

    if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
        eprintln!(
            "[chic-debug] llvm emission sees {} functions",
            ctx.mir.functions.len()
        );
        for function in &ctx.mir.functions {
            eprintln!(
                "[chic-debug] fn {} is_async={} async_machine={}",
                function.name,
                function.is_async,
                function.body.async_machine.is_some()
            );
        }
    }

    let mut out = String::new();
    writeln!(&mut out, "target triple = \"{target_triple}\"").ok();
    writeln!(&mut out).ok();

    let str_literals = emit_string_literals(&mut out, &ctx.mir.interned_strs);

    let mut externals = BTreeSet::new();
    externals.insert("chic_rt_object_new".into());
    externals.insert("chic_rt_panic".into());
    externals.insert("chic_rt_random_fill".into());
    externals.insert("chic_rt_arc_clone".into());
    externals.insert("chic_rt_arc_drop".into());
    externals.insert("chic_rt_hash_invoke".into());
    externals.insert("chic_rt_eq_invoke".into());
    if !ctx.mir.attributes.suppress_startup_descriptor && !ctx.kind.is_library() {
        externals.insert("chic_rt_startup_store_state".into());
        externals.insert("chic_rt_startup_has_run_tests_flag".into());
        externals.insert("chic_rt_startup_descriptor_snapshot".into());
        externals.insert("chic_rt_test_executor_run_all".into());
        if ctx.entry.is_some() {
            externals.insert("chic_rt_startup_call_entry".into());
            externals.insert("chic_rt_startup_call_entry_async".into());
            externals.insert("chic_rt_startup_complete_entry_async".into());
        }
    }
    let mut init_functions = Vec::new();
    let isa_tiers = ctx.isa_tiers();
    let arch = ctx.arch();
    let mut emitted_multiversion = false;
    let mut emitted_symbols: HashSet<String> = HashSet::new();

    emit_trait_vtables(&mut out, &ctx)?;
    emit_async_vtables(&mut out, &ctx)?;
    emit_class_vtables(&mut out, &ctx)?;
    if let Some(init) = emit_interface_defaults(&mut out, &ctx)? {
        externals.insert("chic_rt_install_interface_defaults".into());
        init_functions.push(init);
    }
    let forward_decls = emit_forward_declarations(&mut out, &ctx);
    emit_static_globals(&mut out, &ctx)?;
    emit_startup_descriptor_fallback(&mut out, &ctx);
    let dynamic_entries = collect_dynamic_ffi_entries(&ctx);
    let eager_descriptors = emit_dynamic_ffi_descriptors(&mut out, &dynamic_entries)?;
    if let Some(init) = emit_dynamic_ffi_config(&mut out, ctx.options().ffi_config.as_ref())? {
        externals.insert("chic_rt_ffi_add_search_path");
        externals.insert("chic_rt_ffi_set_default_pattern");
        init_functions.push(init);
    }
    let trace_index: HashMap<&str, &crate::perf::Tracepoint> = ctx
        .perf
        .tracepoints
        .iter()
        .map(|trace| (trace.function.as_str(), trace))
        .collect();

    for function in &ctx.mir.functions {
        let sig = ctx.signatures.get(&function.name).ok_or_else(|| {
            Error::Codegen(format!("missing LLVM signature for {}", function.name))
        })?;
        if emitted_symbols.contains(&sig.symbol) {
            if std::env::var("CHIC_DEBUG_DUP_SYMBOL").is_ok() {
                eprintln!(
                    "[chic-debug] skipping duplicate symbol emission for {} ({})",
                    function.name, sig.symbol
                );
            }
            continue;
        }

        if let Some(dynamic) = sig.dynamic.as_ref() {
            externals.insert("chic_rt_ffi_resolve");
            emit_dynamic_ffi_stub(&mut out, function, sig, dynamic, &mut externals)?;
            writeln!(&mut out).ok();
            continue;
        }

        if ctx.multiversion_enabled() && should_multiversion(function) {
            emit_multiversion_variants(
                &mut out,
                function,
                sig,
                ctx.signatures,
                trace_index.get(function.name.as_str()).copied(),
                &mut externals,
                &vtable_symbols,
                &ctx.async_vtables,
                ctx.mir.trait_vtables.as_slice(),
                ctx.mir.class_vtables.as_slice(),
                isa_tiers,
                arch,
                ctx.target,
                ctx.mir.statics.as_slice(),
                &str_literals,
                &ctx.mir.type_layouts,
                &mut metadata,
            )?;
            emitted_multiversion = true;
            emitted_symbols.insert(sig.symbol.clone());
        } else {
            let active_tier = isa_tiers.last().copied().unwrap_or(CpuIsaTier::Baseline);
            let linkage = if function.name.starts_with("__cl_drop__")
                || function.name.starts_with("__cl_clone__")
                || function.name.starts_with("__cl_hash__")
                || function.name.starts_with("__cl_eq__")
            {
                "linkonce_odr dso_local"
            } else if function.is_local() {
                "internal"
            } else {
                "dso_local"
            };
            emit_function_with_async(
                &mut out,
                function,
                sig,
                &sig.symbol,
                if function.is_weak {
                    "weak dso_local"
                } else {
                    linkage
                },
                ctx.signatures,
                &mut externals,
                &vtable_symbols,
                &ctx.async_vtables,
                ctx.mir.trait_vtables.as_slice(),
                ctx.mir.class_vtables.as_slice(),
                active_tier,
                isa_tiers,
                arch,
                ctx.target,
                ctx.mir.statics.as_slice(),
                &str_literals,
                &ctx.mir.type_layouts,
                &mut metadata,
                trace_index.get(function.name.as_str()).copied(),
            )?;
            emitted_symbols.insert(sig.symbol.clone());
            writeln!(&mut out).ok();
        }
    }

    if emitted_multiversion {
        emit_cpu_dispatch_helpers(
            &mut out,
            isa_tiers,
            &mut externals,
            arch,
            ctx.is_apple_target(),
            ctx.options().sve_vector_bits,
        );
    }

    if !eager_descriptors.is_empty() {
        let init = emit_dynamic_ffi_init(&mut out, &eager_descriptors);
        externals.insert("chic_rt_ffi_eager_resolve");
        init_functions.push(init);
    }

    if let Some(init) = emit_drop_table(&mut out, &ctx)? {
        externals.insert("chic_rt_install_drop_table".into());
        init_functions.push(init);
    }
    if let Some(init) = emit_hash_table(&mut out, &ctx)? {
        externals.insert("chic_rt_install_hash_table".into());
        init_functions.push(init);
    }
    if let Some(init) = emit_eq_table(&mut out, &ctx)? {
        externals.insert("chic_rt_install_eq_table".into());
        init_functions.push(init);
    }
    if let Some(init) = emit_type_metadata_table(&mut out, &ctx)? {
        externals.insert("chic_rt_install_type_metadata".into());
        init_functions.push(init);
    }
    if !ctx.kind.is_library() && !ctx.mir.attributes.suppress_startup_descriptor {
        let startup_mir = ctx.global_mir.unwrap_or(ctx.mir);
        emit_startup_metadata(&mut out, startup_mir, ctx.signatures, ctx.entry)?;
    }
    emit_native_entrypoint(&mut out, &ctx)?;

    if !init_functions.is_empty() {
        let count = init_functions.len();
        const CTOR_ENTRY_TYPE: &str = "{ i32, ptr, ptr }";
        writeln!(
            &mut out,
            "@llvm.global_ctors = appending global [{count} x {CTOR_ENTRY_TYPE}] ["
        )
        .ok();
        for (index, init) in init_functions.iter().enumerate() {
            let suffix = if index + 1 == count { "" } else { "," };
            writeln!(
                &mut out,
                "    {CTOR_ENTRY_TYPE} {{ i32 65535, ptr @{init}, ptr null }}{suffix}"
            )
            .ok();
        }
        writeln!(&mut out, "]").ok();
        writeln!(&mut out).ok();
    }

    // Avoid redeclaring runtime hooks that are already defined in this module (e.g. stdlib
    // overrides that export the same symbols). LLVM rejects duplicate declarations even when the
    // signatures match.
    let defined_symbols = collect_defined_symbols(&out);
    externals.retain(|name| {
        !emitted_symbols.contains(*name)
            && !defined_symbols.contains(*name)
            && !forward_decls.contains(*name)
    });
    if std::env::var("CHIC_DEBUG_EXTERNALS").is_ok() {
        eprintln!(
            "[chic-debug] externals captured ({}): {:?}",
            externals.len(),
            externals
        );
    }

    emit_external_declarations(&mut out, &externals);

    let debug_metadata = metadata.allocate_node("!{i32 2, !\"Debug Info Version\", i32 3}".into());
    let dwarf_metadata = metadata.allocate_node("!{i32 2, !\"Dwarf Version\", i32 5}".into());
    let ident_metadata = metadata.allocate_node("!{!\"Chic LLVM backend\"}".into());
    writeln!(
        &mut out,
        "!llvm.module.flags = !{{!{}, !{}}}",
        debug_metadata.0, dwarf_metadata.0
    )
    .ok();
    writeln!(&mut out, "!llvm.ident = !{{!{}}}", ident_metadata.0).ok();
    metadata.emit(&mut out);

    Ok(out)
}

fn emit_startup_descriptor_fallback(out: &mut String, ctx: &LlvmEmitContext<'_>) {
    let descriptor_symbol = startup::STARTUP_DESCRIPTOR_SYMBOL;
    let already_declared = ctx.mir.statics.iter().any(|var| {
        if let Some(alias) = var
            .extern_spec
            .as_ref()
            .and_then(|spec| spec.alias.as_ref())
        {
            alias == descriptor_symbol
        } else {
            var.qualified.ends_with(descriptor_symbol)
        }
    });
    if already_declared {
        return;
    }
    let defines_descriptor =
        !ctx.kind.is_library() && !ctx.mir.attributes.suppress_startup_descriptor;
    if defines_descriptor {
        return;
    }
    writeln!(
        out,
        "@{descriptor_symbol} = extern_weak global {{ i32, [4 x i8], {{ ptr, i32, i32 }}, {{ ptr, i64 }} }}, align 8"
    )
    .ok();
    writeln!(out).ok();
}

fn emit_forward_declarations(out: &mut String, ctx: &LlvmEmitContext<'_>) -> HashSet<String> {
    let locals: HashSet<&str> = ctx.mir.functions.iter().map(|f| f.name.as_str()).collect();
    let local_symbols: HashSet<&str> = ctx
        .mir
        .functions
        .iter()
        .filter_map(|function| ctx.signatures.get(function.name.as_str()))
        .map(|sig| sig.symbol.as_str())
        .collect();
    let mut declared_symbols: HashSet<String> = HashSet::new();
    let mut wrote_any = false;
    for (name, sig) in ctx.signatures {
        if locals.contains(name.as_str()) || local_symbols.contains(sig.symbol.as_str()) {
            continue;
        }
        if sig.dynamic.is_some() {
            continue;
        }
        if ctx.entry.is_some() && sig.symbol == "main" {
            continue;
        }
        if sig.symbol.starts_with("llvm.") {
            continue;
        }
        if declared_symbols.contains(sig.symbol.as_str()) {
            continue;
        }
        let ret = sig.ret.clone().unwrap_or_else(|| "void".to_string());
        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                let attrs = sig
                    .param_attrs
                    .get(index)
                    .filter(|attrs| !attrs.is_empty())
                    .map(|attrs| format!(" {}", attrs.join(" ")))
                    .unwrap_or_default();
                format!("{ty}{attrs}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let params = if sig.variadic {
            if params.is_empty() {
                "...".to_string()
            } else {
                format!("{params}, ...")
            }
        } else {
            params
        };
        let weak_prefix = if sig.weak { "extern_weak " } else { "" };
        writeln!(
            out,
            "declare {weak_prefix}{ret} @{}({})",
            sig.symbol, params
        )
        .ok();
        wrote_any = true;
        declared_symbols.insert(sig.symbol.clone());
    }
    if wrote_any {
        writeln!(out).ok();
    }
    declared_symbols
}

fn collect_defined_symbols(llvm_ir: &str) -> HashSet<String> {
    let mut symbols = HashSet::new();
    for line in llvm_ir.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("define ") {
            continue;
        }
        let Some(at_pos) = trimmed.find('@') else {
            continue;
        };
        let rest = &trimmed[at_pos + 1..];
        let Some(end) = rest.find('(') else {
            continue;
        };
        let name = rest[..end].trim();
        if !name.is_empty() {
            symbols.insert(name.to_string());
        }
    }
    symbols
}

fn emit_trait_vtables(out: &mut String, ctx: &LlvmEmitContext<'_>) -> Result<(), Error> {
    for table in &ctx.mir.trait_vtables {
        if table.slots.is_empty() {
            continue;
        }
        let mut entries = Vec::with_capacity(table.slots.len());
        for slot in &table.slots {
            let Some(signature) = ctx.signatures.get(&slot.symbol) else {
                continue;
            };
            entries.push(format!("ptr @{}", signature.symbol));
        }
        let slot_list = entries.join(", ");
        writeln!(
            out,
            "@{} = internal constant [{} x ptr] [{slot_list}]",
            table.symbol,
            entries.len()
        )
        .ok();
    }
    if !ctx.mir.trait_vtables.is_empty() {
        writeln!(out).ok();
    }
    Ok(())
}

fn emit_async_vtables(out: &mut String, ctx: &LlvmEmitContext<'_>) -> Result<(), Error> {
    for plan in &ctx.mir.async_plans {
        let Some(symbol) = ctx.async_vtable_symbol(&plan.function_name) else {
            continue;
        };
        let poll_sig = ctx.signatures.get(&plan.poll_fn).ok_or_else(|| {
            Error::Codegen(format!(
                "async poll stub `{}` missing LLVM signature",
                plan.poll_fn
            ))
        })?;
        let drop_sig = ctx.signatures.get(&plan.drop_fn).ok_or_else(|| {
            Error::Codegen(format!(
                "async drop stub `{}` missing LLVM signature",
                plan.drop_fn
            ))
        })?;
        writeln!(
            out,
            "@{sym} = internal constant {{ ptr, ptr }} {{ ptr @{poll}, ptr @{drop} }}",
            sym = symbol,
            poll = poll_sig.symbol,
            drop = drop_sig.symbol
        )
        .ok();
    }
    if !ctx.mir.async_plans.is_empty() {
        writeln!(out).ok();
    }
    Ok(())
}

fn emit_class_vtables(out: &mut String, ctx: &LlvmEmitContext<'_>) -> Result<(), Error> {
    for table in &ctx.mir.class_vtables {
        let mut entries = Vec::with_capacity(table.slots.len());
        for slot in &table.slots {
            let Some(signature) = ctx.signatures.get(&slot.symbol) else {
                continue;
            };
            entries.push(format!("ptr @{}", signature.symbol));
        }
        let slot_list = entries.join(", ");
        writeln!(
            out,
            "@{} = linkonce_odr constant [{} x ptr] [{slot_list}]",
            table.symbol,
            entries.len()
        )
        .ok();
    }
    if !ctx.mir.class_vtables.is_empty() {
        writeln!(out).ok();
    }
    Ok(())
}

const IFACE_DEFAULT_ENTRY: &str = "%__chx_iface_default";
const IFACE_DEFAULT_INIT: &str = "__chic_init_interface_defaults";

fn emit_interface_defaults(
    out: &mut String,
    ctx: &LlvmEmitContext<'_>,
) -> Result<Option<String>, Error> {
    if ctx.mir.interface_defaults.is_empty() {
        return Ok(None);
    }

    let mut pool: HashMap<String, String> = HashMap::new();
    let mut strings_in_order = Vec::new();

    let mut intern = |value: &str| -> String {
        if let Some(existing) = pool.get(value) {
            return existing.clone();
        }
        let symbol = format!("@__iface_default_str_{}", pool.len());
        pool.insert(value.to_string(), symbol.clone());
        strings_in_order.push((symbol.clone(), value.to_string()));
        symbol
    };

    let mut entries = Vec::new();
    for binding in &ctx.mir.interface_defaults {
        let implementer = intern(&binding.implementer);
        let interface = intern(&binding.interface);
        let method = intern(&binding.method);
        let signature = ctx.signatures.get(&binding.symbol).ok_or_else(|| {
            Error::Codegen(format!(
                "interface default `{}` missing LLVM signature",
                binding.symbol
            ))
        })?;
        entries.push(format!(
            "{{ ptr {implementer}, ptr {interface}, ptr {method}, ptr @{sym} }}",
            sym = signature.symbol
        ));
    }

    writeln!(out, "{IFACE_DEFAULT_ENTRY} = type {{ ptr, ptr, ptr, ptr }}").ok();

    for (symbol, value) in &strings_in_order {
        let array_len = value.len().max(1);
        if value.is_empty() {
            writeln!(
                out,
                "{symbol} = private unnamed_addr constant [{array_len} x i8] zeroinitializer"
            )
            .ok();
        } else {
            let encoded = encode_llvm_bytes(value.as_bytes());
            writeln!(
                out,
                "{symbol} = private unnamed_addr constant [{array_len} x i8] c\"{encoded}\""
            )
            .ok();
        }
    }
    writeln!(out).ok();

    let count = entries.len();
    writeln!(
        out,
        "@__chx_iface_defaults = private constant [{count} x {IFACE_DEFAULT_ENTRY}] ["
    )
    .ok();
    for (idx, entry) in entries.iter().enumerate() {
        let suffix = if idx + 1 == count { "" } else { "," };
        writeln!(out, "    {entry}{suffix}").ok();
    }
    writeln!(out, "]").ok();
    writeln!(out).ok();

    writeln!(out, "define internal void @{IFACE_DEFAULT_INIT}() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(
        out,
        "  call void @chic_rt_install_interface_defaults(ptr getelementptr inbounds ([{count} x {IFACE_DEFAULT_ENTRY}], ptr @__chx_iface_defaults, i32 0, i32 0), i64 {count})"
    )
    .ok();
    writeln!(out, "  ret void").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    Ok(Some(IFACE_DEFAULT_INIT.to_string()))
}

fn emit_static_globals(out: &mut String, ctx: &LlvmEmitContext<'_>) -> Result<(), Error> {
    if ctx.mir.statics.is_empty() {
        return Ok(());
    }
    for var in &ctx.mir.statics {
        let llvm_ty = map_type_owned(&var.ty, Some(&ctx.mir.type_layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "static `{}` lowers to void type in LLVM backend",
                var.qualified
            ))
        })?;
        let symbol_name = match var.extern_spec.as_ref().and_then(|spec| spec.alias.clone()) {
            Some(alias) => alias,
            None if var.is_extern => var
                .qualified
                .rsplit("::")
                .next()
                .map(sanitise_symbol)
                .unwrap_or_else(|| sanitise_symbol(&var.qualified)),
            None => sanitise_symbol(&var.qualified),
        };
        if var.is_extern
            && symbol_name == startup::STARTUP_DESCRIPTOR_SYMBOL
            && !ctx.kind.is_library()
            && !ctx.mir.attributes.suppress_startup_descriptor
        {
            // `emit_startup_metadata` injects a concrete descriptor definition for executables.
            // Avoid emitting an `extern_weak` declaration with the same symbol name in the same
            // LLVM module (LLVM IR rejects redeclarations + definitions).
            continue;
        }
        let symbol = format!("@{symbol_name}");
        let init = if let Some(value) = &var.initializer {
            const_repr(value, &llvm_ty)?
        } else {
            "zeroinitializer".into()
        };
        let linkage = if var.is_weak {
            "weak dso_local"
        } else {
            match var.visibility {
                Visibility::Public => "dso_local",
                _ => "internal",
            }
        };
        let import_linkage = if var.is_weak_import {
            "extern_weak"
        } else {
            "external"
        };
        let thread_local = if var.threadlocal { " thread_local" } else { "" };
        let align = ctx
            .mir
            .type_layouts
            .size_and_align_for_ty(&var.ty)
            .map(|(_, align)| align)
            .unwrap_or(1)
            .max(1);
        if var.is_extern && var.is_import {
            writeln!(
                out,
                "{symbol} = {import_linkage}{thread_local} global {llvm_ty}, align {align}"
            )
            .ok();
        } else {
            writeln!(
                out,
                "{symbol} = {linkage}{thread_local} global {llvm_ty} {init}, align {align}"
            )
            .ok();
        }
    }
    writeln!(out).ok();
    Ok(())
}

fn collect_dynamic_ffi_entries<'a>(
    ctx: &'a LlvmEmitContext<'a>,
) -> Vec<(
    &'a MirFunction,
    &'a LlvmFunctionSignature,
    &'a DynamicExternSignature,
)> {
    ctx.mir
        .functions
        .iter()
        .filter_map(|function| {
            let signature = ctx.signatures.get(&function.name)?;
            signature
                .dynamic
                .as_ref()
                .map(|dynamic| (function, signature, dynamic))
        })
        .collect()
}

fn emit_dynamic_ffi_descriptors(
    out: &mut String,
    entries: &[(
        &MirFunction,
        &LlvmFunctionSignature,
        &DynamicExternSignature,
    )],
) -> Result<Vec<String>, Error> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    writeln!(
        out,
        "%chic_ffi_descriptor = type {{ ptr, ptr, i32, i32, i1 }}"
    )
    .ok();
    writeln!(out).ok();
    let mut eager = Vec::new();
    for (_, _, dynamic) in entries {
        emit_c_string(out, &dynamic.library_const, &dynamic.library_value)?;
        emit_c_string(out, &dynamic.symbol_const, &dynamic.symbol_value)?;
        let binding = match dynamic.binding {
            ExternBinding::Eager => 2,
            ExternBinding::Lazy => 1,
            ExternBinding::Static => 0,
        };
        let optional = if dynamic.optional { "1" } else { "0" };
        let convention = ffi_convention_tag(&dynamic.convention);
        writeln!(
            out,
            "@{} = private constant %chic_ffi_descriptor {{ ptr @{}, ptr @{}, i32 {convention}, i32 {binding}, i1 {optional} }}",
            dynamic.descriptor_global, dynamic.library_const, dynamic.symbol_const
        )
        .ok();
        if matches!(dynamic.binding, ExternBinding::Eager) {
            eager.push(dynamic.descriptor_global.clone());
        }
    }
    writeln!(out).ok();
    Ok(eager)
}

fn emit_dynamic_ffi_stub(
    out: &mut String,
    _function: &MirFunction,
    signature: &LlvmFunctionSignature,
    dynamic: &DynamicExternSignature,
    externals: &mut BTreeSet<&'static str>,
) -> Result<(), Error> {
    if signature.variadic {
        return Err(Error::Codegen(
            "dynamic FFI stubs for variadic extern functions are not supported yet".into(),
        ));
    }
    externals.insert("chic_rt_ffi_resolve");
    let ret_ty = signature.ret.clone().unwrap_or_else(|| "void".to_string());
    let params_repr = signature
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let attrs = signature
                .param_attrs
                .get(index)
                .filter(|attrs| !attrs.is_empty())
                .map(|attrs| format!(" {}", attrs.join(" ")))
                .unwrap_or_default();
            format!("{ty}{attrs} %arg{index}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params_repr = if signature.variadic {
        if params_repr.is_empty() {
            "...".to_string()
        } else {
            format!("{params_repr}, ...")
        }
    } else {
        params_repr
    };
    writeln!(
        out,
        "define dso_local {ret_ty} @{}({params_repr}) {{",
        signature.symbol
    )
    .ok();
    writeln!(out, "entry:").ok();
    writeln!(
        out,
        "  %ffi_ptr = call ptr @chic_rt_ffi_resolve(ptr @{})",
        dynamic.descriptor_global
    )
    .ok();

    if dynamic.optional {
        writeln!(out, "  %ffi_missing = icmp eq ptr %ffi_ptr, null").ok();
        writeln!(
            out,
            "  br i1 %ffi_missing, label %ffi_optional, label %ffi_invoke"
        )
        .ok();
        writeln!(out, "ffi_optional:").ok();
        if ret_ty == "void" {
            writeln!(out, "  ret void").ok();
            writeln!(out, "ffi_invoke:").ok();
        } else {
            let default = default_ffi_literal(&ret_ty);
            writeln!(out, "  br label %ffi_exit_default").ok();
            writeln!(out, "ffi_invoke:").ok();
            emit_dynamic_ffi_call(out, signature, &ret_ty)?;
            writeln!(out, "  br label %ffi_exit_value").ok();
            writeln!(out, "ffi_exit_default:").ok();
            writeln!(out, "  ret {ret_ty} {default}").ok();
            writeln!(out, "ffi_exit_value:").ok();
            writeln!(out, "  ret {ret_ty} %ffi_result").ok();
            writeln!(out, "}}").ok();
            return Ok(());
        }
    } else {
        writeln!(out, "  br label %ffi_invoke").ok();
        writeln!(out, "ffi_invoke:").ok();
    }

    emit_dynamic_ffi_call(out, signature, &ret_ty)?;
    if ret_ty == "void" {
        writeln!(out, "  ret void").ok();
    } else {
        writeln!(out, "  ret {ret_ty} %ffi_result").ok();
    }
    writeln!(out, "}}").ok();
    Ok(())
}

fn emit_dynamic_ffi_call(
    out: &mut String,
    signature: &LlvmFunctionSignature,
    ret_ty: &str,
) -> Result<(), Error> {
    let params_repr = signature.params.join(", ");
    let params_repr = if signature.variadic {
        if params_repr.is_empty() {
            "...".to_string()
        } else {
            format!("{params_repr}, ...")
        }
    } else {
        params_repr
    };
    let fn_type = format!("{ret_ty} ({params_repr})*");
    writeln!(out, "  %ffi_callee = bitcast ptr %ffi_ptr to {fn_type}").ok();
    let call_args = signature
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let attrs = signature
                .param_attrs
                .get(index)
                .filter(|attrs| !attrs.is_empty())
                .map(|attrs| format!(" {}", attrs.join(" ")))
                .unwrap_or_default();
            format!("{ty}{attrs} %arg{index}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    if ret_ty == "void" {
        writeln!(out, "  call void %ffi_callee({call_args})").ok();
    } else {
        writeln!(
            out,
            "  %ffi_result = call {ret_ty} %ffi_callee({call_args})"
        )
        .ok();
    }
    Ok(())
}

fn emit_c_string(out: &mut String, name: &str, value: &str) -> Result<(), Error> {
    let mut escaped = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_graphic() && byte != b'\"' && byte != b'\\' {
            escaped.push(byte as char);
        } else {
            escaped.push_str(&format!("\\{:02X}", byte));
        }
    }
    escaped.push_str("\\00");
    let len = value.len() + 1;
    writeln!(
        out,
        "@{name} = private unnamed_addr constant [{len} x i8] c\"{escaped}\""
    )
    .ok();
    Ok(())
}

fn default_ffi_literal(ret_ty: &str) -> String {
    match ret_ty {
        "i1" | "i8" | "i16" | "i32" | "i64" | "i128" => "0".into(),
        "float" | "double" => "0.0".into(),
        _ if ret_ty.ends_with('*') || ret_ty == "ptr" => "null".into(),
        _ => "zeroinitializer".into(),
    }
}

fn emit_dynamic_ffi_init(out: &mut String, descriptors: &[String]) -> String {
    const INIT_SYMBOL: &str = "__chic_init_ffi";
    writeln!(out, "define internal void @{INIT_SYMBOL}() {{").ok();
    for descriptor in descriptors {
        writeln!(
            out,
            "  call ptr @chic_rt_ffi_eager_resolve(ptr @{descriptor})"
        )
        .ok();
    }
    writeln!(out, "  ret void").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();
    INIT_SYMBOL.to_string()
}

fn emit_dynamic_ffi_config(
    out: &mut String,
    config: Option<&crate::codegen::FfiConfig>,
) -> Result<Option<String>, Error> {
    let Some(config) = config else {
        return Ok(None);
    };
    if config.search_paths.is_empty() && config.default_pattern.is_none() {
        return Ok(None);
    }

    let mut search_names = Vec::new();
    for (idx, path) in config.search_paths.iter().enumerate() {
        let name = format!(".chic_ffi_search_{idx}");
        emit_c_string(out, &name, path)?;
        search_names.push(name);
    }
    let pattern_name = if let Some(pattern) = &config.default_pattern {
        let name = ".chic_ffi_default_pattern".to_string();
        emit_c_string(out, &name, pattern)?;
        Some(name)
    } else {
        None
    };
    const INIT_SYMBOL: &str = "__chic_init_ffi_config";
    writeln!(out, "define internal void @{INIT_SYMBOL}() {{").ok();
    for name in &search_names {
        writeln!(out, "  call void @chic_rt_ffi_add_search_path(ptr @{name})").ok();
    }
    if let Some(pattern) = &pattern_name {
        writeln!(
            out,
            "  call void @chic_rt_ffi_set_default_pattern(ptr @{pattern})"
        )
        .ok();
    }
    writeln!(out, "  ret void").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();
    Ok(Some(INIT_SYMBOL.to_string()))
}

fn ffi_convention_tag(convention: &str) -> u32 {
    match convention.to_ascii_lowercase().as_str() {
        "c" => 0,
        "system" => 1,
        "stdcall" => 2,
        "fastcall" => 3,
        "vectorcall" => 4,
        other => {
            let mut hash = 0u32;
            for byte in other.bytes() {
                hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
            }
            hash
        }
    }
}

fn emit_native_entrypoint(out: &mut String, ctx: &LlvmEmitContext<'_>) -> Result<(), Error> {
    if ctx.kind.is_library() {
        return Ok(());
    }
    if ctx.mir.attributes.suppress_startup_descriptor {
        return Ok(());
    }
    if ctx
        .mir
        .functions
        .iter()
        .filter(|function| function.extern_spec.is_none())
        .filter_map(|function| ctx.signatures.get(&function.name))
        .any(|sig| sig.symbol == "main")
    {
        // A user-defined or stdlib-provided main symbol is already present in this unit.
        return Ok(());
    }
    // When a custom startup stub is injected (e.g., async/stdlib overrides used in tests),
    // that stub provides the process `main` symbol. Avoid emitting another entrypoint to
    // prevent duplicate symbol failures while still allowing the descriptor to be linked.
    if std::env::var_os("CHIC_STARTUP_STDLIB_OVERRIDE").is_some() {
        return Ok(());
    }
    writeln!(
        out,
        "define dso_local i32 @main(i32 %argc, ptr %argv, ptr %envp) {{"
    )
    .ok();
    writeln!(out, "entry:").ok();
    if !ctx.type_metadata().is_empty() {
        writeln!(out, "  call void @__chic_init_type_metadata()").ok();
    }
    writeln!(
        out,
        "  call void @chic_rt_startup_store_state(i32 %argc, ptr %argv, ptr %envp)"
    )
    .ok();
    writeln!(
        out,
        "  %run_tests_raw = call i32 @chic_rt_startup_has_run_tests_flag()"
    )
    .ok();
    writeln!(out, "  %should_run_tests = icmp ne i32 %run_tests_raw, 0").ok();
    writeln!(
        out,
        "  br i1 %should_run_tests, label %run_tests, label %run_entry"
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "run_tests:").ok();
    writeln!(
        out,
        "  %test_exit = call i32 @chic_rt_test_executor_run_all()"
    )
    .ok();
    writeln!(out, "  ret i32 %test_exit").ok();
    writeln!(out).ok();

    writeln!(out, "run_entry:").ok();
    let Some(entry_name) = ctx.entry else {
        writeln!(out, "  ret i32 0").ok();
        writeln!(out, "}}").ok();
        writeln!(out).ok();
        return Ok(());
    };
    if ctx
        .mir
        .functions
        .iter()
        .all(|function| function.name != entry_name)
    {
        writeln!(out, "  ret i32 0").ok();
        writeln!(out, "}}").ok();
        writeln!(out).ok();
        return Ok(());
    }

    const STARTUP_DESCRIPTOR_SNAPSHOT_LLVM_TYPE: &str =
        "{ i32, [4 x i8], { i64, i32, i32 }, { i64, i64 } }";
    writeln!(
        out,
        "  %startup_desc = call {STARTUP_DESCRIPTOR_SNAPSHOT_LLVM_TYPE} @chic_rt_startup_descriptor_snapshot()"
    )
    .ok();
    writeln!(
        out,
        "  %entry_desc = extractvalue {STARTUP_DESCRIPTOR_SNAPSHOT_LLVM_TYPE} %startup_desc, 2"
    )
    .ok();
    writeln!(
        out,
        "  %entry_flags = extractvalue {{ i64, i32, i32 }} %entry_desc, 1"
    )
    .ok();
    writeln!(out, "  %entry_async_bit = and i32 %entry_flags, 1").ok();
    writeln!(out, "  %entry_is_async = icmp ne i32 %entry_async_bit, 0").ok();
    writeln!(
        out,
        "  br i1 %entry_is_async, label %entry_async, label %entry_sync"
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "entry_sync:").ok();
    writeln!(
        out,
        "  %exit_code = call i32 @chic_rt_startup_call_entry(ptr @__chic_program_main, i32 %entry_flags, i32 %argc, ptr %argv, ptr %envp)"
    )
    .ok();
    writeln!(out, "  ret i32 %exit_code").ok();
    writeln!(out).ok();

    writeln!(out, "entry_async:").ok();
    writeln!(
        out,
        "  %entry_task = call ptr @chic_rt_startup_call_entry_async(ptr @__chic_program_main, i32 %entry_flags, i32 %argc, ptr %argv, ptr %envp)"
    )
    .ok();
    writeln!(
        out,
        "  %exit_code_async = call i32 @chic_rt_startup_complete_entry_async(ptr %entry_task, i32 %entry_flags)"
    )
    .ok();
    writeln!(out, "  ret i32 %exit_code_async").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodegenOptions;
    use crate::codegen::llvm::signatures::LlvmFunctionSignature;
    use crate::drop_glue::{SynthesisedDropGlue, drop_type_identity};
    use crate::mir::{
        Abi, AsyncFrameMetrics, AsyncFramePolicy, AsyncLoweringArtifact, BasicBlock, ConstValue,
        FnSig, FunctionKind, InterfaceDefaultImpl, LocalDecl, LocalKind, MirBody, MirExternSpec,
        MirFunction, MirModule, StaticId, StaticVar, TraitVTable, Ty, VTableSlot,
    };
    use crate::perf::PerfMetadata;
    use crate::target::Target;
    use crate::type_metadata::TypeFlags;

    fn simple_function(name: &str) -> MirFunction {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        body.blocks.push(BasicBlock {
            id: body.entry(),
            statements: Vec::new(),
            terminator: Some(crate::mir::Terminator::Return),
            span: None,
        });
        MirFunction {
            name: name.into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: Vec::new(),
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),

                lends_to_return: None,

                variadic: false,
            },
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        }
    }

    fn async_artifact(name: &str) -> AsyncLoweringArtifact {
        AsyncLoweringArtifact {
            function_index: 0,
            function_name: name.into(),
            frame_type: format!("{name}::Frame"),
            context_local: None,
            policy: AsyncFramePolicy::default(),
            metrics: AsyncFrameMetrics::default(),
            frame_fields: Vec::new(),
            suspend_points: Vec::new(),
            resume_states: Vec::new(),
            state_count: 1,
            poll_fn: format!("{name}::poll"),
            drop_fn: format!("{name}::drop"),
            implicit_promotion: false,
            captured_arguments: Vec::new(),
            captured_locals: Vec::new(),
        }
    }

    #[test]
    fn emits_trait_vtable_globals() {
        let mut module = MirModule::default();
        module.trait_vtables.push(TraitVTable {
            symbol: "__vtable_Demo__Formatter__Demo__Widget".into(),
            trait_name: "Demo::Formatter".into(),
            impl_type: "Demo::Widget".into(),
            slots: vec![VTableSlot {
                method: "Format".into(),
                symbol: "Demo::Formatter::Format".into(),
            }],
        });
        module
            .functions
            .push(simple_function("Demo::Formatter::Format"));

        let mut signatures = HashMap::new();
        signatures.insert(
            "Demo::Formatter::Format".into(),
            LlvmFunctionSignature {
                symbol: "Demo__Formatter__Format".into(),
                ret: Some("void".into()),
                params: Vec::new(),
                param_attrs: Vec::new(),
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            },
        );

        let target = Target::host();
        let options = CodegenOptions::default();
        let perf = PerfMetadata::default();
        let ctx = LlvmEmitContext::new(
            &module,
            None,
            &perf,
            &signatures,
            HashMap::new(),
            None,
            ChicKind::StaticLibrary,
            target.triple(),
            &target,
            &options,
            &[],
            &[],
            &[],
            &[],
        );
        let mut out = String::new();
        emit_trait_vtables(&mut out, &ctx).expect("emit vtables");
        assert!(out.contains("@__vtable_Demo__Formatter__Demo__Widget"));
        assert!(out.contains("[1 x ptr] [ptr @Demo__Formatter__Format]"));
    }

    #[test]
    fn collects_async_vtables_for_trait_and_generic_functions() {
        let mut module = MirModule::default();
        module.async_plans = vec![
            async_artifact("Demo::Formatter::FormatAsync"),
            async_artifact("Demo::Generic<T>::RunAsync"),
        ];
        let mut signatures = HashMap::new();
        for name in [
            "Demo::Formatter::FormatAsync::poll",
            "Demo::Formatter::FormatAsync::drop",
            "Demo::Generic<T>::RunAsync::poll",
            "Demo::Generic<T>::RunAsync::drop",
        ] {
            signatures.insert(
                name.to_string(),
                LlvmFunctionSignature {
                    symbol: sanitize_async_symbol(name),
                    ret: Some("ptr".into()),
                    params: Vec::new(),
                    param_attrs: Vec::new(),
                    dynamic: None,
                    c_abi: None,
                    variadic: false,
                    weak: false,
                },
            );
        }

        let vtables =
            collect_async_vtable_symbols(&module, &signatures).expect("async vtable collection");
        assert_eq!(vtables.len(), 2);
        assert_eq!(
            vtables
                .get("Demo::Formatter::FormatAsync")
                .expect("formatter async entry"),
            "__async_vtable_Demo_Formatter_FormatAsync"
        );
        assert_eq!(
            vtables
                .get("Demo::Generic<T>::RunAsync")
                .expect("generic async entry"),
            "__async_vtable_Demo_Generic_T_RunAsync"
        );
    }

    #[test]
    fn emits_module_static_global() {
        let mut module = MirModule::default();
        module.statics.push(StaticVar {
            id: StaticId(0),
            qualified: "Demo::ANSWER".into(),
            owner: None,
            namespace: Some("Demo".into()),
            ty: Ty::named("int"),
            visibility: Visibility::Public,
            is_readonly: true,
            threadlocal: false,
            is_weak: false,
            is_extern: false,
            is_import: false,
            is_weak_import: false,
            link_library: None,
            extern_spec: None,
            span: None,
            initializer: Some(ConstValue::Int(7)),
        });

        let signatures = HashMap::new();
        let target = Target::host();
        let options = CodegenOptions::default();
        let perf = PerfMetadata::default();
        let ctx = LlvmEmitContext::new(
            &module,
            None,
            &perf,
            &signatures,
            HashMap::new(),
            None,
            ChicKind::StaticLibrary,
            target.triple(),
            &target,
            &options,
            &[],
            &[],
            &[],
            &[],
        );
        let mut out = String::new();
        emit_static_globals(&mut out, &ctx).expect("emit statics");
        assert!(
            out.contains("@Demo__ANSWER = dso_local global i32 7"),
            "expected global definition for module static, got:\\n{out}"
        );
    }

    #[test]
    fn emit_interface_defaults_emits_table() {
        let mut module = MirModule::default();
        module.interface_defaults.push(InterfaceDefaultImpl {
            implementer: "Demo::Widget".into(),
            interface: "Demo::IRenderable".into(),
            method: "Draw".into(),
            symbol: "Demo::IRenderable::Draw".into(),
        });
        module
            .functions
            .push(simple_function("Demo::IRenderable::Draw"));

        let mut signatures = HashMap::new();
        signatures.insert(
            "Demo::IRenderable::Draw".into(),
            LlvmFunctionSignature {
                symbol: "Demo__IRenderable__Draw".into(),
                ret: None,
                params: Vec::new(),
                param_attrs: Vec::new(),
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            },
        );

        let target = Target::host();
        let options = CodegenOptions::default();
        let perf = PerfMetadata::default();
        let ctx = LlvmEmitContext::new(
            &module,
            None,
            &perf,
            &signatures,
            HashMap::new(),
            None,
            ChicKind::StaticLibrary,
            target.triple(),
            &target,
            &options,
            &[],
            &[],
            &[],
            &[],
        );
        let mut out = String::new();
        let init = emit_interface_defaults(&mut out, &ctx).expect("emit interface defaults");
        assert_eq!(
            init,
            Some("__chic_init_interface_defaults".into()),
            "expected init function name to be returned"
        );
        assert!(
            out.contains("@__chx_iface_defaults"),
            "expected interface default table global"
        );
        assert!(
            out.contains("ptr @Demo__IRenderable__Draw"),
            "expected interface default entry to reference lowered function symbol"
        );
        assert!(
            out.contains("call void @chic_rt_install_interface_defaults"),
            "expected init function to install interface defaults via runtime hook"
        );
        assert!(
            out.contains("Demo::Widget"),
            "expected implementer name literal to be emitted"
        );
    }

    #[test]
    fn emit_module_includes_drop_table_metadata() {
        let mut module = MirModule::default();
        module.functions.push(simple_function("Demo::Main"));

        let mut signatures = HashMap::new();
        signatures.insert(
            "Demo::Main".into(),
            LlvmFunctionSignature {
                symbol: "Demo__Main".into(),
                ret: None,
                params: Vec::new(),
                param_attrs: Vec::new(),
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            },
        );

        let target = Target::host();
        let options = CodegenOptions::default();
        let perf = PerfMetadata::default();
        let drop_glue = [
            SynthesisedDropGlue {
                type_name: "Demo::Type".into(),
                symbol: "__cl_drop__Demo__Type".into(),
                function_index: 0,
                type_identity: drop_type_identity("Demo::Type"),
            },
            SynthesisedDropGlue {
                type_name: "Demo::Status".into(),
                symbol: "__cl_drop__Demo__Status".into(),
                function_index: 1,
                type_identity: drop_type_identity("Demo::Status"),
            },
        ];
        let type_metadata = [
            SynthesisedTypeMetadata {
                type_name: "Demo::Type".into(),
                type_identity: drop_type_identity("Demo::Type"),
                size: 24,
                align: 8,
                drop_symbol: Some("__cl_drop__Demo__Type".into()),
                variances: Vec::new(),
                flags: TypeFlags::empty(),
            },
            SynthesisedTypeMetadata {
                type_name: "Demo::Status".into(),
                type_identity: drop_type_identity("Demo::Status"),
                size: 8,
                align: 4,
                drop_symbol: Some("__cl_drop__Demo__Status".into()),
                variances: Vec::new(),
                flags: TypeFlags::empty(),
            },
        ];

        let ir = emit_module(
            &module,
            None,
            &perf,
            &signatures,
            None,
            ChicKind::StaticLibrary,
            target.triple(),
            &target,
            &options,
            &drop_glue,
            &[],
            &[],
            &type_metadata,
        )
        .expect("emit module");

        assert!(
            ir.contains("@__chic_drop_entries"),
            "drop entries array missing:\n{ir}"
        );
        assert!(
            ir.contains("__cl_drop__Demo__Type"),
            "drop glue symbol should appear in table:\n{ir}"
        );
        assert!(
            ir.contains("__cl_drop__Demo__Status"),
            "second drop glue symbol should appear in table:\n{ir}"
        );
        let type_id_demo_type =
            i64::from_ne_bytes(drop_type_identity("Demo::Type").to_ne_bytes()).to_string();
        assert!(
            ir.contains(&type_id_demo_type),
            "type identity for Demo::Type missing from drop table:\n{ir}"
        );
        let type_id_demo_status =
            i64::from_ne_bytes(drop_type_identity("Demo::Status").to_ne_bytes()).to_string();
        assert!(
            ir.contains(&type_id_demo_status),
            "type identity for Demo::Status missing from drop table:\n{ir}"
        );
        assert!(
            ir.contains("call void @chic_rt_install_drop_table"),
            "initializer should register drop table with runtime:\n{ir}"
        );
        assert!(
            ir.contains("@__chic_type_metadata"),
            "type metadata table should be emitted:\n{ir}"
        );
        assert!(
            ir.contains("call void @chic_rt_install_type_metadata"),
            "initializer should register type metadata with runtime:\n{ir}"
        );
        assert!(
            ir.contains("@llvm.global_ctors"),
            "global constructor registration missing:\n{ir}"
        );
    }

    #[test]
    fn emit_module_generates_dynamic_ffi_stub() {
        let mut module = MirModule::default();
        let mut function = simple_function("Interop::MessageBox");
        function.extern_spec = Some(MirExternSpec {
            convention: "system".into(),
            library: Some("user32".into()),
            alias: Some("MessageBoxW".into()),
            binding: ExternBinding::Lazy,
            optional: false,
            charset: None,
            weak: false,
        });
        module.functions.push(function);

        let mut signatures = HashMap::new();
        signatures.insert(
            "Interop::MessageBox".into(),
            LlvmFunctionSignature {
                symbol: "Interop__MessageBox".into(),
                ret: Some("i32".into()),
                params: vec!["ptr".into()],
                param_attrs: vec![Vec::new()],
                dynamic: Some(DynamicExternSignature {
                    descriptor_global: "__chic_ffi_desc_Interop__MessageBox".into(),
                    library_const: ".chic_ffi_lib_Interop__MessageBox".into(),
                    symbol_const: ".chic_ffi_sym_Interop__MessageBox".into(),
                    library_value: "user32".into(),
                    symbol_value: "MessageBoxW".into(),
                    convention: "system".into(),
                    binding: ExternBinding::Lazy,
                    optional: false,
                }),
                c_abi: None,
                variadic: false,
                weak: false,
            },
        );

        let target = Target::host();
        let options = CodegenOptions::default();
        let perf = PerfMetadata::default();
        let ir = emit_module(
            &module,
            None,
            &perf,
            &signatures,
            None,
            ChicKind::StaticLibrary,
            target.triple(),
            &target,
            &options,
            &[],
            &[],
            &[],
            &[],
        )
        .expect("emit module");
        assert!(
            ir.contains("@__chic_ffi_desc_Interop__MessageBox"),
            "descriptor global missing: {ir}"
        );
        assert!(
            ir.contains("call ptr @chic_rt_ffi_resolve"),
            "stub should invoke resolver: {ir}"
        );
    }
}
