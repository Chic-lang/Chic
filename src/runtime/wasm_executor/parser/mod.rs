mod diagnostics;
mod lexer;
mod state;
#[cfg(test)]
mod tests;
use std::collections::HashMap;

use super::errors::WasmExecutionError;
use super::module::{
    DataSegment, Function, GlueRecord, InterfaceDefault, Module, Table, TableElementType,
    TypeMetadataRecord, TypeVarianceRecord,
};
use super::types::{FuncType, ValueType};
use super::{WASM_MAGIC, WASM_VERSION};
use lexer::{read_init_expr, read_string, read_u64, read_uleb, read_value_type};
use state::parse_instructions;

#[expect(
    clippy::too_many_lines,
    reason = "Bootstrap parser keeps the initial WASM slice traversal in one routine for clarity."
)]
/// Parse raw WebAssembly bytes into a Chic runtime module description.
///
/// # Errors
/// Returns `WasmExecutionError` when the payload is malformed or relies on unsupported features.
pub fn parse_module(bytes: &[u8]) -> Result<Module, WasmExecutionError> {
    if bytes.len() < 8
        || !bytes.starts_with(WASM_MAGIC.as_slice())
        || bytes.get(4..8) != Some(WASM_VERSION.as_slice())
    {
        return Err(WasmExecutionError {
            message: "invalid wasm header".into(),
        });
    }

    let mut cursor = 8usize;
    let mut types = Vec::new();
    let mut func_type_indices = Vec::new();
    let mut functions = Vec::new();
    let mut tables = Vec::new();
    let mut imports = Vec::new();
    let mut exports = HashMap::new();
    let mut code_bodies: Vec<Vec<u8>> = Vec::new();
    let mut memory_min_pages: Option<u32> = None;
    let mut globals = Vec::new();
    let mut data_segments = Vec::new();
    let mut interface_defaults = Vec::new();
    let mut type_metadata = Vec::new();
    let mut hash_glue = Vec::new();
    let mut eq_glue = Vec::new();
    let mut defined_function_names: Vec<String> = Vec::new();

    while cursor < bytes.len() {
        let section_id = bytes[cursor];
        cursor += 1;
        let section_size = read_uleb(bytes, &mut cursor)? as usize;
        let end = cursor + section_size;
        if end > bytes.len() {
            return Err(WasmExecutionError {
                message: "section length exceeds module size".into(),
            });
        }
        match section_id {
            1 => {
                let type_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..type_count {
                    let form = bytes
                        .get(cursor)
                        .copied()
                        .ok_or_else(|| WasmExecutionError {
                            message: "unexpected end of type section".into(),
                        })?;
                    cursor += 1;
                    if form != 0x60 {
                        return Err(WasmExecutionError {
                            message: "unsupported function type form".into(),
                        });
                    }
                    let param_count = read_uleb(bytes, &mut cursor)?;
                    let mut params = Vec::new();
                    for _ in 0..param_count {
                        params.push(read_value_type(bytes, &mut cursor)?);
                    }
                    let result_count = read_uleb(bytes, &mut cursor)?;
                    let mut results = Vec::new();
                    for _ in 0..result_count {
                        results.push(read_value_type(bytes, &mut cursor)?);
                    }
                    types.push(FuncType { params, results });
                }
            }
            3 => {
                let func_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..func_count {
                    func_type_indices.push(read_uleb(bytes, &mut cursor)?);
                }
            }
            4 => {
                let table_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..table_count {
                    let element_type =
                        bytes
                            .get(cursor)
                            .copied()
                            .ok_or_else(|| WasmExecutionError {
                                message: "unexpected end of table section".into(),
                            })?;
                    cursor += 1;
                    if element_type != 0x70 {
                        return Err(WasmExecutionError {
                            message: "only funcref tables are supported".into(),
                        });
                    }
                    let flags = read_uleb(bytes, &mut cursor)?;
                    if flags & !0x01 != 0 {
                        return Err(WasmExecutionError {
                            message: "unsupported table limits flag".into(),
                        });
                    }
                    let min = read_uleb(bytes, &mut cursor)?;
                    let max = if flags & 0x01 != 0 {
                        Some(read_uleb(bytes, &mut cursor)?)
                    } else {
                        None
                    };
                    let min = min as u32;
                    let max = max.map(|value| value as u32);
                    let mut elements = Vec::new();
                    elements.resize(min as usize, None);
                    tables.push(Table {
                        element_type: TableElementType::FuncRef,
                        min,
                        max,
                        elements,
                    });
                }
            }
            2 => {
                let import_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..import_count {
                    let module = read_string(bytes, &mut cursor)?;
                    let name = read_string(bytes, &mut cursor)?;
                    let kind = bytes
                        .get(cursor)
                        .copied()
                        .ok_or_else(|| WasmExecutionError {
                            message: "unexpected end of import section".into(),
                        })?;
                    cursor += 1;
                    if kind != 0x00 {
                        return Err(WasmExecutionError {
                            message: "only function imports are supported".into(),
                        });
                    }
                    let type_index = read_uleb(bytes, &mut cursor)?;
                    imports.push(super::module::Import {
                        module,
                        name,
                        type_index,
                    });
                }
            }
            5 => {
                let memory_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..memory_count {
                    let flags = read_uleb(bytes, &mut cursor)?;
                    let min = read_uleb(bytes, &mut cursor)?;
                    if flags & 0x1 != 0 {
                        let _max = read_uleb(bytes, &mut cursor)?;
                    }
                    memory_min_pages = Some(min);
                }
            }
            6 => {
                let global_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..global_count {
                    let ty = read_value_type(bytes, &mut cursor)?;
                    let mutable = match bytes.get(cursor).copied() {
                        Some(0) => {
                            cursor += 1;
                            false
                        }
                        Some(1) => {
                            cursor += 1;
                            true
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "unsupported global mutability flag".into(),
                            });
                        }
                    };
                    let initial = read_init_expr(bytes, &mut cursor, ty)?;
                    globals.push(super::module::Global {
                        ty,
                        mutable,
                        initial,
                    });
                }
            }
            7 => {
                let export_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..export_count {
                    let name = read_string(bytes, &mut cursor)?;
                    let kind = bytes
                        .get(cursor)
                        .copied()
                        .ok_or_else(|| WasmExecutionError {
                            message: "unexpected end of export section".into(),
                        })?;
                    cursor += 1;
                    if kind != 0x00 {
                        return Err(WasmExecutionError {
                            message: "only function exports are supported".into(),
                        });
                    }
                    let index = read_uleb(bytes, &mut cursor)?;
                    exports.insert(name, index);
                }
            }
            9 => {
                let segment_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..segment_count {
                    let flags = read_uleb(bytes, &mut cursor)?;
                    if flags != 0 {
                        return Err(WasmExecutionError {
                            message: "unsupported element segment flags".into(),
                        });
                    }
                    let offset_value =
                        read_init_expr(bytes, &mut cursor, ValueType::I32)?.as_i32()?;
                    if offset_value < 0 {
                        return Err(WasmExecutionError {
                            message: "element segment offset cannot be negative".into(),
                        });
                    }
                    let offset = offset_value as usize;
                    let func_count = read_uleb(bytes, &mut cursor)? as usize;
                    if tables.is_empty() {
                        return Err(WasmExecutionError {
                            message: "element segment references missing table".into(),
                        });
                    }
                    let table = tables.get_mut(0).ok_or_else(|| WasmExecutionError {
                        message:
                            "element segment references table index 0 but no tables were declared"
                                .into(),
                    })?;
                    if table.elements.len() < offset + func_count {
                        table.elements.resize(offset + func_count, None);
                    }
                    for index in 0..func_count {
                        let func_index = read_uleb(bytes, &mut cursor)?;
                        let slot = offset + index;
                        if slot >= table.elements.len() {
                            return Err(WasmExecutionError {
                                message: "element initializer exceeds table bounds".into(),
                            });
                        }
                        table.elements[slot] = Some(func_index);
                    }
                }
            }
            10 => {
                let body_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..body_count {
                    let body_size = read_uleb(bytes, &mut cursor)? as usize;
                    if cursor + body_size > bytes.len() {
                        return Err(WasmExecutionError {
                            message: "function body exceeds module size".into(),
                        });
                    }
                    code_bodies.push(bytes[cursor..cursor + body_size].to_vec());
                    cursor += body_size;
                }
            }
            11 => {
                let segment_count = read_uleb(bytes, &mut cursor)?;
                for _ in 0..segment_count {
                    let flags = read_uleb(bytes, &mut cursor)? as u32;
                    let (memory_index, offset) = match flags {
                        0 => {
                            let offset_value =
                                read_init_expr(bytes, &mut cursor, ValueType::I32)?.as_i32()?;
                            if offset_value < 0 {
                                return Err(WasmExecutionError {
                                    message: "data segment offset cannot be negative".into(),
                                });
                            }
                            (0u32, offset_value as u32)
                        }
                        2 => {
                            let mem = read_uleb(bytes, &mut cursor)? as u32;
                            let offset_value =
                                read_init_expr(bytes, &mut cursor, ValueType::I32)?.as_i32()?;
                            if offset_value < 0 {
                                return Err(WasmExecutionError {
                                    message: "data segment offset cannot be negative".into(),
                                });
                            }
                            (mem, offset_value as u32)
                        }
                        1 | 3 => {
                            return Err(WasmExecutionError {
                                message: "passive data segments are not supported".into(),
                            });
                        }
                        other => {
                            return Err(WasmExecutionError {
                                message: format!("unsupported data segment flags {other}"),
                            });
                        }
                    };
                    if memory_index != 0 {
                        return Err(WasmExecutionError {
                            message: "data segment references unsupported memory index".into(),
                        });
                    }
                    let byte_len = read_uleb(bytes, &mut cursor)? as usize;
                    if cursor + byte_len > bytes.len() {
                        return Err(WasmExecutionError {
                            message: "data segment exceeds module size".into(),
                        });
                    }
                    let mut init = Vec::with_capacity(byte_len);
                    init.extend_from_slice(&bytes[cursor..cursor + byte_len]);
                    cursor += byte_len;
                    data_segments.push(DataSegment {
                        offset,
                        bytes: init,
                    });
                }
            }
            0 => {
                let name = read_string(bytes, &mut cursor)?;
                match name.as_str() {
                    "chic.fn.names" => {
                        let entry_count = read_uleb(bytes, &mut cursor)? as usize;
                        defined_function_names = Vec::with_capacity(entry_count);
                        for _ in 0..entry_count {
                            defined_function_names.push(read_string(bytes, &mut cursor)?);
                        }
                    }
                    "chic.iface.defaults" => {
                        let entry_count = read_uleb(bytes, &mut cursor)?;
                        for _ in 0..entry_count {
                            let implementer = read_string(bytes, &mut cursor)?;
                            let interface = read_string(bytes, &mut cursor)?;
                            let method = read_string(bytes, &mut cursor)?;
                            let function_index = read_uleb(bytes, &mut cursor)? as u32;
                            interface_defaults.push(InterfaceDefault {
                                implementer,
                                interface,
                                method,
                                function_index,
                            });
                        }
                    }
                    "chic.type.metadata" => {
                        let entry_count = read_uleb(bytes, &mut cursor)?;
                        for _ in 0..entry_count {
                            let type_id = read_u64(bytes, &mut cursor)?;
                            let size = read_uleb(bytes, &mut cursor)?;
                            let align = read_uleb(bytes, &mut cursor)?;
                            let variance_len = read_uleb(bytes, &mut cursor)? as usize;
                            let mut variance = Vec::with_capacity(variance_len);
                            for _ in 0..variance_len {
                                let byte = bytes.get(cursor).copied().ok_or_else(|| {
                                    WasmExecutionError {
                                        message: "variance entry exceeds metadata section size"
                                            .into(),
                                    }
                                })?;
                                cursor += 1;
                                let tag = match byte {
                                    0 => TypeVarianceRecord::Invariant,
                                    1 => TypeVarianceRecord::Covariant,
                                    2 => TypeVarianceRecord::Contravariant,
                                    other => {
                                        // Unknown tags default to invariant to keep execution going.
                                        tracing::warn!(
                                            "metadata.variance: unknown tag `{other}`, defaulting to invariant"
                                        );
                                        TypeVarianceRecord::Invariant
                                    }
                                };
                                variance.push(tag);
                            }
                            // Flags are currently metadata-only, but still consume the encoded value
                            // to keep cursor alignment with the encoded payload.
                            let _flags = read_uleb(bytes, &mut cursor)?;
                            type_metadata.push(TypeMetadataRecord {
                                type_id,
                                size,
                                align,
                                variance,
                            });
                        }
                    }
                    "chic.hash.glue" => {
                        let entry_count = read_uleb(bytes, &mut cursor)?;
                        for _ in 0..entry_count {
                            let type_id = read_u64(bytes, &mut cursor)?;
                            let function_index = read_uleb(bytes, &mut cursor)? as u32;
                            hash_glue.push(GlueRecord {
                                type_id,
                                function_index,
                            });
                        }
                    }
                    "chic.eq.glue" => {
                        let entry_count = read_uleb(bytes, &mut cursor)?;
                        for _ in 0..entry_count {
                            let type_id = read_u64(bytes, &mut cursor)?;
                            let function_index = read_uleb(bytes, &mut cursor)? as u32;
                            eq_glue.push(GlueRecord {
                                type_id,
                                function_index,
                            });
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        cursor = end;
    }

    if func_type_indices.len() != code_bodies.len() {
        return Err(WasmExecutionError {
            message: "function and code counts mismatch".into(),
        });
    }

    for (type_index, body) in func_type_indices.into_iter().zip(code_bodies.into_iter()) {
        let mut cursor = 0usize;
        let locals_count = read_uleb(&body, &mut cursor)?;
        let mut locals = Vec::new();
        for _ in 0..locals_count {
            let count = read_uleb(&body, &mut cursor)?;
            let ty = read_value_type(&body, &mut cursor)?;
            for _ in 0..count {
                locals.push(ty);
            }
        }
        let instructions = parse_instructions(&body[cursor..])?;
        functions.push(Function {
            type_index,
            locals,
            code: instructions,
        });
    }

    let mut function_names = Vec::with_capacity(imports.len() + functions.len());
    for import in &imports {
        function_names.push(format!("{}.{}", import.module, import.name));
    }
    if defined_function_names.len() == functions.len() {
        function_names.extend(defined_function_names);
    }

    Ok(Module {
        types,
        imports,
        functions,
        function_names,
        tables,
        exports,
        memory_min_pages,
        globals,
        data_segments,
        interface_defaults,
        type_metadata,
        hash_glue,
        eq_glue,
    })
}
