use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;
#[cfg(test)]
use std::sync::OnceLock;

use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::{
    BasicBlock, BlockId, ClassVTable, LocalId, LocalKind, MirFunction, Place, ProjectionElem,
    RoundingMode, StaticId, StaticVar, StrId, TraitVTable, Ty, TypeLayoutTable,
};
use crate::perf::Tracepoint;
use crate::target::Target;
use crate::target::TargetArch;

use super::super::metadata_pool::MetadataRegistry;
use crate::abi::{CAbiPass, CAbiReturn};
use crate::codegen::llvm::signatures::{LlvmFunctionSignature, sanitise_symbol};

use super::super::literals::StrLiteralInfo;

#[cfg(test)]
pub(crate) fn emit_function<'a>(
    out: &mut String,
    function: &MirFunction,
    sig: &LlvmFunctionSignature,
    symbol: &str,
    linkage: &str,
    signatures: &HashMap<String, LlvmFunctionSignature>,
    externals: &mut BTreeSet<&'static str>,
    vtable_symbols: &HashSet<String>,
    trait_vtables: &'a [TraitVTable],
    class_vtables: &'a [ClassVTable],
    tier: CpuIsaTier,
    available_tiers: &[CpuIsaTier],
    arch: TargetArch,
    target: &'a Target,
    statics: &'a [StaticVar],
    str_literals: &HashMap<StrId, StrLiteralInfo>,
    type_layouts: &TypeLayoutTable,
    metadata: &mut MetadataRegistry,
    tracepoint: Option<&'a Tracepoint>,
) -> Result<(), Error> {
    static EMPTY_ASYNC: OnceLock<HashMap<String, String>> = OnceLock::new();
    let empty = EMPTY_ASYNC.get_or_init(HashMap::new);
    emit_function_with_async(
        out,
        function,
        sig,
        symbol,
        linkage,
        signatures,
        externals,
        vtable_symbols,
        empty,
        trait_vtables,
        class_vtables,
        tier,
        available_tiers,
        arch,
        target,
        statics,
        str_literals,
        type_layouts,
        metadata,
        tracepoint,
    )
}

pub(crate) fn emit_function_with_async<'a>(
    out: &mut String,
    function: &MirFunction,
    sig: &LlvmFunctionSignature,
    symbol: &str,
    linkage: &str,
    signatures: &HashMap<String, LlvmFunctionSignature>,
    externals: &mut BTreeSet<&'static str>,
    vtable_symbols: &HashSet<String>,
    async_vtables: &HashMap<String, String>,
    trait_vtables: &'a [TraitVTable],
    class_vtables: &'a [ClassVTable],
    tier: CpuIsaTier,
    available_tiers: &[CpuIsaTier],
    arch: TargetArch,
    target: &'a Target,
    statics: &'a [StaticVar],
    str_literals: &HashMap<StrId, StrLiteralInfo>,
    type_layouts: &TypeLayoutTable,
    metadata: &mut MetadataRegistry,
    tracepoint: Option<&'a Tracepoint>,
) -> Result<(), Error> {
    // Force a read of the ABI classification metadata so the shared classifier wiring stays live
    // while the actual C ABI lowering is implemented in subsequent stages.
    let _ = sig.c_abi.as_ref();
    if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
        eprintln!(
            "[chic-debug] emit_function {} async={} machine={} async_result={:?}",
            function.name,
            function.is_async,
            function.body.async_machine.is_some(),
            function.async_result
        );
    }
    let ret_ty = sig.ret.clone();
    let ret_str = ret_ty.clone().unwrap_or_else(|| "void".to_string());

    let params_repr = sig
        .params
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let attrs = sig
                .param_attrs
                .get(index)
                .filter(|attrs| !attrs.is_empty())
                .map(|attrs| attrs.join(" "));
            match attrs {
                Some(attr) => format!("{ty} {attr} %arg{index}"),
                None => format!("{ty} %arg{index}"),
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params_repr = if sig.variadic {
        if params_repr.is_empty() {
            "...".to_string()
        } else {
            format!("{params_repr}, ...")
        }
    } else {
        params_repr
    };

    let mut fn_attrs = Vec::new();
    let hints = &function.optimization_hints;
    if hints.hot {
        fn_attrs.push("hot");
    }
    if hints.cold {
        fn_attrs.push("cold");
    }
    if hints.always_inline {
        fn_attrs.push("alwaysinline");
    }
    if hints.never_inline {
        fn_attrs.push("noinline");
    }
    let fn_attr_suffix = if fn_attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", fn_attrs.join(" "))
    };

    writeln!(
        out,
        "define {linkage} {ret_str} @{symbol}({params_repr}){fn_attr_suffix} {{"
    )
    .ok();

    let mut emitter = FunctionEmitter::new_with_async(
        function,
        signatures,
        externals,
        vtable_symbols,
        async_vtables,
        trait_vtables,
        class_vtables,
        tier,
        available_tiers,
        arch,
        target,
        statics,
        str_literals,
        type_layouts,
        metadata,
        tracepoint,
    );
    let body_ir = emitter.emit()?;
    out.push_str(&body_ir);

    writeln!(out, "}}").ok();
    Ok(())
}

