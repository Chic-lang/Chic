use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::encoding::{push_string, write_u32};
use crate::codegen::wasm::metadata::build_metadata_payload;
use crate::codegen::wasm::runtime_hooks::{ALL_RUNTIME_HOOKS, RuntimeHook};
use crate::codegen::wasm::{STACK_BASE, ValueType, WASM_MAGIC, WASM_VERSION, ensure_u32, map_type};
use crate::eq_glue::SynthesisedEqGlue;
use crate::error::Error;
use crate::frontend::parser::parse_type_expression_text;
use crate::hash_glue::SynthesisedHashGlue;
use crate::mir::FloatValue;
use crate::mir::{
    AliasContract, ConstValue, LocalKind, MirFunction, MirModule, StaticId, StaticVar, StrId, Ty,
};
use crate::perf::{PerfMetadata, Tracepoint};
use crate::type_metadata::SynthesisedTypeMetadata;

use super::sections::Section;
use super::signature::FunctionSignature;
use super::support::{DataSegment, RuntimeImport, align_u32};

#[derive(Debug, Clone, Copy)]
pub(crate) struct WasmStrLiteral {
    pub(crate) offset: u32,
    pub(crate) len: u32,
}

pub(crate) struct ModuleBuilder<'a> {
    pub(crate) module: &'a MirModule,
    pub(crate) entry: Option<String>,
    pub(crate) kind: ChicKind,
    pub(crate) coverage_enabled: bool,
    type_metadata_entries: &'a [SynthesisedTypeMetadata],
    hash_glue_entries: &'a [SynthesisedHashGlue],
    eq_glue_entries: &'a [SynthesisedEqGlue],
    reflection_payload: Option<Vec<u8>>,
    pub(super) signatures: Vec<FunctionSignature>,
    pub(super) signature_indices: HashMap<FunctionSignature, u32>,
    pub(super) function_type_indices: Vec<u32>,
    pub(super) index_by_name: HashMap<String, u32>,
    pub(super) function_signatures: HashMap<String, FunctionSignature>,
    pub(super) function_return_tys: HashMap<String, Ty>,
    pub(super) function_param_tys: HashMap<String, Vec<Ty>>,
    pub(super) alias_contracts: HashMap<String, Vec<AliasContract>>,
    pub(super) imports: Vec<RuntimeImport>,
    pub(super) data_segments: Vec<DataSegment>,
    pub(super) string_literals: HashMap<StrId, WasmStrLiteral>,
    pub(super) table_initializers: Vec<u32>,
    pub(super) trait_vtable_offsets: HashMap<String, u32>,
    pub(super) class_vtable_offsets: HashMap<String, u32>,
    pub(super) async_vtable_offsets: HashMap<String, u32>,
    pub(super) static_offsets: HashMap<StaticId, u32>,
    pub(super) tracepoints: HashMap<String, &'a Tracepoint>,
    linear_memory_offset: u32,
}

