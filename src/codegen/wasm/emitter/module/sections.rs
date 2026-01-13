use std::collections::HashSet;
use std::time::Instant;

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::{
    LINEAR_MEMORY_MIN_PAGES, STACK_BASE, ValueType, emitter::function::emit_function_with_async,
    ensure_u32, push_i32_const_expr, push_string, write_u32,
};
use crate::error::Error;
use crate::mir::FunctionKind;

use super::builder::ModuleBuilder;
use super::signature::ensure_supported_function;

#[derive(Debug)]
pub(crate) struct Section {
    id: u8,
    payload: Vec<u8>,
}

impl Section {
    pub(crate) fn new(id: u8, payload: Vec<u8>) -> Self {
        Self { id, payload }
    }

    #[cfg(test)]
    pub(crate) fn id(&self) -> u8 {
        self.id
    }

    #[cfg(test)]
    pub(crate) fn payload_bytes(&self) -> &[u8] {
        &self.payload
    }

    pub(crate) fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        out.push(self.id);
        let mut temp = Vec::new();
        let len = crate::codegen::wasm::ensure_u32(
            self.payload.len(),
            "section payload exceeds WebAssembly limits",
        )?;
        write_u32(&mut temp, len);
        out.extend_from_slice(&temp);
        out.extend_from_slice(&self.payload);
        Ok(())
    }
}

pub(super) struct ExportEntry {
    pub(super) name: String,
    pub(super) kind: u8,
    pub(super) index: u32,
}

impl<'a> ModuleBuilder<'a> {
    pub(crate) fn emit_type_section(&self) -> Result<Section, Error> {
        let mut payload = Vec::new();
        write_u32(
            &mut payload,
            ensure_u32(
                self.signatures.len(),
                "function type count exceeds WebAssembly limits",
            )?,
        );
        for sig in &self.signatures {
            payload.push(0x60); // func type
            write_u32(
                &mut payload,
                ensure_u32(
                    sig.params.len(),
                    "parameter count exceeds WebAssembly limits",
                )?,
            );
            for ty in &sig.params {
                payload.push(ty.to_byte());
            }
            write_u32(
                &mut payload,
                ensure_u32(sig.results.len(), "result count exceeds WebAssembly limits")?,
            );
            for ty in &sig.results {
                payload.push(ty.to_byte());
            }
        }
        Ok(Section::new(1, payload))
    }

    pub(crate) fn emit_import_section(&self) -> Result<Section, Error> {
        let mut payload = Vec::new();
        write_u32(
            &mut payload,
            ensure_u32(
                self.imports.len(),
                "import count exceeds WebAssembly limits",
            )?,
        );
        for import in &self.imports {
            push_string(&mut payload, import.module)?;
            push_string(&mut payload, import.name)?;
            payload.push(0x00); // function import kind
            write_u32(&mut payload, import.type_index);
        }
        Ok(Section::new(2, payload))
    }

    pub(crate) fn emit_function_section(&self) -> Result<Section, Error> {
        let mut payload = Vec::new();
        write_u32(
            &mut payload,
            ensure_u32(
                self.module.functions.len(),
                "function count exceeds WebAssembly limits",
            )?,
        );
        for &type_index in &self.function_type_indices {
            write_u32(&mut payload, type_index);
        }
        Ok(Section::new(3, payload))
    }

    pub(crate) fn emit_table_section(&self) -> Result<Option<Section>, Error> {
        if self.table_initializers.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        write_u32(&mut payload, 1); // table count
        payload.push(0x70); // funcref element type
        payload.push(0x00); // limits: min only
        write_u32(
            &mut payload,
            ensure_u32(
                self.table_initializers.len(),
                "function table size exceeds WebAssembly limits",
            )?,
        );
        Ok(Some(Section::new(4, payload)))
    }

    pub(crate) fn emit_memory_section(&self) -> Result<Option<Section>, Error> {
        let mut payload = Vec::new();
        write_u32(&mut payload, 1); // memory count
        payload.push(0x00); // min only
        write_u32(&mut payload, LINEAR_MEMORY_MIN_PAGES);
        Ok(Some(Section::new(5, payload)))
    }

    pub(crate) fn emit_global_section(&self) -> Result<Option<Section>, Error> {
        let mut payload = Vec::new();
        write_u32(&mut payload, 1); // global count
        payload.push(ValueType::I32.to_byte()); // i32 global
        payload.push(0x01); // mutable
        push_i32_const_expr(&mut payload, STACK_BASE as i32);
        Ok(Some(Section::new(6, payload)))
    }