#[cfg(test)]
impl<'a> FunctionEmitter<'a> {
    pub(crate) fn set_local_types_for_tests(&mut self, types: Vec<Option<String>>) {
        self.local_tys = types;
    }

    pub(crate) fn ir(&self) -> &str {
        &self.builder
    }
}

pub(super) struct FunctionEmitter<'a> {
    pub(super) function: &'a MirFunction,
    pub(super) signatures: &'a HashMap<String, LlvmFunctionSignature>,
    pub(super) externals: &'a mut BTreeSet<&'static str>,
    pub(super) vtable_symbols: &'a HashSet<String>,
    pub(super) async_vtables: &'a HashMap<String, String>,
    pub(super) trait_vtables: &'a [TraitVTable],
    pub(super) class_vtables: &'a [ClassVTable],
    pub(super) statics: &'a [StaticVar],
    pub(super) str_literals: &'a HashMap<StrId, StrLiteralInfo>,
    pub(super) type_layouts: &'a TypeLayoutTable,
    pub(super) target: &'a Target,
    pub(super) builder: String,
    pub(super) temp_counter: usize,
    pub(super) local_ptrs: Vec<Option<String>>,
    pub(super) local_tys: Vec<Option<String>>,
    pub(super) decimal_local_structs: HashMap<usize, &'static str>,
    pub(super) block_labels: HashMap<BlockId, String>,
    pub(super) return_local: Option<usize>,
    pub(super) isa_tier: CpuIsaTier,
    pub(super) available_tiers: Vec<CpuIsaTier>,
    pub(super) arch: TargetArch,
    pub(super) metadata: &'a mut MetadataRegistry,
    alias_scopes: Vec<Option<AliasScopeInfo>>,
    aux_label_counter: usize,
    pub(super) tracepoint: Option<&'a Tracepoint>,
    pub(super) current_rounding: Option<RoundingMode>,
}

#[derive(Clone)]
struct AliasScopeInfo {
    alias_scope: String,
    noalias: Option<String>,
}
impl<'a> FunctionEmitter<'a> {
    pub(super) fn current_fn_uses_sret(&self) -> bool {
        let Some(sig) = self.signatures.get(self.function.name.as_str()) else {
            return false;
        };
        let Some(c_abi) = sig.c_abi.as_ref() else {
            return false;
        };
        matches!(&c_abi.ret, CAbiReturn::IndirectSret { .. })
    }

    fn current_fn_arg_is_indirect_aggregate(&self, arg_index: usize) -> bool {
        let Some(sig) = self.signatures.get(self.function.name.as_str()) else {
            return false;
        };
        let Some(c_abi) = sig.c_abi.as_ref() else {
            return false;
        };
        let Some(param) = c_abi.params.get(arg_index) else {
            return false;
        };
        matches!(
            param.pass,
            CAbiPass::IndirectByVal { .. } | CAbiPass::IndirectPtr { .. }
        )
    }