impl<'a> ModuleBuilder<'a> {
    pub(crate) fn new(
        module: &'a MirModule,
        entry: Option<String>,
        kind: ChicKind,
        type_metadata_entries: &'a [SynthesisedTypeMetadata],
        hash_glue_entries: &'a [SynthesisedHashGlue],
        eq_glue_entries: &'a [SynthesisedEqGlue],
        reflection_payload: Option<Vec<u8>>,
        perf: &'a PerfMetadata,
        coverage_enabled: bool,
    ) -> Result<Self, Error> {
        let mut builder = Self {
            module,
            entry,
            kind,
            coverage_enabled,
            type_metadata_entries,
            hash_glue_entries,
            eq_glue_entries,
            reflection_payload,
            signatures: Vec::new(),
            signature_indices: HashMap::new(),
            function_type_indices: Vec::new(),
            index_by_name: HashMap::new(),
            function_signatures: HashMap::new(),
            function_return_tys: HashMap::new(),
            function_param_tys: HashMap::new(),
            alias_contracts: HashMap::new(),
            imports: Vec::new(),
            data_segments: Vec::new(),
            string_literals: HashMap::new(),
            table_initializers: Vec::new(),
            trait_vtable_offsets: HashMap::new(),
            class_vtable_offsets: HashMap::new(),
            async_vtable_offsets: HashMap::new(),
            static_offsets: HashMap::new(),
            tracepoints: HashMap::new(),
            linear_memory_offset: 0,
        };

        builder.tracepoints = perf
            .tracepoints
            .iter()
            .map(|trace| (trace.function.clone(), trace))
            .collect();

        // WebAssembly doesn't have a native weak-linking model. For definitions we simply ignore
        // the weak marker (there is no link step that could introduce duplicate definitions).
        // Weak `extern` imports, however, cannot be represented faithfully.
        for function in &builder.module.functions {
            if function
                .extern_spec
                .as_ref()
                .map_or(false, |spec| spec.weak)
            {
                return Err(Error::Codegen(format!(
                    "weak extern imports are not supported by the WASM backend (`{}`)",
                    function.name
                )));
            }
        }

        for (idx, function) in builder.module.functions.iter().enumerate() {
            let sig = FunctionSignature::from_mir(function, &builder.module.type_layouts);
            builder
                .function_signatures
                .insert(function.name.clone(), sig.clone());
            builder
                .function_return_tys
                .insert(function.name.clone(), function.signature.ret.clone());
            builder
                .function_param_tys
                .insert(function.name.clone(), function.signature.params.clone());
            builder
                .alias_contracts
                .insert(function.name.clone(), collect_alias_contracts(function));
            let type_index = builder.intern_signature(sig)?;
            builder.function_type_indices.push(type_index);
            builder.index_by_name.insert(
                function.name.clone(),
                ensure_u32(idx, "function index exceeds WebAssembly limits")?,
            );
        }
        if std::env::var_os("CHIC_DEBUG_WASM_FUNCTIONS").is_some() {
            for (name, index) in builder.index_by_name.iter() {
                eprintln!("{index:>5} {name}");
            }
        }
        // Pre-register function-pointer signatures so indirect calls find table entries.
        for key in builder.module.type_layouts.types.keys() {
            if key.trim_start().starts_with("fn") {
                if let Some(expr) = parse_type_expression_text(key) {
                    let ty = Ty::from_type_expr(&expr);
                    if let Ty::Fn(fn_ty) = ty {
                        let sig =
                            FunctionSignature::from_fn_ty(&fn_ty, &builder.module.type_layouts);
                        let _ = builder.intern_signature(sig);
                    }
                }
            }
        }

        for hook in ALL_RUNTIME_HOOKS {
            if matches!(hook, RuntimeHook::CoverageHit) && !coverage_enabled {
                continue;
            }
            builder.register_runtime_hook(*hook)?;
        }

        let import_limit = ensure_u32(
            builder.imports.len(),
            "runtime hook index exceeds WebAssembly limits",
        )?;
        let mut extern_imports: HashSet<String> = HashSet::new();
        for function in builder.module.functions.iter() {
            if function.extern_spec.is_none() {
                continue;
            }
            let mut candidates = Vec::new();
            if let Some(spec) = &function.extern_spec {
                if let Some(alias) = &spec.alias {
                    candidates.push(alias.clone());
                }
            }
            if let Some(last) = function.name.rsplit("::").next() {
                candidates.push(last.to_string());
            }
            for symbol in candidates {
                if let Some(&idx) = builder.index_by_name.get(&symbol) {
                    if idx < import_limit {
                        builder.index_by_name.insert(function.name.clone(), idx);
                        extern_imports.insert(function.name.clone());
                        break;
                    }
                }
            }
        }

        let import_count = builder.imports.len();
        for (idx, function) in builder.module.functions.iter().enumerate() {
            let total = import_count.checked_add(idx).ok_or_else(|| {
                Error::Codegen("function index exceeds WebAssembly limits".into())
            })?;
            let index = ensure_u32(total, "function index exceeds WebAssembly limits")?;
            let preserve_import = extern_imports.contains(&function.name);
            if preserve_import {
                continue;
            }
            match builder.index_by_name.get_mut(&function.name) {
                Some(entry) => {
                    *entry = index;
                }
                None => {
                    builder.index_by_name.insert(function.name.clone(), index);
                }
            }
        }

        let total_functions = import_count
            .checked_add(builder.module.functions.len())
            .ok_or_else(|| Error::Codegen("function index exceeds WebAssembly limits".into()))?;
        let mut table_initializers = Vec::with_capacity(total_functions);
        for idx in 0..total_functions {
            table_initializers.push(ensure_u32(
                idx,
                "function index exceeds WebAssembly limits",
            )?);
        }
        builder.table_initializers = table_initializers;

        builder.allocate_string_literals()?;
        builder.allocate_static_vars()?;
        builder.allocate_async_vtables()?;
        builder.allocate_trait_vtables()?;
        builder.allocate_class_vtables()?;
        if builder.linear_memory_offset >= STACK_BASE {
            return Err(Error::Codegen(
                "linear memory reserved for stack exhausted by data segments in WASM backend"
                    .into(),
            ));
        }
        Ok(builder)
    }

    fn intern_signature(&mut self, sig: FunctionSignature) -> Result<u32, Error> {
        if let Some(&index) = self.signature_indices.get(&sig) {
            return Ok(index);
        }
        let index = ensure_u32(
            self.signatures.len(),
            "function type count exceeds WebAssembly limits",
        )?;
        self.signatures.push(sig.clone());
        self.signature_indices.insert(sig, index);
        Ok(index)
    }