    pub(crate) fn emit_export_section(&self) -> Result<Option<Section>, Error> {
        let mut exports = Vec::new();
        let mut seen = HashSet::new();
        if let Some(entry) = &self.entry
            && let Some(&index) = self.index_by_name.get(entry)
        {
            exports.push(ExportEntry {
                name: "chic_main".to_string(),
                kind: 0x00,
                index,
            });
            seen.insert("chic_main".to_string());
        }
        if matches!(
            self.kind,
            ChicKind::StaticLibrary | ChicKind::DynamicLibrary
        ) {
            for function in &self.module.functions {
                if let Some(&index) = self.index_by_name.get(&function.name) {
                    if !seen.insert(function.name.clone()) {
                        continue;
                    }
                    exports.push(ExportEntry {
                        name: function.name.clone(),
                        kind: 0x00,
                        index,
                    });
                }
            }
        }
        for function in &self.module.functions {
            if matches!(function.kind, FunctionKind::Testcase)
                && let Some(&index) = self.index_by_name.get(&function.name)
            {
                let symbol = format!("test::{}", function.name);
                if !seen.insert(symbol.clone()) {
                    continue;
                }
                exports.push(ExportEntry {
                    name: symbol,
                    kind: 0x00,
                    index,
                });
            }
        }
        for export in &self.module.exports {
            if let Some(&index) = self.index_by_name.get(&export.function) {
                if !seen.insert(export.symbol.clone()) {
                    continue;
                }
                exports.push(ExportEntry {
                    name: export.symbol.clone(),
                    kind: 0x00,
                    index,
                });
            }
        }
        if exports.is_empty() {
            Ok(None)
        } else {
            let mut payload = Vec::new();
            write_u32(
                &mut payload,
                ensure_u32(exports.len(), "export count exceeds WebAssembly limits")?,
            );
            for entry in exports {
                push_string(&mut payload, &entry.name)?;
                payload.push(entry.kind);
                write_u32(&mut payload, entry.index);
            }
            Ok(Some(Section::new(7, payload)))
        }
    }

    pub(crate) fn emit_element_section(&self) -> Result<Option<Section>, Error> {
        if self.table_initializers.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        write_u32(&mut payload, 1); // element segment count
        payload.push(0x00); // active segment targeting table 0
        push_i32_const_expr(&mut payload, 0);
        write_u32(
            &mut payload,
            ensure_u32(
                self.table_initializers.len(),
                "function table initialiser count exceeds WebAssembly limits",
            )?,
        );
        for &func_index in &self.table_initializers {
            write_u32(&mut payload, func_index);
        }
        Ok(Some(Section::new(9, payload)))
    }

    pub(crate) fn emit_code_section(&self) -> Result<Section, Error> {
        wasm_debug!(
            "emit_code_section: lowering {} functions",
            self.module.functions.len()
        );
        let mut payload = Vec::new();
        write_u32(
            &mut payload,
            ensure_u32(
                self.module.functions.len(),
                "function count exceeds WebAssembly limits",
            )?,
        );
        for (function_index, function) in self.module.functions.iter().enumerate() {
            let fn_start = Instant::now();
            wasm_debug!("  function `{}`: ensure supported", function.name);
            ensure_supported_function(function)?;
            let emit_body_start = Instant::now();
            let tracepoint = self.tracepoints.get(function.name.as_str()).copied();
            let body = emit_function_with_async(
                function,
                &self.index_by_name,
                &self.function_return_tys,
                Some(&self.function_param_tys),
                &self.module.type_layouts,
                Some(&self.string_literals),
                Some(&self.trait_vtable_offsets),
                Some(&self.class_vtable_offsets),
                Some(&self.async_vtable_offsets),
                Some(&self.signature_indices),
                Some(&self.function_signatures),
                &self.module.trait_vtables,
                &self.module.class_vtables,
                Some(&self.module.statics),
                Some(&self.static_offsets),
                tracepoint,
                self.coverage_enabled,
                ensure_u32(
                    function_index,
                    "coverage function index exceeds wasm limits",
                )?,
            )?;
            wasm_debug!(
                "  function `{}`: emit_body completed in {:?}",
                function.name,
                emit_body_start.elapsed()
            );
            write_u32(
                &mut payload,
                ensure_u32(body.len(), "code body exceeds WebAssembly limits")?,
            );
            payload.extend_from_slice(&body);
            wasm_debug!(
                "  function `{}`: body emitted ({} bytes)",
                function.name,
                body.len()
            );
            wasm_debug!(
                "  function `{}`: total lowering time {:?}",
                function.name,
                fn_start.elapsed()
            );
        }
        Ok(Section::new(10, payload))
    }

    pub(crate) fn emit_data_section(&self) -> Result<Option<Section>, Error> {
        if self.data_segments.is_empty() {
            return Ok(None);
        }
        let mut payload = Vec::new();
        write_u32(
            &mut payload,
            ensure_u32(
                self.data_segments.len(),
                "data segment count exceeds WebAssembly limits",
            )?,
        );
        for segment in &self.data_segments {
            payload.push(0x00); // active segment
            push_i32_const_expr(&mut payload, segment.offset as i32);
            write_u32(
                &mut payload,
                ensure_u32(
                    segment.bytes.len(),
                    "data segment size exceeds WebAssembly limits",
                )?,
            );
            payload.extend_from_slice(&segment.bytes);
        }
        Ok(Some(Section::new(11, payload)))
    }
}
