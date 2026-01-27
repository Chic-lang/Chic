use std::collections::HashMap;

use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::error::Error;
use crate::frontend::ast::{ClassMember, ExternBinding, FunctionDecl, Item, Module as AstModule};
use crate::hash_glue::SynthesisedHashGlue;
use crate::mir::{
    AliasContract, FunctionKind, GenericArg, MirFunction, MirModule, ParamMode, Ty, TypeLayoutTable,
};
use crate::target::{Target, TargetOs, TargetRuntime};

use crate::abi::CAbiPass;
use crate::abi::{CAbiReturn, CAbiSignature, classify_module_functions, param_modes_for_function};

use super::types::map_type_owned;
use crate::runtime::startup;

#[derive(Clone)]
pub(crate) struct LlvmFunctionSignature {
    pub symbol: String,
    pub ret: Option<String>,
    pub params: Vec<String>,
    pub param_attrs: Vec<Vec<String>>,
    pub dynamic: Option<DynamicExternSignature>,
    pub c_abi: Option<CAbiSignature>,
    pub variadic: bool,
    pub weak: bool,
}

#[derive(Clone)]
pub(crate) struct DynamicExternSignature {
    pub descriptor_global: String,
    pub library_const: String,
    pub symbol_const: String,
    pub library_value: String,
    pub symbol_value: String,
    pub convention: String,
    pub binding: ExternBinding,
    pub optional: bool,
}