    fn register_runtime_hook(&mut self, hook: RuntimeHook) -> Result<(), Error> {
        let signature = hook.signature();
        let type_index = self.intern_signature(signature.clone())?;

        let import_index = ensure_u32(
            self.imports.len(),
            "runtime hook index exceeds WebAssembly limits",
        )?;
        self.imports.push(RuntimeImport {
            module: hook.module(),
            name: hook.name(),
            type_index,
        });

        self.index_by_name
            .insert(hook.qualified_name(), import_index);
        self.index_by_name
            .insert(hook.legacy_symbol(), import_index);
        self.function_signatures
            .insert(hook.qualified_name(), signature.clone());
        self.function_signatures
            .insert(hook.legacy_symbol(), signature);
        Ok(())
    }

    fn allocate_string_literals(&mut self) -> Result<(), Error> {
        let mut offset: u32 = self.linear_memory_offset;
        for literal in &self.module.interned_strs {
            offset = align_u32(offset, 4);
            let raw_len = ensure_u32(
                literal.value.len(),
                "string literal length exceeds WebAssembly limits",
            )?;
            let mut bytes = literal.value.as_bytes().to_vec();
            if bytes.is_empty() {
                bytes.push(0);
            }
            let stored_len = ensure_u32(
                bytes.len(),
                "string literal storage exceeds WebAssembly limits",
            )?;
            self.data_segments.push(DataSegment {
                offset,
                bytes: bytes.clone(),
            });
            self.string_literals.insert(
                literal.id,
                WasmStrLiteral {
                    offset,
                    len: raw_len,
                },
            );
            offset = offset.checked_add(stored_len).ok_or_else(|| {
                Error::Codegen(
                    "string literal data exceeds addressable space in WASM backend".into(),
                )
            })?;
        }
        self.linear_memory_offset = offset;
        Ok(())
    }

    fn allocate_static_vars(&mut self) -> Result<(), Error> {
        if self.module.statics.is_empty() {
            return Ok(());
        }

        for var in &self.module.statics {
            if var.is_extern {
                if var.is_weak_import {
                    self.static_offsets.insert(var.id, 0);
                    continue;
                }
                return Err(Error::Codegen(format!(
                    "[FFI-WASM0001] extern globals are unsupported on the WASM backend (`{}`)",
                    var.qualified
                )));
            }
        }

        for var in &self.module.statics {
            if var.is_extern {
                continue;
            }
            if var.threadlocal {
                return Err(Error::Codegen(format!(
                    "threadlocal statics are not supported by the WASM backend (`{}`)",
                    var.qualified
                )));
            }
        }

        let mut offset = align_u32(self.linear_memory_offset, 4);
        for var in &self.module.statics {
            let value_ty = map_type(&var.ty);
            let base_width = scalar_width(value_ty).ok_or_else(|| {
                Error::Codegen(format!(
                    "static `{}` of type `{}` is not supported by the WASM backend",
                    var.qualified,
                    var.ty.canonical_name()
                ))
            })?;
            let width = self
                .module
                .type_layouts
                .size_and_align_for_ty(&var.ty)
                .map(|(size, _)| size.max(base_width))
                .unwrap_or(base_width);
            offset = align_u32(offset, width as u32);
            let bytes = encode_static_bytes(var, width, value_ty)?;
            self.static_offsets.insert(var.id, offset);
            self.data_segments.push(DataSegment {
                offset,
                bytes: bytes.clone(),
            });
            offset = offset.checked_add(width as u32).ok_or_else(|| {
                Error::Codegen("static data exceeds addressable space in WASM backend".into())
            })?;
        }
        self.linear_memory_offset = offset;
        Ok(())
    }