    pub(super) fn decimal_struct_override(&self, index: usize) -> Option<Ty> {
        self.decimal_local_structs
            .get(&index)
            .map(|name| Ty::named(name.to_string()))
    }

    pub(super) fn async_vtable_symbol(&self) -> Option<&String> {
        self.async_vtables.get(&self.function.name)
    }

    pub(super) fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}_{}", self.aux_label_counter);
        self.aux_label_counter += 1;
        label
    }

    #[cfg(test)]
    pub(super) fn new(
        function: &'a MirFunction,
        signatures: &'a HashMap<String, LlvmFunctionSignature>,
        externals: &'a mut BTreeSet<&'static str>,
        vtable_symbols: &'a HashSet<String>,
        trait_vtables: &'a [TraitVTable],
        class_vtables: &'a [ClassVTable],
        isa_tier: CpuIsaTier,
        available_tiers: &[CpuIsaTier],
        arch: TargetArch,
        target: &'a Target,
        statics: &'a [StaticVar],
        str_literals: &'a HashMap<StrId, StrLiteralInfo>,
        type_layouts: &'a TypeLayoutTable,
        metadata: &'a mut MetadataRegistry,
        tracepoint: Option<&'a Tracepoint>,
    ) -> Self {
        static EMPTY_ASYNC: OnceLock<HashMap<String, String>> = OnceLock::new();
        let empty = EMPTY_ASYNC.get_or_init(HashMap::new);
        Self::new_with_async(
            function,
            signatures,
            externals,
            vtable_symbols,
            empty,
            trait_vtables,
            class_vtables,
            isa_tier,
            available_tiers,
            arch,
            target,
            statics,
            str_literals,
            type_layouts,
            metadata,
            tracepoint,
        )
    }

    pub(super) fn new_with_async(
        function: &'a MirFunction,
        signatures: &'a HashMap<String, LlvmFunctionSignature>,
        externals: &'a mut BTreeSet<&'static str>,
        vtable_symbols: &'a HashSet<String>,
        async_vtables: &'a HashMap<String, String>,
        trait_vtables: &'a [TraitVTable],
        class_vtables: &'a [ClassVTable],
        isa_tier: CpuIsaTier,
        available_tiers: &[CpuIsaTier],
        arch: TargetArch,
        target: &'a Target,
        statics: &'a [StaticVar],
        str_literals: &'a HashMap<StrId, StrLiteralInfo>,
        type_layouts: &'a TypeLayoutTable,
        metadata: &'a mut MetadataRegistry,
        tracepoint: Option<&'a Tracepoint>,
    ) -> Self {
        Self {
            function,
            signatures,
            externals,
            vtable_symbols,
            async_vtables,
            trait_vtables,
            class_vtables,
            statics,
            str_literals,
            type_layouts,
            target,
            builder: String::new(),
            temp_counter: 0,
            local_ptrs: Vec::new(),
            local_tys: Vec::new(),
            decimal_local_structs: HashMap::new(),
            block_labels: HashMap::new(),
            return_local: None,
            isa_tier,
            available_tiers: available_tiers.to_vec(),
            arch,
            metadata,
            alias_scopes: Vec::new(),
            aux_label_counter: 0,
            tracepoint,
            current_rounding: None,
        }
    }

    pub(super) fn with_rounding<T, F>(&mut self, rounding: Option<RoundingMode>, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let previous = self.current_rounding;
        self.current_rounding = rounding;
        let result = f(self);
        self.current_rounding = previous;
        result
    }

    fn emit(&mut self) -> Result<String, Error> {
        self.local_tys = self.compute_local_types()?;
        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok()
            && self.function.name.contains("AsyncTestcases::")
        {
            eprintln!(
                "[chic-debug] local types for {} (ret {}): {:?}",
                self.function.name,
                self.function.signature.ret.canonical_name(),
                self.local_tys
            );
        }
        if std::env::var("CHIC_DEBUG_LOCALS").is_ok() {
            let mir_tys: Vec<String> = self
                .function
                .body
                .locals
                .iter()
                .map(|local| local.ty.canonical_name())
                .collect();
            let local_names: Vec<Option<String>> = self
                .function
                .body
                .locals
                .iter()
                .map(|local| local.name.clone())
                .collect();
            eprintln!(
                "[chic-debug] local types for {} (ret {}): {:?} (mir={:?}) (names={:?})",
                self.function.name,
                self.function.signature.ret.canonical_name(),
                self.local_tys,
                mir_tys,
                local_names
            );
        }
        if (std::env::var("CHIC_DEBUG_PLACE_TYPES").is_ok()
            || std::env::var("CHIC_DEBUG_LOCALS").is_ok())
            && self.function.name.contains("DateTimeFormatting")
        {
            eprintln!(
                "[chic-debug] mir body for {}: {:?}",
                self.function.name, self.function.body
            );
            for key in [
                "Std::Span::ReadOnlySpan<byte>",
                "Std::Span::ReadOnlySpan",
                "Std::Span::Span<byte>",
                "Std::Span::Span",
                "Std::Span::ReadOnlySpanPtr",
                "Std::Span::SpanPtr",
            ] {
                let status = if self.type_layouts.layout_for_name(key).is_some() {
                    "present"
                } else {
                    "missing"
                };
                eprintln!("[chic-debug] layout {key}: {status}");
            }
        }
        if std::env::var("CHIC_DEBUG_CALLS").is_ok()
            && self
                .function
                .name
                .contains("Numeric::NumericBitOperations::RotateLeftInt32")
        {
            eprintln!(
                "[chic-debug] mir body for {}: {:?}",
                self.function.name, self.function.body
            );
        }
        self.init_alias_metadata();
        self.assign_block_labels();
        self.emit_blocks()?;
        Ok(self.builder.clone())
    }

    fn emit_blocks(&mut self) -> Result<(), Error> {
        let entry_id = self.function.body.entry();
        if let Some(entry) = self.function.body.blocks.iter().find(|b| b.id == entry_id) {
            self.emit_entry_block(entry)?;
        } else {
            self.initialise_locals()?;
            self.initialise_arguments()?;
        }

        for block in &self.function.body.blocks {
            if block.id == entry_id {
                continue;
            }
            let label = self.block_label(block.id)?;
            writeln!(&mut self.builder, "{label}:").ok();
            self.emit_block_contents(block)?;
        }
        Ok(())
    }

    fn emit_entry_block(&mut self, block: &BasicBlock) -> Result<(), Error> {
        let label = self.block_label(block.id)?;
        writeln!(&mut self.builder, "{label}:").ok();
        self.initialise_locals()?;
        self.initialise_arguments()?;
        self.emit_trace_enter()?;
        self.emit_block_contents(block)
    }

    fn emit_block_contents(&mut self, block: &BasicBlock) -> Result<(), Error> {
        for statement in &block.statements {
            self.emit_statement(statement)?;
        }
        if let Some(term) = &block.terminator {
            self.emit_terminator(term)?;
        } else {
            // Safeguard: emit an explicit unreachable so even malformed/unreachable
            // blocks still produce valid LLVM IR.
            writeln!(&mut self.builder, "  unreachable").ok();
        }
        Ok(())
    }

    fn assign_block_labels(&mut self) {
        for block in &self.function.body.blocks {
            self.block_labels
                .insert(block.id, format!("bb{}", block.id.0));
        }
    }

    pub(super) fn block_label(&self, id: BlockId) -> Result<String, Error> {
        self.block_labels
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::Codegen("missing block label".into()))
    }

    fn static_var(&self, id: StaticId) -> Result<&StaticVar, Error> {
        self.statics
            .get(id.0)
            .ok_or_else(|| Error::Codegen(format!("static identifier {} was not registered", id.0)))
    }

    pub(super) fn static_symbol(&self, id: StaticId) -> Result<String, Error> {
        let var = self.static_var(id)?;
        let symbol = match var.extern_spec.as_ref().and_then(|spec| spec.alias.clone()) {
            Some(alias) => alias,
            None if var.is_extern => var
                .qualified
                .rsplit("::")
                .next()
                .map(sanitise_symbol)
                .unwrap_or_else(|| sanitise_symbol(&var.qualified)),
            None => sanitise_symbol(&var.qualified),
        };
        Ok(format!("@{symbol}"))
    }

    pub(super) fn static_llvm_type(&self, id: StaticId) -> Result<String, Error> {
        let var = self.static_var(id)?;
        map_type_owned(&var.ty, Some(self.type_layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "static `{}` lowers to void type in LLVM backend",
                var.qualified
            ))
        })
    }

    fn initialise_locals(&mut self) -> Result<(), Error> {
        let body = &self.function.body;
        if self.local_tys.len() < body.locals.len() {
            return Err(Error::Codegen(
                "local type inference produced inconsistent results".into(),
            ));
        }
        self.return_local = None;
        self.local_ptrs.clear();
        let uses_sret = self.current_fn_uses_sret();
        let arg_offset = if uses_sret { 1 } else { 0 };

        for (index, local) in body.locals.iter().enumerate() {
            if matches!(local.kind, LocalKind::Return) {
                self.return_local = Some(index);
            }

            let mut alloc_ty = self.local_tys.get(index).cloned().flatten();
            if let LocalKind::Arg(arg_index) = local.kind
                && let Some(sig) = self.signatures.get(self.function.name.as_str())
                && let Some(c_abi) = sig.c_abi.as_ref()
                && let Some(param) = c_abi.params.get(arg_index)
                && matches!(param.pass, CAbiPass::Direct)
                && param.coerce.is_some()
            {
                if let Some(native_ty) = map_type_owned(&param.ty, Some(self.type_layouts))? {
                    alloc_ty = Some(native_ty.clone());
                    if let Some(slot) = self.local_tys.get_mut(index) {
                        *slot = Some(native_ty);
                    }
                }
            }
            if uses_sret
                && matches!(local.kind, LocalKind::Return)
                && !matches!(alloc_ty.as_deref(), Some("void") | None)
            {
                // C ABI sret returns receive their destination pointer as `%arg0`.
                self.local_ptrs.push(Some("%arg0".to_string()));
                continue;
            }
            if let LocalKind::Arg(arg_index) = local.kind
                && self.current_fn_arg_is_indirect_aggregate(arg_index)
                && !matches!(alloc_ty.as_deref(), Some("void") | None)
            {
                // C ABI indirect aggregate parameters are already materialised in memory and passed as a `ptr`.
                self.local_ptrs
                    .push(Some(format!("%arg{}", arg_index + arg_offset)));
                continue;
            }
            match alloc_ty.as_deref() {
                Some("void") | None => {
                    self.local_ptrs.push(None);
                }
                Some(ty) => {
                    let ptr_name = format!("%l{index}");
                    writeln!(&mut self.builder, "  {ptr_name} = alloca {ty}").ok();
                    self.local_ptrs.push(Some(ptr_name));
                }
            }
        }
        Ok(())
    }

    fn initialise_arguments(&mut self) -> Result<(), Error> {
        let body = &self.function.body;
        let arg_offset = if self.current_fn_uses_sret() { 1 } else { 0 };
        let fn_sig = self.signatures.get(self.function.name.as_str());
        for (index, local) in body.locals.iter().enumerate() {
            if let LocalKind::Arg(arg_index) = local.kind {
                if let Some(name) = local.name.as_deref() {
                    writeln!(&mut self.builder, "  ; arg{arg_index} = {name}").ok();
                }
                if self.current_fn_arg_is_indirect_aggregate(arg_index) {
                    // Indirect aggregate parameters already refer to a stack slot holding the value.
                    continue;
                }
                let Some(ptr) = self.local_ptrs.get(index).and_then(|opt| opt.as_deref()) else {
                    continue;
                };
                let ty = self
                    .local_tys
                    .get(index)
                    .and_then(|ty| ty.as_deref())
                    .ok_or_else(|| Error::Codegen("argument local missing type".into()))?;
                let arg_ty = fn_sig
                    .and_then(|sig| sig.params.get(arg_index + arg_offset))
                    .cloned()
                    .unwrap_or_else(|| ty.to_string());
                let align = self.align_for_ty(&local.ty);
                writeln!(
                    &mut self.builder,
                    "  store {arg_ty} %arg{}, ptr {ptr}, align {align}",
                    arg_index + arg_offset
                )
                .ok();
            }
        }
        Ok(())
    }

    pub(super) fn new_temp(&mut self) -> String {
        let name = format!("%t{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    pub(super) fn new_internal_label(&mut self, prefix: &str) -> String {
        let name = format!("{prefix}_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    fn init_alias_metadata(&mut self) {
        self.alias_scopes = vec![None; self.function.body.locals.len()];
        let mut alias_locals = Vec::new();
        for (index, local) in self.function.body.locals.iter().enumerate() {
            if !matches!(local.kind, LocalKind::Arg(_)) {
                continue;
            }
            if !(local.aliasing.noalias || local.aliasing.restrict) {
                continue;
            }
            if !self.local_supports_alias_metadata(index) {
                continue;
            }
            let arg_index = match local.kind {
                LocalKind::Arg(arg) => arg,
                _ => continue,
            };
            alias_locals.push((index, arg_index));
        }
        if alias_locals.is_empty() {
            return;
        }
        let fn_label = self.function.name.replace("::", "_");
        let domain_label = format!("chx.alias.domain.{fn_label}");
        let domain = self.metadata.alias_scope_domain(&domain_label);
        let mut scope_records = Vec::new();
        for (local_index, arg_index) in &alias_locals {
            let scope_label = format!("chx.alias.arg{arg_index}.{fn_label}");
            let scope_id = self.metadata.alias_scope(domain, &scope_label);
            let scope_set = self
                .metadata
                .alias_scope_set(&[scope_id])
                .expect("alias scope set requires entry");
            scope_records.push((*local_index, scope_id, scope_set));
        }
        let all_scopes: Vec<_> = scope_records.iter().map(|(_, scope, _)| *scope).collect();
        for (local_index, scope_id, scope_set) in scope_records {
            let other_scopes: Vec<_> = all_scopes
                .iter()
                .copied()
                .filter(|candidate| *candidate != scope_id)
                .collect();
            let alias_scope_ref = self.metadata.reference(scope_set);
            let noalias_ref = self
                .metadata
                .alias_scope_set(&other_scopes)
                .map(|id| self.metadata.reference(id));
            self.alias_scopes[local_index] = Some(AliasScopeInfo {
                alias_scope: alias_scope_ref,
                noalias: noalias_ref,
            });
        }
    }

    fn local_supports_alias_metadata(&self, index: usize) -> bool {
        if self.is_reference_param(index) {
            return true;
        }
        self.local_tys
            .get(index)
            .and_then(|ty| ty.as_deref())
            .map(|ty| ty == "ptr")
            .unwrap_or(false)
    }

    pub(super) fn alias_suffix_for_place(&self, place: &Place) -> Option<String> {
        let info = self.alias_scopes.get(place.local.0)?.as_ref()?;
        if !self.place_accesses_pointee(place) {
            return None;
        }
        let mut suffix = format!(", !alias.scope {}", info.alias_scope);
        if let Some(noalias) = &info.noalias {
            suffix.push_str(", !noalias ");
            suffix.push_str(noalias);
        }
        Some(suffix)
    }

    fn place_accesses_pointee(&self, place: &Place) -> bool {
        if self.is_reference_param(place.local.0) {
            return true;
        }
        place
            .projection
            .iter()
            .any(|elem| matches!(elem, ProjectionElem::Deref | ProjectionElem::Index(_)))
    }

    pub(super) fn async_runtime_context_ptr(&mut self) -> Result<Option<String>, Error> {
        if let Some((index, _)) = self
            .function
            .body
            .locals
            .iter()
            .enumerate()
            .find(|(_, decl)| decl.name.as_deref() == Some("__async_ctx"))
        {
            let place = Place::new(LocalId(index));
            let ptr = self.place_ptr(&place)?;
            return Ok(Some(ptr));
        }
        Ok(None)
    }
}
