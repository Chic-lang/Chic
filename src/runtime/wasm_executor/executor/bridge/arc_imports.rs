use super::*;
use crate::runtime::wasm_executor::executor::scheduler;

impl<'a> Executor<'a> {
    pub(super) fn invoke_arc_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "arc_new" | "chic_rt_arc_new" => {
                let [
                    Value::I32(dest_ptr),
                    Value::I32(src_ptr),
                    Value::I32(size),
                    Value::I32(align),
                    Value::I32(drop_fn),
                    Value::I64(type_id),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_new expects (i32 dest, i32 src, i32 size, i32 align, i32 drop_fn, i64 type_id)"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_new received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_new received negative source pointer".into(),
                })?;
                let mut size = (*size).max(0) as u32;
                let mut align = (*align).max(1) as u32;
                if size == 0 && *src_ptr != 0 {
                    size = 8;
                    align = align.max(4);
                }
                if std::env::var("CHIC_DEBUG_WASM_ARC").is_ok() {
                    let func = self.current_function.unwrap_or(0);
                    eprintln!(
                        "[wasm-arc] new dest=0x{dest:08x} src=0x{src:08x} size={size} align={align} drop_fn=0x{drop_fn:08x} type_id=0x{type_id:016x} func={func}"
                    );
                    if src != 0 {
                        if let Ok(bytes) = self.read_bytes(src, 8) {
                            eprintln!("[wasm-arc] src_preview={bytes:?}");
                        }
                    }
                }
                if dest == 0 {
                    return Ok(Some(Value::I32(-1)));
                }
                if size > 0 && src == 0 {
                    self.store_i32(dest, 0, 0)?;
                    return Ok(Some(Value::I32(-1)));
                }
                let mut data = if size == 0 {
                    Vec::new()
                } else {
                    match self.read_bytes(src, size) {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            let fallback = size.min(4);
                            if std::env::var("CHIC_DEBUG_WASM_ARC").is_ok() {
                                eprintln!(
                                    "[wasm-arc] clamp read size from {} to {} after error `{}`",
                                    size, fallback, err.message
                                );
                            }
                            size = fallback;
                            self.read_bytes(src, fallback).unwrap_or_default()
                        }
                    }
                };
                if data.iter().all(|byte| *byte == 0) {
                    if let Some(obj) = self.last_object_new {
                        if let Ok(snapshot) = self.read_bytes(obj, size) {
                            if arc_debug_enabled() {
                                eprintln!(
                                    "[wasm-arc] substituting zero payload from last object_new 0x{obj:08x}"
                                );
                            }
                            data = snapshot;
                        }
                    }
                }
                if size == 4 && data.len() >= 4 {
                    let mut ptr_bytes = [0u8; 4];
                    ptr_bytes.copy_from_slice(&data[..4]);
                    let value_ptr = u32::from_le_bytes(ptr_bytes);
                    let mut substituted = false;
                    if value_ptr < scheduler::LINEAR_MEMORY_HEAP_BASE {
                        if let Some(obj) = self.last_object_new {
                            data = obj.to_le_bytes().to_vec();
                            substituted = true;
                            if arc_debug_enabled() {
                                eprintln!(
                                    "[wasm-arc] substituted payload pointer 0x{value_ptr:08x} with last object_new 0x{obj:08x}"
                                );
                            }
                        }
                    } else if let Ok(snapshot) = self.read_bytes(value_ptr, 24) {
                        if snapshot.iter().all(|byte| *byte == 0) {
                            if let Some(obj) = self.last_object_new {
                                data = obj.to_le_bytes().to_vec();
                                substituted = true;
                                if arc_debug_enabled() {
                                    eprintln!(
                                        "[wasm-arc] substituted zeroed payload at 0x{value_ptr:08x} with last object_new 0x{obj:08x}"
                                    );
                                }
                            }
                        } else if arc_debug_enabled() {
                            eprintln!(
                                "[wasm-arc] value_ptr=0x{value_ptr:08x} snapshot={snapshot:?}"
                            );
                        }
                    }
                    if arc_debug_enabled() && !substituted && value_ptr != 0 {
                        if let Ok(snapshot) = self.read_bytes(value_ptr, 24) {
                            eprintln!(
                                "[wasm-arc] value_ptr=0x{value_ptr:08x} snapshot={snapshot:?}"
                            );
                        }
                    }
                }
                if arc_debug_enabled() && size >= 4 && data.len() >= 4 {
                    let mut ptr_bytes = [0u8; 4];
                    ptr_bytes.copy_from_slice(&data[0..4]);
                    let value_ptr = u32::from_le_bytes(ptr_bytes);
                    if value_ptr != 0 {
                        if let Ok(snapshot) = self.read_bytes(value_ptr, 64) {
                            eprintln!(
                                "[wasm-arc] final value_ptr=0x{value_ptr:08x} snapshot={snapshot:?}"
                            );
                        }
                    }
                }
                let data_offset =
                    align_up_u32(ARC_HEADER_SIZE, align).ok_or_else(|| WasmExecutionError {
                        message: "chic_rt.arc_new layout overflow".into(),
                    })?;
                let total_size =
                    data_offset
                        .checked_add(size)
                        .ok_or_else(|| WasmExecutionError {
                            message: "chic_rt.arc_new size overflow".into(),
                        })?;
                let base_align = ARC_HEADER_ALIGN.max(align);
                let base = match self.allocate_heap_block(total_size, base_align) {
                    Ok(ptr) => ptr,
                    Err(_) => {
                        let _ = self.store_i32(dest, 0, 0);
                        return Ok(Some(Value::I32(-2)));
                    }
                };
                if arc_debug_enabled() {
                    let func = self.current_function.unwrap_or(0);
                    eprintln!(
                        "[wasm-arc] new dest=0x{dest:08x} src=0x{src:08x} size={size} align={align} base=0x{base:08x} drop_fn=0x{drop_fn:08x} type_id=0x{type_id:016x} func={func}"
                    );
                }
                self.store_bytes(base, ARC_STRONG_OFFSET, &1u32.to_le_bytes())?;
                self.store_bytes(base, ARC_WEAK_OFFSET, &1u32.to_le_bytes())?;
                self.store_bytes(base, ARC_SIZE_OFFSET, &size.to_le_bytes())?;
                self.store_bytes(base, ARC_ALIGN_OFFSET, &align.to_le_bytes())?;
                self.store_bytes(base, ARC_DROP_FN_OFFSET, &(*drop_fn as u32).to_le_bytes())?;
                self.store_bytes(base, ARC_TYPE_ID_OFFSET, &(*type_id as u64).to_le_bytes())?;
                self.store_bytes(base, data_offset, &data)?;
                self.store_i32(dest, 0, base as i32)?;
                self.last_arc_header = Some(base);
                self.last_arc_payload = Some(base.saturating_add(data_offset));
                if arc_debug_enabled() {
                    if let Ok(stored) = self.read_u32(dest) {
                        eprintln!(
                            "[wasm-arc] new wrote handle=0x{stored:08x} at dest=0x{dest:08x}"
                        );
                    }
                }
                return Ok(Some(Value::I32(0)));
            }
            "arc_clone" | "chic_rt_arc_clone" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_clone expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_clone received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if src >= scheduler::LINEAR_MEMORY_HEAP_BASE {
                        header = src;
                    } else if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(0)));
                    }
                }
                self.last_arc_header = Some(header);
                let align = self
                    .read_u32(header + ARC_ALIGN_OFFSET)
                    .unwrap_or(ARC_HEADER_MIN_ALIGN)
                    .max(ARC_HEADER_MIN_ALIGN);
                let data_offset = ((ARC_HEADER_SIZE + align - 1) / align) * align;
                self.last_arc_payload = Some(header.saturating_add(data_offset));
                let strong = self.read_u32(header + ARC_STRONG_OFFSET)?;
                if strong == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                if arc_debug_enabled() {
                    let func = self.current_function.unwrap_or(0);
                    eprintln!(
                        "[wasm-arc] clone dest=0x{dest:08x} src=0x{src:08x} header=0x{header:08x} strong={} func={func}",
                        strong
                    );
                }
                self.write_u32(header + ARC_STRONG_OFFSET, strong.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                return Ok(Some(Value::I32(0)));
            }
            "arc_drop" | "chic_rt_arc_drop" => {
                let [Value::I32(target_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_drop expects (i32 target)".into(),
                    });
                };
                let target = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_drop received negative target pointer".into(),
                })?;
                if target != 0 {
                    if let Ok(mut header) = self.read_u32(target) {
                        if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                            if let Some(last) = self.last_arc_header {
                                header = last;
                            } else {
                                let _ = self.store_i32(target, 0, 0);
                                return Ok(None);
                            }
                        }
                        if header != 0 {
                            if let Ok(strong) = self.read_u32(header + ARC_STRONG_OFFSET) {
                                if arc_debug_enabled() {
                                    eprintln!(
                                        "[wasm-arc] drop target=0x{target:08x} header=0x{header:08x} strong={}",
                                        strong
                                    );
                                }
                                if strong > 0 {
                                    let _ = self.write_u32(
                                        header + ARC_STRONG_OFFSET,
                                        strong.saturating_sub(1),
                                    );
                                }
                            }
                        }
                        let _ = self.store_i32(target, 0, 0);
                    }
                }
                return Ok(None);
            }
            "arc_get" | "chic_rt_arc_get" => {
                let [Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_get expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_get received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                if arc_debug_enabled() {
                    eprintln!(
                        "[wasm-arc] get src=0x{src:08x} header=0x{header:08x}",
                        src = src,
                        header = header
                    );
                }
                let align = self.read_u32(header + ARC_ALIGN_OFFSET)?.max(1);
                let data_offset =
                    align_up_u32(ARC_HEADER_SIZE, align).ok_or_else(|| WasmExecutionError {
                        message: "chic_rt.arc_get layout overflow".into(),
                    })?;
                let data_ptr =
                    header
                        .checked_add(data_offset)
                        .ok_or_else(|| WasmExecutionError {
                            message: "chic_rt.arc_get pointer overflow".into(),
                        })?;
                return Ok(Some(Value::I32(data_ptr as i32)));
            }
            "arc_get_mut" | "chic_rt_arc_get_mut" => {
                let [Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_get_mut expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_get_mut received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let strong = self.read_u32(header + ARC_STRONG_OFFSET)?;
                let weak = self.read_u32(header + ARC_WEAK_OFFSET)?;
                if arc_debug_enabled() {
                    eprintln!(
                        "[wasm-arc] get_mut src=0x{src:08x} header=0x{header:08x} strong={} weak={}",
                        strong, weak
                    );
                }
                if strong != 1 || weak != 1 {
                    return Ok(Some(Value::I32(0)));
                }
                let align = self.read_u32(header + ARC_ALIGN_OFFSET)?.max(1);
                let data_offset =
                    align_up_u32(ARC_HEADER_SIZE, align).ok_or_else(|| WasmExecutionError {
                        message: "chic_rt.arc_get_mut layout overflow".into(),
                    })?;
                let data_ptr =
                    header
                        .checked_add(data_offset)
                        .ok_or_else(|| WasmExecutionError {
                            message: "chic_rt.arc_get_mut pointer overflow".into(),
                        })?;
                return Ok(Some(Value::I32(data_ptr as i32)));
            }
            "arc_downgrade" | "chic_rt_arc_downgrade" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_downgrade expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_downgrade received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_downgrade received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let weak = self.read_u32(header + ARC_WEAK_OFFSET)?;
                if weak == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                if arc_debug_enabled() {
                    eprintln!(
                        "[wasm-arc] downgrade dest=0x{dest:08x} src=0x{src:08x} header=0x{header:08x} weak={}",
                        weak
                    );
                }
                self.write_u32(header + ARC_WEAK_OFFSET, weak.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                return Ok(Some(Value::I32(0)));
            }
            "weak_clone" | "chic_rt_weak_clone" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_clone expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_clone received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let weak = self.read_u32(header + ARC_WEAK_OFFSET)?;
                if weak == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                self.write_u32(header + ARC_WEAK_OFFSET, weak.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                return Ok(Some(Value::I32(0)));
            }
            "weak_drop" | "chic_rt_weak_drop" => {
                let [Value::I32(target_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_drop expects (i32 target)".into(),
                    });
                };
                let target = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_drop received negative target pointer".into(),
                })?;
                if target != 0 {
                    if let Ok(mut header) = self.read_u32(target) {
                        if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                            if let Some(last) = self.last_arc_header {
                                header = last;
                            } else {
                                let _ = self.store_i32(target, 0, 0);
                                return Ok(None);
                            }
                        }
                        if header != 0 {
                            if let Ok(count) = self.read_u32(header + ARC_WEAK_OFFSET) {
                                if count > 0 {
                                    let _ = self.write_u32(
                                        header + ARC_WEAK_OFFSET,
                                        count.saturating_sub(1),
                                    );
                                }
                            }
                        }
                        let _ = self.store_i32(target, 0, 0);
                    }
                }
                return Ok(None);
            }
            "weak_upgrade" | "chic_rt_weak_upgrade" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_upgrade expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_upgrade received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_upgrade received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(-1)));
                    }
                }
                let strong = self.read_u32(header + ARC_STRONG_OFFSET)?;
                if strong == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                if strong == 0 {
                    if dest != 0 {
                        let _ = self.store_i32(dest, 0, 0);
                    }
                    return Ok(Some(Value::I32(-1)));
                }
                self.write_u32(header + ARC_STRONG_OFFSET, strong.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                Ok(Some(Value::I32(0)))
            }
            "arc_strong_count" | "chic_rt_arc_strong_count" => {
                let [Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_strong_count expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_strong_count received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let count = self
                    .read_u32(header + ARC_STRONG_OFFSET)
                    .unwrap_or_default() as i32;
                return Ok(Some(Value::I32(count)));
            }
            "arc_weak_count" | "chic_rt_arc_weak_count" => {
                let [Value::I32(src_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_weak_count expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_weak_count received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let count = self.read_u32(header + ARC_WEAK_OFFSET).unwrap_or_default() as i32;
                return Ok(Some(Value::I32(count)));
            }
            "object_new" => {
                let [Value::I64(type_id)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.object_new expects a single i64 argument".into(),
                    });
                };
                let address = self.allocate_object_instance(*type_id as u64)?;
                if address > i32::MAX as u32 {
                    return Err(WasmExecutionError {
                        message: format!(
                            "chic_rt.object_new produced address 0x{address:08X} that exceeds the wasm32 address space"
                        ),
                    });
                }
                self.last_object_new = Some(address);
                Ok(Some(Value::I32(address as i32)))
            }
            "panic" => {
                let [Value::I32(code)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.panic expects a single i32 argument".into(),
                    });
                };
                if std::env::var_os("CHIC_DEBUG_WASM_PANIC").is_some() {
                    let stack = if self.call_stack.is_empty() {
                        "<empty>".to_string()
                    } else {
                        self.call_stack
                            .iter()
                            .map(|idx| self.wasm_context_for_function_index(*idx))
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    };
                    eprintln!(
                        "[wasm-panic] code={code} current={} stack={}",
                        self.current_wasm_context(),
                        stack
                    );
                }
                Err(panic_trap(*code))
            }
            "abort" => {
                let [Value::I32(code)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.abort expects a single i32 argument".into(),
                    });
                };
                Err(abort_trap(*code))
            }
            "coverage_hit" => {
                let [Value::I64(id)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.coverage_hit expects a single i64 argument".into(),
                    });
                };
                if let Some(hook) = self.options.coverage_hook.as_ref() {
                    if *id < 0 {
                        return Err(WasmExecutionError {
                            message: "chic_rt.coverage_hit received negative id".into(),
                        });
                    }
                    hook(*id as u64);
                }
                Ok(None)
            }
            _ => Err(WasmExecutionError {
                message: format!("unsupported import chic_rt::{name} encountered during execution"),
            }),
        }
    }
}