    fn allocate_async_vtables(&mut self) -> Result<(), Error> {
        if self.module.async_plans.is_empty() {
            return Ok(());
        }
        let mut offset = align_u32(self.linear_memory_offset, 4);
        for plan in &self.module.async_plans {
            let poll_index = *self.index_by_name.get(&plan.poll_fn).ok_or_else(|| {
                Error::Codegen(format!(
                    "async poll stub `{}` missing function index",
                    plan.poll_fn
                ))
            })?;
            let drop_index = *self.index_by_name.get(&plan.drop_fn).ok_or_else(|| {
                Error::Codegen(format!(
                    "async drop stub `{}` missing function index",
                    plan.drop_fn
                ))
            })?;
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&poll_index.to_le_bytes());
            bytes.extend_from_slice(&drop_index.to_le_bytes());
            self.async_vtable_offsets
                .insert(plan.function_name.clone(), offset);
            self.data_segments.push(DataSegment { offset, bytes });
            offset = offset.checked_add(8).ok_or_else(|| {
                Error::Codegen("async vtable data exceeds addressable space".into())
            })?;
            offset = align_u32(offset, 4);
        }
        self.linear_memory_offset = offset;
        Ok(())
    }

    fn allocate_trait_vtables(&mut self) -> Result<(), Error> {
        let mut offset = align_u32(self.linear_memory_offset, 4);
        for table in &self.module.trait_vtables {
            let mut bytes = Vec::with_capacity(table.slots.len() * 4);
            for slot in &table.slots {
                let index = *self.index_by_name.get(&slot.symbol).ok_or_else(|| {
                    Error::Codegen(format!(
                        "trait vtable slot `{}` references unknown function",
                        slot.symbol
                    ))
                })?;
                bytes.extend_from_slice(&index.to_le_bytes());
            }
            if bytes.is_empty() {
                bytes.resize(4, 0);
            }
            self.trait_vtable_offsets
                .insert(table.symbol.clone(), offset);
            self.data_segments.push(DataSegment {
                offset,
                bytes: bytes.clone(),
            });
            offset = offset.checked_add(bytes.len() as u32).ok_or_else(|| {
                Error::Codegen("trait vtable data exceeds addressable space in WASM backend".into())
            })?;
            offset = align_u32(offset, 4);
        }
        self.linear_memory_offset = offset;
        Ok(())
    }

    fn allocate_class_vtables(&mut self) -> Result<(), Error> {
        let mut offset = align_u32(self.linear_memory_offset, 4);
        for table in &self.module.class_vtables {
            let mut bytes = Vec::with_capacity(table.slots.len() * 4);
            for slot in &table.slots {
                let index = *self.index_by_name.get(&slot.symbol).ok_or_else(|| {
                    Error::Codegen(format!(
                        "class vtable slot `{}` references unknown function",
                        slot.symbol
                    ))
                })?;
                bytes.extend_from_slice(&index.to_le_bytes());
            }
            if bytes.is_empty() {
                bytes.resize(4, 0);
            }
            self.class_vtable_offsets
                .insert(table.symbol.clone(), offset);
            self.data_segments.push(DataSegment {
                offset,
                bytes: bytes.clone(),
            });
            offset = offset.checked_add(bytes.len() as u32).ok_or_else(|| {
                Error::Codegen("class vtable data exceeds addressable space in WASM backend".into())
            })?;
            offset = align_u32(offset, 4);
        }
        self.linear_memory_offset = offset;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn static_offsets(&self) -> &HashMap<StaticId, u32> {
        &self.static_offsets
    }

    #[cfg(test)]
    pub(crate) fn class_vtable_offsets(&self) -> &HashMap<String, u32> {
        &self.class_vtable_offsets
    }

    pub(crate) fn emit(&self) -> Result<Vec<u8>, Error> {
        let module_start = Instant::now();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&WASM_MAGIC);
        bytes.extend_from_slice(&WASM_VERSION);

        let type_start = Instant::now();
        let type_section = self.emit_type_section()?;
        wasm_debug!("emit_type_section completed in {:?}", type_start.elapsed());
        type_section.encode_into(&mut bytes)?;

        if !self.imports.is_empty() {
            let import_start = Instant::now();
            let import_section = self.emit_import_section()?;
            wasm_debug!(
                "emit_import_section completed in {:?}",
                import_start.elapsed()
            );
            import_section.encode_into(&mut bytes)?;
        }

        let function_start = Instant::now();
        let function_section = self.emit_function_section()?;
        wasm_debug!(
            "emit_function_section completed in {:?}",
            function_start.elapsed()
        );
        function_section.encode_into(&mut bytes)?;

        let table_start = Instant::now();
        if let Some(table_section) = self.emit_table_section()? {
            wasm_debug!(
                "emit_table_section completed in {:?}",
                table_start.elapsed()
            );
            table_section.encode_into(&mut bytes)?;
        } else {
            wasm_debug!(
                "emit_table_section completed in {:?} (no tables)",
                table_start.elapsed()
            );
        }

        let memory_start = Instant::now();
        if let Some(memory_section) = self.emit_memory_section()? {
            wasm_debug!(
                "emit_memory_section completed in {:?}",
                memory_start.elapsed()
            );
            memory_section.encode_into(&mut bytes)?;
        }

        let global_start = Instant::now();
        if let Some(global_section) = self.emit_global_section()? {
            wasm_debug!(
                "emit_global_section completed in {:?}",
                global_start.elapsed()
            );
            global_section.encode_into(&mut bytes)?;
        }

        let export_start = Instant::now();
        let export_section = self.emit_export_section()?;
        if let Some(section) = export_section {
            wasm_debug!(
                "emit_export_section completed in {:?}",
                export_start.elapsed()
            );
            section.encode_into(&mut bytes)?;
        } else {
            wasm_debug!(
                "emit_export_section completed in {:?} (no exports)",
                export_start.elapsed()
            );
        }

        let element_start = Instant::now();
        if let Some(element_section) = self.emit_element_section()? {
            wasm_debug!(
                "emit_element_section completed in {:?}",
                element_start.elapsed()
            );
            element_section.encode_into(&mut bytes)?;
        } else {
            wasm_debug!(
                "emit_element_section completed in {:?} (no elements)",
                element_start.elapsed()
            );
        }

        let code_start = Instant::now();
        let code_section = self.emit_code_section()?;
        wasm_debug!("emit_code_section completed in {:?}", code_start.elapsed());
        code_section.encode_into(&mut bytes)?;

        if let Some(data_section) = self.emit_data_section()? {
            wasm_debug!(
                "emit_data_section emitted {} segment(s)",
                self.data_segments.len()
            );
            data_section.encode_into(&mut bytes)?;
        } else {
            wasm_debug!("emit_data_section skipped (no data segments)");
        }

        let metadata_start = Instant::now();
        let metadata_section = self.emit_metadata_section()?;
        wasm_debug!(
            "emit_metadata_section completed in {:?}",
            metadata_start.elapsed()
        );
        metadata_section.encode_into(&mut bytes)?;

        if let Some(section) = self.emit_reflection_section()? {
            wasm_debug!(
                "emit_reflection_section completed in {:?}",
                metadata_start.elapsed()
            );
            section.encode_into(&mut bytes)?;
        }

        if let Some(section) = self.emit_type_metadata_section()? {
            wasm_debug!(
                "emit_type_metadata_section completed in {:?}",
                metadata_start.elapsed()
            );
            section.encode_into(&mut bytes)?;
        }
        if let Some(section) = self.emit_hash_glue_section()? {
            wasm_debug!(
                "emit_hash_glue_section completed in {:?}",
                metadata_start.elapsed()
            );
            section.encode_into(&mut bytes)?;
        }
        if let Some(section) = self.emit_eq_glue_section()? {
            wasm_debug!(
                "emit_eq_glue_section completed in {:?}",
                metadata_start.elapsed()
            );
            section.encode_into(&mut bytes)?;
        }

        let iface_start = Instant::now();
        if let Some(section) = self.emit_interface_defaults_section()? {
            wasm_debug!(
                "emit_interface_defaults_section completed in {:?}",
                iface_start.elapsed()
            );
            section.encode_into(&mut bytes)?;
        }

        if let Some(section) = self.emit_alias_contracts_section()? {
            wasm_debug!("emit_alias_contracts_section emitted metadata");
            section.encode_into(&mut bytes)?;
        }

        if let Some(section) = self.emit_hint_section()? {
            wasm_debug!("emit_hint_section emitted metadata");
            section.encode_into(&mut bytes)?;
        }

        if let Some(section) = self.emit_function_names_section()? {
            wasm_debug!("emit_function_names_section emitted metadata");
            section.encode_into(&mut bytes)?;
        }

        wasm_debug!(
            "ModuleBuilder::emit completed in {:?}",
            module_start.elapsed()
        );

        Ok(bytes)
    }

    pub(crate) fn emit_metadata_section(&self) -> Result<Section, Error> {
        let kind_label = self.kind.as_str();
        let payload = build_metadata_payload(kind_label)?;
        Ok(Section::new(0, payload))
    }

    pub(crate) fn emit_type_metadata_section(&self) -> Result<Option<Section>, Error> {
        if self.type_metadata_entries.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        push_string(&mut payload, "chic.type.metadata")?;
        write_u32(
            &mut payload,
            ensure_u32(
                self.type_metadata_entries.len(),
                "type metadata count exceeds WebAssembly limits",
            )?,
        );
        for entry in self.type_metadata_entries {
            payload.extend_from_slice(&entry.type_identity.to_le_bytes());
            let size = ensure_u32(
                entry.size,
                &format!("type `{}` size exceeds WebAssembly limits", entry.type_name),
            )?;
            let align = ensure_u32(
                entry.align.max(1),
                &format!(
                    "type `{}` alignment exceeds WebAssembly limits",
                    entry.type_name
                ),
            )?;
            write_u32(&mut payload, size);
            write_u32(&mut payload, align);
            let variance_len = ensure_u32(
                entry.variances.len(),
                &format!(
                    "type `{}` variance count exceeds WebAssembly limits",
                    entry.type_name
                ),
            )?;
            write_u32(&mut payload, variance_len);
            for variance in &entry.variances {
                payload.push(variance.encode());
            }
            write_u32(&mut payload, entry.flags.bits());
        }
        Ok(Some(Section::new(0, payload)))
    }

    pub(crate) fn emit_reflection_section(&self) -> Result<Option<Section>, Error> {
        let Some(payload) = self.reflection_payload.as_ref() else {
            return Ok(None);
        };
        if payload.is_empty() {
            return Ok(None);
        }
        let mut data = Vec::new();
        push_string(&mut data, "chic.reflect")?;
        data.extend_from_slice(payload);
        Ok(Some(Section::new(0, data)))
    }

    pub(crate) fn emit_hash_glue_section(&self) -> Result<Option<Section>, Error> {
        if self.hash_glue_entries.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        push_string(&mut payload, "chic.hash.glue")?;
        write_u32(
            &mut payload,
            ensure_u32(
                self.hash_glue_entries.len(),
                "hash glue entry count exceeds WebAssembly limits",
            )?,
        );
        for entry in self.hash_glue_entries {
            payload.extend_from_slice(&entry.type_identity.to_le_bytes());
            let index = *self.index_by_name.get(&entry.symbol).ok_or_else(|| {
                Error::Codegen(format!(
                    "hash glue function `{}` is missing from WASM module",
                    entry.symbol
                ))
            })?;
            write_u32(&mut payload, index);
        }
        Ok(Some(Section::new(0, payload)))
    }

    pub(crate) fn emit_eq_glue_section(&self) -> Result<Option<Section>, Error> {
        if self.eq_glue_entries.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        push_string(&mut payload, "chic.eq.glue")?;
        write_u32(
            &mut payload,
            ensure_u32(
                self.eq_glue_entries.len(),
                "eq glue entry count exceeds WebAssembly limits",
            )?,
        );
        for entry in self.eq_glue_entries {
            payload.extend_from_slice(&entry.type_identity.to_le_bytes());
            let index = *self.index_by_name.get(&entry.symbol).ok_or_else(|| {
                Error::Codegen(format!(
                    "eq glue function `{}` is missing from WASM module",
                    entry.symbol
                ))
            })?;
            write_u32(&mut payload, index);
        }
        Ok(Some(Section::new(0, payload)))
    }

    pub(crate) fn emit_hint_section(&self) -> Result<Option<Section>, Error> {
        let mut entries = Vec::new();
        for func in &self.module.functions {
            let hints = func.optimization_hints;
            if hints.is_empty() {
                continue;
            }
            let mut parts = Vec::new();
            if hints.hot {
                parts.push("hot");
            }
            if hints.cold {
                parts.push("cold");
            }
            if hints.always_inline {
                parts.push("always_inline");
            }
            if hints.never_inline {
                parts.push("never_inline");
            }
            if parts.is_empty() {
                continue;
            }
            entries.push(format!("{}:{}", func.name, parts.join("|")));
        }

        if entries.is_empty() {
            return Ok(None);
        }

        let mut payload = Vec::new();
        push_string(&mut payload, "chic.hints")?;
        push_string(&mut payload, &entries.join(";"))?;
        Ok(Some(Section::new(0, payload)))
    }

    pub(crate) fn emit_function_names_section(&self) -> Result<Option<Section>, Error> {
        if self.module.functions.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        push_string(&mut payload, "chic.fn.names")?;
        write_u32(
            &mut payload,
            ensure_u32(
                self.module.functions.len(),
                "function count exceeds WebAssembly limits",
            )?,
        );
        for func in &self.module.functions {
            push_string(&mut payload, &func.name)?;
        }
        Ok(Some(Section::new(0, payload)))
    }

    pub(crate) fn emit_interface_defaults_section(&self) -> Result<Option<Section>, Error> {
        if self.module.interface_defaults.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        push_string(&mut payload, "chic.iface.defaults")?;
        write_u32(
            &mut payload,
            ensure_u32(
                self.module.interface_defaults.len(),
                "interface default count exceeds WebAssembly limits",
            )?,
        );
        for binding in &self.module.interface_defaults {
            push_string(&mut payload, &binding.implementer)?;
            push_string(&mut payload, &binding.interface)?;
            push_string(&mut payload, &binding.method)?;
            let Some(&index) = self.index_by_name.get(&binding.symbol) else {
                return Err(Error::Codegen(format!(
                    "interface default symbol `{}` missing from WebAssembly module",
                    binding.symbol
                )));
            };
            write_u32(&mut payload, index);
        }
        Ok(Some(Section::new(0, payload)))
    }

    pub(crate) fn emit_alias_contracts_section(&self) -> Result<Option<Section>, Error> {
        let mut entries = Vec::new();
        for function in &self.module.functions {
            if let Some(contracts) = self.alias_contracts.get(&function.name) {
                if contracts.iter().any(alias_contract_has_data) {
                    entries.push((&function.name, contracts));
                }
            }
        }
        if entries.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        push_string(&mut payload, "chx.alias.contracts")?;
        write_u32(
            &mut payload,
            ensure_u32(
                entries.len(),
                "alias metadata entry count exceeds WebAssembly limits",
            )?,
        );
        for (name, contracts) in entries {
            push_string(&mut payload, name)?;
            write_u32(
                &mut payload,
                ensure_u32(
                    contracts.len(),
                    "alias contract parameter count exceeds WebAssembly limits",
                )?,
            );
            for contract in contracts.iter() {
                payload.push(alias_contract_flags(contract));
                write_u32(&mut payload, contract.alignment.unwrap_or(0));
            }
        }
        Ok(Some(Section::new(0, payload)))
    }
}