pub(crate) fn build_signatures(
    mir: &MirModule,
    entry: Option<&str>,
    target: &Target,
) -> Result<HashMap<String, LlvmFunctionSignature>, Error> {
    let debug_native_rt = std::env::var_os("CHIC_DEBUG_NATIVE_RT").is_some();
    let debug_async = std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok();
    let debug_types = std::env::var("CHIC_DEBUG_TYPES").is_ok();
    if debug_async {
        let mut task_keys: Vec<_> = mir
            .type_layouts
            .types
            .keys()
            .filter(|k| k.contains("Task"))
            .cloned()
            .collect();
        task_keys.sort();
        eprintln!("[chic-debug] layout keys containing Task: {:?}", task_keys);
    }
    let mut signatures = HashMap::new();
    let mut export_map = HashMap::new();
    for export in &mir.exports {
        export_map.insert(
            canonical_function_name(&export.function),
            export.symbol.clone(),
        );
    }
    if debug_native_rt {
        eprintln!(
            "[chic-debug native-rt] export entries = {}",
            export_map.len()
        );
    }
    let debug_signatures = std::env::var_os("CHIC_DEBUG_SIGNATURES").is_some();
    for function in &mir.functions {
        if matches!(target.runtime(), TargetRuntime::Wasm)
            && (function.is_weak
                || function
                    .extern_spec
                    .as_ref()
                    .map_or(false, |spec| spec.weak))
        {
            return Err(Error::Codegen(format!(
                "weak linkage is not supported by the WASM backend (`{}`)",
                function.name
            )));
        }
        if matches!(target.os(), TargetOs::Windows)
            && function
                .extern_spec
                .as_ref()
                .map_or(false, |spec| spec.weak)
        {
            return Err(Error::Codegen(format!(
                "`@weak_import` is not supported on COFF/Windows targets (`{}`)",
                function.name
            )));
        }
    }
    let c_abi_classifications = classify_module_functions(mir, target)
        .map_err(|err| Error::Codegen(format!("C ABI classification failed: {err}")))?;
    let canonical_entry = entry.map(canonical_function_name);
    for function in &mir.functions {
        if debug_native_rt {
            eprintln!(
                "[chic-debug native-rt] fn={} extern_spec={:?} kind={:?} blocks={}",
                function.name,
                function.extern_spec,
                function.kind,
                function.body.blocks.len()
            );
        }
        if std::env::var_os("CHIC_DEBUG_SIGNATURES").is_some() && function.name.contains("VecPtr") {
            let param_names = function
                .signature
                .params
                .iter()
                .map(Ty::canonical_name)
                .collect::<Vec<_>>();
            eprintln!(
                "[chic-debug sig] fn={} params={param_names:?} modes={:?}",
                function.name,
                param_modes_for_function(function)
            );
        }
        let canonical = canonical_function_name(&function.name);
        if debug_native_rt && function.name.contains("ThreadRuntimeExports") {
            let mapped = export_map.get(&canonical);
            eprintln!(
                "[chic-debug native-rt] export-lookup canonical={} mapped={:?}",
                canonical, mapped
            );
        }
        let mut symbol = entry_symbol(canonical_entry.as_deref(), &canonical);
        let simple_name = function
            .name
            .rsplit("::")
            .next()
            .unwrap_or(function.name.as_str());
        let simple_symbol = sanitise_symbol(simple_name);
        let mut dynamic_spec = None;
        if let Some(spec) = &function.extern_spec {
            if spec.library.is_some() {
                dynamic_spec = Some(spec.clone());
            }
            if let Some(alias) = &spec.alias {
                symbol = alias.clone();
            } else {
                symbol = simple_symbol.clone();
            }
        }
        if let Some(export_symbol) = export_map.get(&canonical) {
            symbol = export_symbol.clone();
        }
        // Thread runtime trampolines must keep their unmangled C symbols even if
        // package export metadata is trimmed by dependency analysis.
        if canonical == "Std::Platform::Thread::ThreadRuntimeExports::chic_thread_invoke" {
            symbol = "chic_thread_invoke".to_string();
        } else if canonical == "Std::Platform::Thread::ThreadRuntimeExports::chic_thread_drop" {
            symbol = "chic_thread_drop".to_string();
        }
        let ret_ty = if function.body.async_machine.is_some() {
            if let Some(result) = &function.async_result {
                Ty::named_generic("Std::Async::Task", vec![GenericArg::Type(result.clone())])
            } else {
                function.signature.ret.clone()
            }
        } else if matches!(function.kind, FunctionKind::Testcase)
            && matches!(function.signature.ret, Ty::Unit)
        {
            Ty::named("int")
        } else {
            function.signature.ret.clone()
        };
        if debug_types {
            eprintln!(
                "[chic-debug] map return type for {} => {}",
                function.name,
                ret_ty.canonical_name()
            );
        }
        let ret = match map_type_owned(&ret_ty, Some(&mir.type_layouts)) {
            Ok(mapped) => mapped.filter(|ty| *ty != "void"),
            Err(err) => {
                return Err(Error::Codegen(format!(
                    "failed to lower return type {} for {}: {err}",
                    ret_ty.canonical_name(),
                    function.name
                )));
            }
        };
        if debug_signatures && function.name.contains("InitializeDefault") {
            let param_names = function
                .signature
                .params
                .iter()
                .map(Ty::canonical_name)
                .collect::<Vec<_>>();
            let modes = param_modes_for_function(function);
            eprintln!(
                "[chic-debug sig] fn={} params={param_names:?} modes={modes:?} ret={}",
                function.name,
                ret_ty.canonical_name()
            );
        }
        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() && function.name.contains("Async") {
            eprintln!(
                "[chic-debug] signature ret map {} => {:?}",
                ret_ty.canonical_name(),
                ret
            );
        }
        let (params, param_attrs) = map_parameter_types(function, &mir.type_layouts)?;
        let c_abi = c_abi_classifications.get(&canonical).cloned();
        let (ret, params, param_attrs) = apply_c_abi_lowering(
            &ret_ty,
            ret,
            params,
            param_attrs,
            c_abi.as_ref(),
            &mir.type_layouts,
        )?;
        let mut entry = LlvmFunctionSignature {
            symbol,
            ret,
            params,
            param_attrs,
            dynamic: None,
            c_abi,
            variadic: function.signature.variadic,
            weak: false,
        };
        entry.weak = function
            .extern_spec
            .as_ref()
            .map(|spec| spec.weak)
            .unwrap_or(false);
        if let Some(spec) = dynamic_spec {
            let sanitized = sanitise_symbol(&canonical);
            let descriptor_global = format!("__chic_ffi_desc_{sanitized}");
            let library_const = format!(".chic_ffi_lib_{sanitized}");
            let symbol_const = format!(".chic_ffi_sym_{sanitized}");
            let alias_value = spec.alias.clone().unwrap_or_else(|| simple_symbol.clone());
            entry.dynamic = Some(DynamicExternSignature {
                descriptor_global,
                library_const,
                symbol_const,
                library_value: spec.library.clone().unwrap_or_default(),
                symbol_value: alias_value,
                convention: spec.convention.clone(),
                binding: spec.binding,
                optional: spec.optional,
            });
        }
        signatures.insert(canonical.clone(), entry);
    }
    insert_builtin_decimal_signatures(&mut signatures, &mir.type_layouts)?;
    insert_builtin_async_signatures(&mut signatures, &mir.type_layouts)?;
    insert_closure_runtime_signatures(&mut signatures, &mir.type_layouts)?;
    if std::env::var_os("CHIC_DEBUG_THREAD").is_some() {
        let mut thread_keys: Vec<_> = signatures
            .keys()
            .filter(|k| k.contains("Thread"))
            .cloned()
            .collect();
        thread_keys.sort();
        eprintln!("[chic-debug] thread-related signatures: {:?}", thread_keys);
    }
    if std::env::var_os("CHIC_DEBUG_ASYNC_READY").is_some() {
        let key = "Std::Async::RuntimeIntrinsics::chic_rt_async_task_header";
        eprintln!(
            "[chic-debug] signature presence ({key}): {} entries={} layouts={}",
            signatures.contains_key(key),
            signatures.len(),
            mir.type_layouts.types.len()
        );
    }
    Ok(signatures)
}