fn scalar_width(value_ty: ValueType) -> Option<usize> {
    match value_ty {
        ValueType::I32 | ValueType::F32 => Some(4),
        ValueType::I64 | ValueType::F64 => Some(8),
    }
}

fn encode_static_bytes(
    var: &StaticVar,
    width: usize,
    value_ty: ValueType,
) -> Result<Vec<u8>, Error> {
    let Some(initializer) = var.initializer.as_ref() else {
        return Ok(vec![0u8; width]);
    };

    match value_ty {
        ValueType::I32 => match initializer {
            ConstValue::Int(value) | ConstValue::Int32(value) => {
                encode_signed(*value, width, &var.qualified)
            }
            ConstValue::UInt(value) => encode_unsigned(*value, width, &var.qualified),
            ConstValue::Bool(value) => encode_unsigned(u128::from(*value), width, &var.qualified),
            ConstValue::Char(value) => encode_unsigned(u128::from(*value), width, &var.qualified),
            ConstValue::Enum { discriminant, .. } => {
                encode_signed(*discriminant, width, &var.qualified)
            }
            ConstValue::Null | ConstValue::Unit => Ok(vec![0u8; width]),
            other => Err(Error::Codegen(format!(
                "static `{}` initializer `{other:?}` is not supported in WASM backend",
                var.qualified
            ))),
        },
        ValueType::I64 => match initializer {
            ConstValue::Int(value) | ConstValue::Int32(value) => {
                encode_signed(*value, width, &var.qualified)
            }
            ConstValue::UInt(value) => encode_unsigned(*value, width, &var.qualified),
            ConstValue::Enum { discriminant, .. } => {
                encode_signed(*discriminant, width, &var.qualified)
            }
            ConstValue::Null | ConstValue::Unit => Ok(vec![0u8; width]),
            other => Err(Error::Codegen(format!(
                "static `{}` initializer `{other:?}` is not supported in WASM backend",
                var.qualified
            ))),
        },
        ValueType::F32 | ValueType::F64 => match initializer {
            ConstValue::Float(value) => encode_float(*value, width, &var.qualified),
            ConstValue::Null | ConstValue::Unit => Ok(vec![0u8; width]),
            other => Err(Error::Codegen(format!(
                "static `{}` initializer `{other:?}` is not supported in WASM backend",
                var.qualified
            ))),
        },
    }
}

fn encode_signed(value: i128, width: usize, context: &str) -> Result<Vec<u8>, Error> {
    let bits = (width * 8) as u32;
    let min = -(1i128 << (bits - 1));
    let max = (1i128 << (bits - 1)) - 1;
    if value < min || value > max {
        return Err(Error::Codegen(format!(
            "value {value} does not fit into {bits}-bit static `{context}`"
        )));
    }
    let bytes = value.to_le_bytes();
    Ok(bytes[..width].to_vec())
}

fn encode_unsigned(value: u128, width: usize, context: &str) -> Result<Vec<u8>, Error> {
    let bits = (width * 8) as u32;
    let max = if bits == 128 {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    };
    if value > max {
        return Err(Error::Codegen(format!(
            "value {value} exceeds {bits}-bit capacity for static `{context}`"
        )));
    }
    let bytes = value.to_le_bytes();
    Ok(bytes[..width].to_vec())
}

fn encode_float(value: FloatValue, width: usize, context: &str) -> Result<Vec<u8>, Error> {
    match width {
        4 => Ok(value.to_f32().to_le_bytes().to_vec()),
        8 => Ok(value.to_f64().to_le_bytes().to_vec()),
        other => Err(Error::Codegen(format!(
            "static `{context}` uses unsupported float width {other} bytes"
        ))),
    }
}