fn apply_c_abi_lowering(
    ret_mir_ty: &Ty,
    ret: Option<String>,
    mut params: Vec<String>,
    mut param_attrs: Vec<Vec<String>>,
    c_abi: Option<&CAbiSignature>,
    layouts: &TypeLayoutTable,
) -> Result<(Option<String>, Vec<String>, Vec<Vec<String>>), Error> {
    let Some(c_abi) = c_abi else {
        return Ok((ret, params, param_attrs));
    };

    for param in &c_abi.params {
        if let Some(coerce) = &param.coerce {
            if let Some(entry) = params.get_mut(param.index) {
                *entry = coerce.clone();
            }
        }
    }

    for param in &c_abi.params {
        let (is_byval, align) = match param.pass {
            CAbiPass::IndirectByVal { align } => (true, align),
            CAbiPass::IndirectPtr { align } => (false, align),
            CAbiPass::Direct => continue,
        };
        let Some(entry) = params.get_mut(param.index) else {
            return Err(Error::Codegen(format!(
                "C ABI byval classification requested for out-of-range parameter {}",
                param.index
            )));
        };
        let param_llvm_ty = map_type_owned(&param.ty, Some(layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "C ABI byval parameter type `{}` lowered to void LLVM type",
                param.ty.canonical_name()
            ))
        })?;
        *entry = "ptr".to_string();
        let Some(attr_list) = param_attrs.get_mut(param.index) else {
            return Err(Error::Codegen(format!(
                "C ABI byval classification requested for missing parameter attributes at index {}",
                param.index
            )));
        };
        if is_byval {
            attr_list.push(format!("byval({param_llvm_ty})"));
        }
        attr_list.push(format!("align {align}"));
    }

    let mut ret = ret;
    match &c_abi.ret {
        CAbiReturn::Direct { coerce, .. } => {
            if let Some(coerce) = coerce {
                ret = Some(coerce.clone());
            }
            Ok((ret, params, param_attrs))
        }
        CAbiReturn::IndirectSret { ty, align } => {
            let ret_llvm_ty = map_type_owned(ty, Some(layouts))?.ok_or_else(|| {
                Error::Codegen(format!(
                    "C ABI sret return type `{}` lowered to void LLVM type",
                    ty.canonical_name()
                ))
            })?;
            if matches!(ret_mir_ty, Ty::Unit) || ret.as_deref() == Some("void") {
                return Err(Error::Codegen(format!(
                    "C ABI sret classification requested for unit return type `{}`",
                    ret_mir_ty.canonical_name()
                )));
            }

            // LLVM models C ABI aggregate returns via a hidden first pointer parameter with
            // `sret(<ret_ty>)` and an explicit `align` attribute.
            params.insert(0, "ptr".to_string());
            param_attrs.insert(
                0,
                vec![format!("sret({ret_llvm_ty})"), format!("align {align}")],
            );
            Ok((None, params, param_attrs))
        }
    }
}