fn collect_alias_contracts(function: &MirFunction) -> Vec<AliasContract> {
    let mut contracts = vec![AliasContract::default(); function.signature.params.len()];
    for local in &function.body.locals {
        if let LocalKind::Arg(index) = local.kind {
            if let Some(slot) = contracts.get_mut(index) {
                *slot = local.aliasing;
            }
        }
    }
    contracts
}

fn alias_contract_has_data(contract: &AliasContract) -> bool {
    contract.noalias
        || contract.restrict
        || contract.readonly
        || contract.writeonly
        || contract.nocapture
        || contract.expose_address
        || contract.alignment.is_some()
}

fn alias_contract_flags(contract: &AliasContract) -> u8 {
    let mut flags = 0u8;
    if contract.noalias {
        flags |= 0x01;
    }
    if contract.restrict {
        flags |= 0x02;
    }
    if contract.readonly {
        flags |= 0x04;
    }
    if contract.writeonly {
        flags |= 0x08;
    }
    if contract.nocapture {
        flags |= 0x10;
    }
    if contract.expose_address {
        flags |= 0x20;
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chic_kind::ChicKind;
    use crate::mir::{
        Abi, AsyncFrameMetrics, AsyncFramePolicy, AsyncLoweringArtifact, BasicBlock, FnSig,
        FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction, MirModule, TraitVTable, Ty,
        VTableSlot,
    };

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

    #[test]
    fn encode_float_preserves_bit_patterns() {
        let nan = FloatValue::f32_from_bits(0x7fc0_0bad);
        let encoded = encode_float(nan, 4, "nan").expect("encode f32");
        assert_eq!(encoded, 0x7fc0_0badu32.to_le_bytes());

        let neg_zero = FloatValue::from_f64(-0.0);
        let encoded64 = encode_float(neg_zero, 8, "neg_zero").expect("encode f64");
        assert_eq!(encoded64, (-0.0f64).to_le_bytes());
    }

    #[test]
    fn allocates_trait_vtable_segments() {
        let mut module = MirModule::default();
        module
            .functions
            .push(simple_function("Demo::Formatter::Format"));
        module.trait_vtables.push(TraitVTable {
            symbol: "__vtable_Demo__Formatter__Demo__Widget".into(),
            trait_name: "Demo::Formatter".into(),
            impl_type: "Demo::Widget".into(),
            slots: vec![VTableSlot {
                method: "Format".into(),
                symbol: "Demo::Formatter::Format".into(),
            }],
        });

        let perf = PerfMetadata::default();
        let builder = ModuleBuilder::new(
            &module,
            None,
            ChicKind::Executable,
            &[],
            &[],
            &[],
            None,
            &perf,
            false,
        )
        .expect("builder creation");
        let offset = builder
            .trait_vtable_offsets
            .get("__vtable_Demo__Formatter__Demo__Widget")
            .copied()
            .expect("vtable offset recorded");
        let segment = builder
            .data_segments
            .iter()
            .find(|seg| seg.offset == offset)
            .expect("data segment for vtable");
        assert_eq!(segment.bytes.len(), 4);
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&segment.bytes[..4]);
        let index = u32::from_le_bytes(bytes);
        assert_eq!(
            index,
            *builder
                .index_by_name
                .get("Demo::Formatter::Format")
                .expect("function index"),
        );
    }

    #[test]
    fn reflection_section_is_emitted_when_payload_present() {
        let module = MirModule::default();
        let perf = PerfMetadata::default();
        let payload = br#"{"version":2,"types":[]}"#.to_vec();
        let builder = ModuleBuilder::new(
            &module,
            None,
            ChicKind::Executable,
            &[],
            &[],
            &[],
            Some(payload),
            &perf,
            false,
        )
        .expect("builder creation");
        let bytes = builder.emit().expect("emit wasm module");
        assert!(
            bytes
                .windows(b"chic.reflect".len())
                .any(|window| window == b"chic.reflect"),
            "reflection custom section name missing"
        );
    }

    #[test]
    fn allocates_async_vtable_segments() {
        let mut module = MirModule::default();
        module.functions.push(simple_function("Demo::Async::Work"));
        module
            .functions
            .push(simple_function("Demo::Async::Work::poll"));
        module
            .functions
            .push(simple_function("Demo::Async::Work::drop"));
        module.async_plans.push(AsyncLoweringArtifact {
            function_index: 0,
            function_name: "Demo::Async::Work".into(),
            frame_type: "Demo::Async::Work::Frame".into(),
            context_local: None,
            policy: AsyncFramePolicy::default(),
            metrics: AsyncFrameMetrics::default(),
            frame_fields: Vec::new(),
            suspend_points: Vec::new(),
            resume_states: Vec::new(),
            state_count: 1,
            poll_fn: "Demo::Async::Work::poll".into(),
            drop_fn: "Demo::Async::Work::drop".into(),
            implicit_promotion: false,
            captured_arguments: Vec::new(),
            captured_locals: Vec::new(),
        });

        let perf = PerfMetadata::default();
        let builder = ModuleBuilder::new(
            &module,
            None,
            ChicKind::Executable,
            &[],
            &[],
            &[],
            None,
            &perf,
            false,
        )
        .expect("builder creation");
        let offset = builder
            .async_vtable_offsets
            .get("Demo::Async::Work")
            .copied()
            .expect("async vtable offset recorded");
        let segment = builder
            .data_segments
            .iter()
            .find(|seg| seg.offset == offset)
            .expect("async vtable data segment");
        assert_eq!(segment.bytes.len(), 8);
        let mut poll_bytes = [0u8; 4];
        poll_bytes.copy_from_slice(&segment.bytes[..4]);
        let mut drop_bytes = [0u8; 4];
        drop_bytes.copy_from_slice(&segment.bytes[4..8]);
        assert_eq!(
            u32::from_le_bytes(poll_bytes),
            *builder
                .index_by_name
                .get("Demo::Async::Work::poll")
                .expect("poll index")
        );
        assert_eq!(
            u32::from_le_bytes(drop_bytes),
            *builder
                .index_by_name
                .get("Demo::Async::Work::drop")
                .expect("drop index")
        );
    }
}