pub(crate) fn ensure_drop_glue_signatures(
    signatures: &mut HashMap<String, LlvmFunctionSignature>,
    drop_glue: &[SynthesisedDropGlue],
) {
    for glue in drop_glue {
        let canonical = canonical_function_name(&glue.symbol);
        signatures
            .entry(canonical)
            .or_insert_with(|| LlvmFunctionSignature {
                symbol: glue.symbol.clone(),
                ret: None,
                params: vec!["ptr".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            });
    }
}

pub(crate) fn ensure_hash_glue_signatures(
    signatures: &mut HashMap<String, LlvmFunctionSignature>,
    hash_glue: &[SynthesisedHashGlue],
) {
    for glue in hash_glue {
        let canonical = canonical_function_name(&glue.symbol);
        signatures
            .entry(canonical)
            .or_insert_with(|| LlvmFunctionSignature {
                symbol: glue.symbol.clone(),
                ret: Some("i64".to_string()),
                params: vec!["ptr".to_string()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            });
    }
}

pub(crate) fn ensure_eq_glue_signatures(
    signatures: &mut HashMap<String, LlvmFunctionSignature>,
    eq_glue: &[SynthesisedEqGlue],
) {
    for glue in eq_glue {
        let canonical = canonical_function_name(&glue.symbol);
        signatures
            .entry(canonical)
            .or_insert_with(|| LlvmFunctionSignature {
                symbol: glue.symbol.clone(),
                ret: Some("i32".to_string()),
                params: vec!["ptr".to_string(), "ptr".to_string()],
                param_attrs: vec![Vec::new(), Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            });
    }
}

pub(crate) fn resolve_function_name(
    signatures: &HashMap<String, LlvmFunctionSignature>,
    repr: &str,
) -> Option<String> {
    let canonical = canonical_function_name(repr);
    let canonical_base = canonical.split('#').next().unwrap_or(&canonical);

    if canonical.starts_with("chic_rt_") {
        let suffix = format!("::{canonical}");
        let mut candidates = signatures
            .keys()
            .filter(|key| {
                key == &&canonical
                    || key.ends_with(&suffix)
                    || key.split("::").last() == Some(canonical.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        if candidates.is_empty() {
            // Fall through to the default resolver for non-runtime aliases.
        } else if candidates.len() == 1 {
            return candidates.into_iter().next();
        } else if let Some(preferred) = candidates
            .iter()
            .find(|key| key.contains("::SpanIntrinsics::"))
        {
            return Some(preferred.clone());
        } else if let Some(preferred) = candidates
            .iter()
            .find(|key| key.contains("::VecIntrinsics::"))
        {
            return Some(preferred.clone());
        } else if let Some(preferred) = candidates.iter().find(|key| {
            key.contains("::HashMapIntrinsics::") || key.contains("::HashSetIntrinsics::")
        }) {
            return Some(preferred.clone());
        } else if let Some(preferred) = candidates
            .iter()
            .find(|key| key.contains("::StringIntrinsics::"))
        {
            return Some(preferred.clone());
        } else {
            return candidates.into_iter().next();
        }
    }
    if signatures.contains_key(&canonical) {
        return Some(canonical);
    }

    let suffix = format!("::{canonical}");
    for key in signatures.keys() {
        if key == &canonical
            || key.ends_with(&suffix)
            || key.split("::").last() == Some(canonical.as_str())
        {
            return Some(key.clone());
        }
    }
    let base = canonical.rsplit("::").next().unwrap_or(&canonical);
    let mut matches = signatures
        .keys()
        .filter(|key| key.rsplit("::").next() == Some(base))
        .cloned()
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    if matches.len() == 1 {
        return matches.into_iter().next();
    }

    if canonical.starts_with("__cl_drop__") {
        let normalise_drop = |repr: &str| {
            let tail = repr.trim_start_matches("__cl_drop__");
            let last = tail.rsplit("__").next().unwrap_or(tail);
            last.split('_').next().unwrap_or(last).to_string()
        };
        let target = normalise_drop(&canonical);
        let mut drop_matches = signatures
            .keys()
            .filter(|key| key.starts_with("__cl_drop__"))
            .filter(|key| normalise_drop(key) == target)
            .cloned()
            .collect::<Vec<_>>();
        drop_matches.sort();
        drop_matches.dedup();
        if drop_matches.len() == 1 {
            return drop_matches.into_iter().next();
        }
    }

    let mut trimmed_matches = signatures
        .keys()
        .filter_map(|key| {
            let key_base = key.split('#').next().unwrap_or(key);
            if key_base == canonical_base {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    trimmed_matches.sort();
    trimmed_matches.dedup();
    if trimmed_matches.len() == 1 {
        return trimmed_matches.into_iter().next();
    }
    None
}

pub(crate) fn find_entry_function(module: &AstModule) -> Option<String> {
    find_entry_in_items(&module.items, module.namespace.as_deref())
}

pub(crate) fn canonical_function_name(repr: &str) -> String {
    repr.replace('.', "::")
}

pub(crate) fn sanitise_symbol(name: &str) -> String {
    let clean = name.replace("::", "__");
    clean
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn entry_symbol(entry: Option<&str>, canonical: &str) -> String {
    if entry == Some(canonical) {
        startup::ENTRY_SYMBOL.to_string()
    } else {
        sanitise_symbol(canonical)
    }
}

fn map_parameter_types(
    function: &MirFunction,
    layouts: &TypeLayoutTable,
) -> Result<(Vec<String>, Vec<Vec<String>>), Error> {
    let debug_types = std::env::var("CHIC_DEBUG_TYPES").is_ok();
    if debug_types {
        let param_names = function
            .signature
            .params
            .iter()
            .map(|ty| ty.canonical_name())
            .collect::<Vec<_>>();
        eprintln!(
            "[chic-debug] map_parameter_types for {} params {:?}",
            function.name, param_names
        );
    }
    let mut params = Vec::with_capacity(function.signature.params.len());
    let mut attrs = Vec::with_capacity(function.signature.params.len());
    let modes = param_modes_for_function(function);
    let contracts = collect_param_contracts(function);
    for (index, ty) in function.signature.params.iter().enumerate() {
        let mode = modes.get(index).copied().unwrap_or(ParamMode::Value);
        let abi_ty = match map_type_owned(ty, Some(layouts)) {
            Ok(Some(mapped)) => mapped,
            Ok(None) => {
                return Err(Error::Codegen(
                    "parameter cannot have unit type in LLVM backend".into(),
                ));
            }
            Err(err) => {
                return Err(Error::Codegen(format!(
                    "failed to lower parameter {index} ({}) for {}: {err}",
                    ty.canonical_name(),
                    function.name
                )));
            }
        };
        let (param_ty, abi_pointer) = match mode {
            ParamMode::Value => (abi_ty, matches!(ty, Ty::Pointer(_))),
            ParamMode::In | ParamMode::Ref | ParamMode::Out => ("ptr".to_string(), true),
        };
        let contract = contracts
            .get(index)
            .copied()
            .unwrap_or_else(AliasContract::default);
        let attr_tokens = llvm_param_attributes(&contract, abi_pointer, mode);
        params.push(param_ty);
        attrs.push(attr_tokens);
    }
    Ok((params, attrs))
}

fn insert_builtin_decimal_signatures(
    signatures: &mut HashMap<String, LlvmFunctionSignature>,
    layouts: &TypeLayoutTable,
) -> Result<(), Error> {
    let runtime_call_ty = map_type_owned(
        &Ty::named("Std::Numeric::Decimal::DecimalRuntimeCall".to_string()),
        Some(layouts),
    )?
    .ok_or_else(|| Error::Codegen("DecimalRuntimeCall missing LLVM mapping".into()))?;
    let status_ty = map_type_owned(
        &Ty::named("Std::Numeric::Decimal::DecimalStatus".to_string()),
        Some(layouts),
    )?
    .ok_or_else(|| Error::Codegen("DecimalStatus missing LLVM mapping".into()))?;

    let usize_ty = map_type_owned(&Ty::named("usize"), Some(layouts))?
        .ok_or_else(|| Error::Codegen("usize missing LLVM mapping".into()))?;
    let uint_ty = map_type_owned(&Ty::named("uint"), Some(layouts))?
        .ok_or_else(|| Error::Codegen("uint missing LLVM mapping".into()))?;

    let decimal_ptr_ty = map_type_owned(
        &Ty::named("Std::Numeric::Decimal::DecimalConstPtr"),
        Some(layouts),
    )?
    .ok_or_else(|| Error::Codegen("DecimalConstPtr missing LLVM mapping".into()))?;
    let decimal_mut_ptr_ty = map_type_owned(
        &Ty::named("Std::Numeric::Decimal::DecimalMutPtr"),
        Some(layouts),
    )?
    .ok_or_else(|| Error::Codegen("DecimalMutPtr missing LLVM mapping".into()))?;
    let rounding_ty = map_type_owned(
        &Ty::named("Std::Numeric::Decimal::DecimalRoundingEncoding"),
        Some(layouts),
    )?
    .ok_or_else(|| Error::Codegen("DecimalRoundingEncoding missing LLVM mapping".into()))?;

    let mut register = |name: &str, symbol: &str, ret: String, params: Vec<String>| {
        let attrs = vec![Vec::new(); params.len()];
        signatures
            .entry(name.to_string())
            .or_insert_with(|| LlvmFunctionSignature {
                symbol: symbol.to_string(),
                ret: Some(ret),
                params,
                param_attrs: attrs,
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            });
    };

    // SIMD availability is handled via Std metadata; no Rust runtime probe.

    let base_params = vec![
        decimal_ptr_ty.clone(),
        usize_ty.clone(),
        rounding_ty.clone(),
        uint_ty.clone(),
    ];
    register(
        "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_sum",
        "chic_rt_decimal_sum",
        runtime_call_ty.clone(),
        base_params.clone(),
    );

    let dot_params = vec![
        decimal_ptr_ty.clone(),
        decimal_ptr_ty.clone(),
        usize_ty.clone(),
        rounding_ty.clone(),
        uint_ty.clone(),
    ];
    register(
        "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_dot",
        "chic_rt_decimal_dot",
        runtime_call_ty.clone(),
        dot_params.clone(),
    );

    let matmul_params = vec![
        decimal_ptr_ty.clone(),
        usize_ty.clone(),
        usize_ty.clone(),
        decimal_ptr_ty.clone(),
        usize_ty.clone(),
        decimal_mut_ptr_ty.clone(),
        rounding_ty.clone(),
        uint_ty.clone(),
    ];
    register(
        "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_matmul",
        "chic_rt_decimal_matmul",
        status_ty.clone(),
        matmul_params,
    );

    Ok(())
}

fn insert_builtin_async_signatures(
    signatures: &mut HashMap<String, LlvmFunctionSignature>,
    layouts: &TypeLayoutTable,
) -> Result<(), Error> {
    let debug = std::env::var_os("CHIC_DEBUG_ASYNC_READY").is_some();
    let has_cts = layouts
        .layout_for_name("Std::Async::CancellationTokenSource")
        .is_some();
    let has_ct = layouts
        .layout_for_name("Std::Async::CancellationToken")
        .is_some();
    if debug {
        eprintln!(
            "[chic-debug] async signature precheck: cts_layout={} ct_layout={} types={}",
            has_cts,
            has_ct,
            layouts.types.len()
        );
    }
    if !has_cts || !has_ct {
        return Ok(());
    }
    let uint_ty = map_type_owned(&Ty::named("uint"), Some(layouts))?
        .ok_or_else(|| Error::Codegen("uint missing LLVM mapping".into()))?;
    fn register(
        signatures: &mut HashMap<String, LlvmFunctionSignature>,
        name: &str,
        symbol: &str,
        ret: String,
        params: Vec<String>,
        overwrite: bool,
    ) {
        let attrs = vec![Vec::new(); params.len()];
        let sig = LlvmFunctionSignature {
            symbol: symbol.to_string(),
            ret: Some(ret),
            params,
            param_attrs: attrs,
            dynamic: None,
            c_abi: None,
            variadic: false,
            weak: false,
        };
        let entry = signatures.entry(name.to_string());
        if overwrite {
            entry.or_insert(sig);
        } else {
            entry.or_insert(sig);
        }
    }

    let bool_ty = map_type_owned(&Ty::named("bool"), Some(layouts))?
        .ok_or_else(|| Error::Codegen("bool missing LLVM mapping".into()))?;
    let token_source_ty = match map_type_owned(
        &Ty::named("Std::Async::CancellationTokenSource"),
        Some(layouts),
    )? {
        Some(ty) => ty,
        None => return Ok(()),
    };
    let token_ty = match map_type_owned(&Ty::named("Std::Async::CancellationToken"), Some(layouts))?
    {
        Some(ty) => ty,
        None => return Ok(()),
    };
    let int_ty = map_type_owned(&Ty::named("int"), Some(layouts))?
        .ok_or_else(|| Error::Codegen("int missing LLVM mapping".into()))?;
    if debug {
        eprintln!(
            "[chic-debug] registering async runtime intrinsics (uint={}, bool={}, cts={}, ct={})",
            uint_ty, bool_ty, token_source_ty, token_ty
        );
    }

    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_token_state",
        "chic_rt_async_token_state",
        uint_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_token_cancel",
        "chic_rt_async_token_cancel",
        uint_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_token_new",
        "chic_rt_async_token_new",
        "ptr".to_string(),
        Vec::new(),
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_spawn",
        "chic_rt_async_spawn",
        "void".to_string(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_spawn_local",
        "chic_rt_async_spawn_local",
        uint_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_scope",
        "chic_rt_async_scope",
        uint_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_block_on",
        "chic_rt_async_block_on",
        "void".to_string(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_cancel",
        "chic_rt_async_cancel",
        uint_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_task_header",
        "chic_rt_async_task_header",
        "ptr".to_string(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_task_header#1",
        "chic_rt_async_task_header",
        "ptr".to_string(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_task_header#2",
        "chic_rt_async_task_header",
        "ptr".to_string(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_task_bool_result",
        "chic_rt_async_task_bool_result",
        bool_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_task_int_result",
        "chic_rt_async_task_int_result",
        int_ty.clone(),
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_task_result",
        "chic_rt_async_task_result",
        uint_ty.clone(),
        vec!["ptr".to_string(), "ptr".to_string(), uint_ty.clone()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_await",
        "chic_rt_await",
        uint_ty.clone(),
        vec!["ptr".to_string(), "ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_yield",
        "chic_rt_yield",
        uint_ty,
        vec!["ptr".to_string()],
        false,
    );
    register(
        signatures,
        "Std::Async::RuntimeIntrinsics::chic_rt_async_register_future",
        "chic_rt_async_register_future",
        "void".to_string(),
        vec!["ptr".to_string()],
        false,
    );
    if debug {
        eprintln!(
            "[chic-debug] async intrinsic signatures inserted; task_header present? {}",
            signatures.contains_key("Std::Async::RuntimeIntrinsics::chic_rt_async_task_header")
        );
    }

    // CancellationTokenSource / CancellationToken methods are defined in the async override
    // but we always need their signatures for cross-unit calls.
    register(
        signatures,
        "Std::Async::CancellationTokenSource::Create",
        "Std__Async__CancellationTokenSource__Create",
        token_source_ty.clone(),
        Vec::new(),
        true,
    );
    register(
        signatures,
        "Std::Async::CancellationTokenSource::Token",
        "Std__Async__CancellationTokenSource__Token",
        token_ty.clone(),
        vec![token_source_ty.clone()],
        true,
    );
    register(
        signatures,
        "Std::Async::CancellationTokenSource::Cancel",
        "Std__Async__CancellationTokenSource__Cancel",
        "void".to_string(),
        vec![token_source_ty.clone()],
        true,
    );
    register(
        signatures,
        "Std::Async::CancellationTokenSource::get_IsCanceled",
        "Std__Async__CancellationTokenSource__get_IsCanceled",
        bool_ty.clone(),
        vec![token_source_ty],
        true,
    );
    register(
        signatures,
        "Std::Async::CancellationToken::IsCancellationRequested",
        "Std__Async__CancellationToken__IsCancellationRequested",
        bool_ty,
        vec![token_ty],
        true,
    );

    Ok(())
}

fn insert_closure_runtime_signatures(
    signatures: &mut HashMap<String, LlvmFunctionSignature>,
    layouts: &TypeLayoutTable,
) -> Result<(), Error> {
    fn register(
        signatures: &mut HashMap<String, LlvmFunctionSignature>,
        name: &str,
        symbol: &str,
        ret: String,
        params: Vec<String>,
    ) {
        let attrs = vec![Vec::new(); params.len()];
        signatures
            .entry(name.to_string())
            .or_insert(LlvmFunctionSignature {
                symbol: symbol.to_string(),
                ret: Some(ret),
                params,
                param_attrs: attrs,
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            });
    }

    let usize_ty = map_type_owned(&Ty::named("usize"), Some(layouts))?
        .ok_or_else(|| Error::Codegen("usize missing LLVM mapping".into()))?;
    register(
        signatures,
        "chic_rt_closure_env_alloc",
        "chic_rt_closure_env_alloc",
        "ptr".to_string(),
        vec![usize_ty.clone(), usize_ty.clone()],
    );
    register(
        signatures,
        "chic_rt_closure_env_clone",
        "chic_rt_closure_env_clone",
        "ptr".to_string(),
        vec!["ptr".to_string(), usize_ty.clone(), usize_ty.clone()],
    );
    register(
        signatures,
        "chic_rt_closure_env_free",
        "chic_rt_closure_env_free",
        "void".to_string(),
        vec!["ptr".to_string(), usize_ty.clone(), usize_ty],
    );

    Ok(())
}

fn collect_param_contracts(function: &MirFunction) -> Vec<AliasContract> {
    let mut contracts = vec![AliasContract::default(); function.signature.params.len()];
    for local in &function.body.locals {
        if let crate::mir::LocalKind::Arg(index) = local.kind {
            if let Some(slot) = contracts.get_mut(index) {
                *slot = local.aliasing;
            }
        }
    }
    contracts
}

fn llvm_param_attributes(
    contract: &AliasContract,
    abi_pointer: bool,
    _mode: ParamMode,
) -> Vec<String> {
    if !abi_pointer {
        return Vec::new();
    }
    let mut attrs = Vec::new();
    if contract.noalias {
        attrs.push("noalias".to_string());
    }
    if contract.nocapture {
        attrs.push("nocapture".to_string());
    }
    if contract.readonly && !contract.writeonly {
        attrs.push("readonly".to_string());
    }
    if contract.writeonly && !contract.readonly {
        attrs.push("writeonly".to_string());
    }
    if let Some(alignment) = contract.alignment {
        attrs.push(format!("align {alignment}"));
    }
    attrs
}

fn find_entry_in_items(items: &[Item], namespace: Option<&str>) -> Option<String> {
    for item in items {
        match item {
            Item::Function(func) if func.name == "Main" => {
                return Some(qualify(namespace, &func.name));
            }
            Item::Struct(def) => {
                let struct_ns = qualify(namespace, &def.name);
                let methods: Vec<_> = def.methods.iter().collect();
                if let Some(found) = find_entry_in_methods(&methods, Some(&struct_ns)) {
                    return Some(found);
                }
                if let Some(found) = find_entry_in_items(&def.nested_types, Some(&struct_ns)) {
                    return Some(found);
                }
            }
            Item::Class(def) => {
                let class_ns = qualify(namespace, &def.name);
                let methods: Vec<_> = def
                    .members
                    .iter()
                    .filter_map(|member| {
                        if let ClassMember::Method(method) = member {
                            Some(method)
                        } else {
                            None
                        }
                    })
                    .collect();
                if let Some(found) = find_entry_in_methods(&methods, Some(&class_ns)) {
                    return Some(found);
                }
                if let Some(found) = find_entry_in_items(&def.nested_types, Some(&class_ns)) {
                    return Some(found);
                }
            }
            Item::Namespace(ns) => {
                let nested = qualify(namespace, &ns.name);
                if let Some(found) = find_entry_in_items(&ns.items, Some(&nested)) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_entry_in_methods(methods: &[&FunctionDecl], namespace: Option<&str>) -> Option<String> {
    for method in methods {
        if method.name == "Main" {
            return Some(qualify(namespace, &method.name));
        }
    }
    None
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop_glue::SynthesisedDropGlue;
    use crate::drop_glue::drop_type_identity;
    use crate::frontend::ast::{
        ExternBinding, FunctionDecl, Item, MemberDispatch, Module as AstModule, NamespaceDecl,
        Signature, TypeExpr, Visibility,
    };
    use crate::mir::{
        Abi, AliasContract, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
        MirModule, ParamMode, PointerQualifiers, PointerTy, Ty,
    };

    #[test]
    fn canonicalises_function_names() {
        assert_eq!(canonical_function_name("Foo.Bar"), "Foo::Bar");
    }

    #[test]
    fn sanitises_symbols() {
        assert_eq!(sanitise_symbol("Foo::Bar"), "Foo__Bar");
        assert_eq!(sanitise_symbol("Foo::init#0"), "Foo__init_0");
    }

    #[test]
    fn finds_entry_function_recursively() {
        let function = FunctionDecl {
            visibility: Visibility::Public,
            name: "Main".into(),
            name_span: None,
            signature: Signature {
                parameters: Vec::new(),
                return_type: TypeExpr::simple("int"),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: None,
            is_async: false,
            is_constexpr: false,
            doc: None,
            modifiers: Vec::new(),
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: None,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        };
        let module = AstModule::with_items(
            Some("Root".into()),
            vec![Item::Namespace(NamespaceDecl {
                name: "Inner".into(),
                items: vec![Item::Function(function)],
                doc: None,
                attributes: Vec::new(),
                span: None,
            })],
        );
        assert_eq!(
            find_entry_function(&module),
            Some("Root::Inner::Main".to_string())
        );
    }

    #[test]
    fn builds_signatures_for_simple_module() {
        let target = Target::parse("x86_64-unknown-linux-gnu").expect("target");
        let mut module = MirModule::default();
        let body = MirBody::new(2, None);
        module.functions.push(MirFunction {
            name: "Root::Add".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![Ty::named("int"), Ty::named("int")],
                ret: Ty::named("int"),
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
        });
        let sigs = match build_signatures(&module, None, &target) {
            Ok(sigs) => sigs,
            Err(err) => panic!("build signatures failed: {err:?}"),
        };
        let Some(sig) = sigs.get("Root::Add") else {
            panic!("missing Root::Add signature");
        };
        assert_eq!(sig.symbol, "Root__Add");
        assert_eq!(sig.params, vec!["i32", "i32"]);
        assert_eq!(sig.param_attrs.len(), 2);
        assert!(sig.param_attrs.iter().all(|attrs| attrs.is_empty()));
        assert_eq!(sig.ret, Some("i32".into()));
    }

    #[test]
    fn build_signatures_use_extern_alias_symbol() {
        let target = Target::parse("x86_64-unknown-linux-gnu").expect("target");
        let mut module = MirModule::default();
        let body = MirBody::new(0, None);
        module.functions.push(MirFunction {
            name: "Interop::MessageBox".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![Ty::named("string")],
                ret: Ty::named("int"),
                abi: Abi::Extern("system".into()),
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
            extern_spec: Some(crate::mir::MirExternSpec {
                convention: "system".into(),
                library: Some("user32".into()),
                alias: Some("MessageBoxW".into()),
                binding: ExternBinding::Eager,
                optional: true,
                charset: Some("utf16".into()),
                weak: false,
            }),
            is_weak: false,
            is_weak_import: false,
        });

        let sigs = build_signatures(&module, None, &target).expect("signatures");
        let sig = sigs
            .get("Interop::MessageBox")
            .expect("interop signature present");
        assert_eq!(sig.symbol, "MessageBoxW");
    }

    #[test]
    fn ensure_drop_glue_signatures_adds_missing_entries() {
        let target = Target::parse("x86_64-unknown-linux-gnu").expect("target");
        let module = MirModule::default();
        let mut signatures = build_signatures(&module, None, &target).expect("signatures");
        let glue = SynthesisedDropGlue {
            type_name: "Demo::Type".into(),
            symbol: "__cl_drop__Demo__Type".into(),
            function_index: 0,
            type_identity: drop_type_identity("Demo::Type"),
        };
        ensure_drop_glue_signatures(&mut signatures, &[glue]);
        let canonical = canonical_function_name("__cl_drop__Demo__Type");
        let entry = signatures
            .get(&canonical)
            .expect("drop glue signature should be registered");
        assert_eq!(entry.symbol, "__cl_drop__Demo__Type");
        assert!(entry.ret.is_none());
        assert_eq!(entry.params, vec!["ptr".to_string()]);
        assert_eq!(entry.param_attrs, vec![Vec::<String>::new()]);
    }

    #[test]
    fn emits_aliasing_metadata_for_parameters() {
        let target = Target::parse("x86_64-unknown-linux-gnu").expect("target");
        let mut module = MirModule::default();
        let mut body = MirBody::new(2, None);
        body.locals.push(
            LocalDecl::new(
                Some("readOnly".into()),
                Ty::named("int"),
                false,
                None,
                LocalKind::Arg(0),
            )
            .with_param_mode(ParamMode::In)
            .with_alias_contract(AliasContract {
                noalias: true,
                nocapture: true,
                readonly: true,
                ..AliasContract::default()
            }),
        );
        let pointer_ty = Ty::Pointer(Box::new(PointerTy::with_qualifiers(
            Ty::named("byte"),
            true,
            PointerQualifiers {
                restrict: true,
                noalias: true,
                readonly: false,
                expose_address: true,
                alignment: Some(32),
            },
        )));
        body.locals.push(
            LocalDecl::new(
                Some("dest".into()),
                pointer_ty.clone(),
                true,
                None,
                LocalKind::Arg(1),
            )
            .with_param_mode(ParamMode::Value)
            .with_alias_contract(AliasContract {
                restrict: true,
                noalias: true,
                alignment: Some(32),
                expose_address: true,
                ..AliasContract::default()
            }),
        );
        module.functions.push(MirFunction {
            name: "Sample::Process".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![Ty::named("int"), pointer_ty],
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
        });

        let signatures = build_signatures(&module, None, &target).expect("signatures");
        let sig = signatures
            .get("Sample::Process")
            .expect("missing signature");
        assert_eq!(sig.param_attrs.len(), 2);
        assert_eq!(
            sig.param_attrs[0],
            vec![
                String::from("noalias"),
                String::from("nocapture"),
                String::from("readonly")
            ]
        );
        assert_eq!(
            sig.param_attrs[1],
            vec![String::from("noalias"), String::from("align 32")]
        );
    }
}
