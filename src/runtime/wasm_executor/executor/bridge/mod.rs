#[cfg(test)]
use crate::mmio::AddressSpaceId;
use crate::mmio::{InvalidWidthError, decode_flags, decode_value, encode_value};
use crate::runtime::float_env::rounding_mode;
use crate::runtime::span::SpanError;
use crate::runtime::tracing::{chic_rt_trace_enter, chic_rt_trace_exit, chic_rt_trace_flush};
use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::module::Import;
use crate::runtime::wasm_executor::types::{Value, ValueType};

use super::AwaitStatus;
use super::runtime::{
    BorrowRuntimeKey, BorrowRuntimeKind, BorrowRuntimeRecord, STRING_EMPTY_PTR,
    STRING_INLINE_CAPACITY, STRING_INLINE_TAG, WasmHashMapIterRepr, WasmHashMapRepr,
    WasmHashSetIterRepr, WasmHashSetRepr, WasmStringRepr, WasmVecRepr,
};
use super::scheduler::{AsyncNode, Executor, SchedulerTracer};
use super::traps::{abort_trap, panic_trap};
use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const FUTURE_FLAG_READY: u32 = 0x0000_0001;
const FUTURE_FLAG_COMPLETED: u32 = 0x0000_0002;
const FUTURE_FLAG_CANCELLED: u32 = 0x0000_0004;
const FUTURE_FLAG_FAULTED: u32 = 0x8000_0000;

static HASHMAP_DEBUG_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TYPECALL_DEBUG_COUNTER: AtomicUsize = AtomicUsize::new(0);
static HASHSET_DEBUG_COUNTER: AtomicUsize = AtomicUsize::new(0);

const ARC_HEADER_SIZE: u32 = 32;
const ARC_HEADER_ALIGN: u32 = 8;
const ARC_STRONG_OFFSET: u32 = 0;
const ARC_WEAK_OFFSET: u32 = 4;
const ARC_SIZE_OFFSET: u32 = 8;
const ARC_ALIGN_OFFSET: u32 = 12;
const ARC_DROP_FN_OFFSET: u32 = 16;
const ARC_TYPE_ID_OFFSET: u32 = 24;
const ARC_HEADER_MIN_ALIGN: u32 = 1;
const STRING_SUCCESS: i32 = 0;
const STRING_UTF8: i32 = 1;
const STRING_CAPACITY_OVERFLOW: i32 = 2;
const STRING_ALLOCATION_FAILED: i32 = 3;
const STRING_INVALID_POINTER: i32 = 4;
const STRING_OUT_OF_BOUNDS: i32 = 5;
const SPAN_DANGLING_PTR: u32 = 1;
const TABLE_STATE_EMPTY: u8 = 0;
const TABLE_STATE_FULL: u8 = 1;
const TABLE_STATE_TOMBSTONE: u8 = 2;
const TABLE_MIN_CAPACITY: u32 = 8;
const TABLE_LOAD_NUM: u64 = 7;
const TABLE_LOAD_DEN: u64 = 10;
const TABLE_SUCCESS: i32 = 0;
#[allow(dead_code)]
const TABLE_ALLOCATION_FAILED: i32 = 1;
const TABLE_INVALID_POINTER: i32 = 2;
const TABLE_CAPACITY_OVERFLOW: i32 = 3;
const TABLE_NOT_FOUND: i32 = 4;
const TABLE_ITERATION_COMPLETE: i32 = 5;

fn align_up_u32(value: u32, align: u32) -> Option<u32> {
    if align <= 1 {
        return Some(value);
    }
    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}

fn value_as_u32(value: &Value, context: &str) -> Result<u32, WasmExecutionError> {
    match value {
        Value::I32(v) => u32::try_from(*v).map_err(|_| WasmExecutionError {
            message: format!("{context} received negative value"),
        }),
        Value::I64(v) => u32::try_from(*v).map_err(|_| WasmExecutionError {
            message: format!("{context} received out-of-range value"),
        }),
        _ => Err(WasmExecutionError {
            message: format!("{context} expected integer argument"),
        }),
    }
}

fn value_as_ptr_u32(value: &Value, context: &str) -> Result<u32, WasmExecutionError> {
    match value {
        Value::I32(v) => u32::try_from(*v).map_err(|_| WasmExecutionError {
            message: format!("{context} received negative pointer ({v})"),
        }),
        Value::I64(v) => u32::try_from(*v).map_err(|_| WasmExecutionError {
            message: format!("{context} received out-of-range value"),
        }),
        _ => Err(WasmExecutionError {
            message: format!("{context} expected integer argument"),
        }),
    }
}

fn value_as_i64(value: &Value, context: &str) -> Result<i64, WasmExecutionError> {
    match value {
        Value::I32(v) => Ok(i64::from(*v)),
        Value::I64(v) => Ok(*v),
        _ => Err(WasmExecutionError {
            message: format!("{context} expected integer argument"),
        }),
    }
}

fn span_validate_stride(elem_size: u32, elem_align: u32) -> SpanError {
    if elem_size == 0 {
        return SpanError::Success;
    }
    if elem_align == 0 || !elem_align.is_power_of_two() {
        return SpanError::InvalidStride;
    }
    SpanError::Success
}

fn arc_debug_enabled() -> bool {
    std::env::var("CHIC_DEBUG_WASM_ARC").is_ok()
}

impl<'a> Executor<'a> {
    pub(super) fn wasm_context_for_function_index(&self, func_index: u32) -> String {
        if let Some(name) = self.module.function_name(func_index) {
            return format!("{name} (idx={func_index})");
        }
        let export = self
            .module
            .exports
            .iter()
            .find_map(|(name, index)| (*index == func_index).then_some(name.as_str()));
        match export {
            Some(name) => format!("{name} (idx={func_index})"),
            None => format!("idx={func_index}"),
        }
    }

    pub(super) fn current_wasm_context(&self) -> String {
        let Some(func_index) = self.current_function else {
            return "<no wasm function>".into();
        };
        if let Some(name) = self.module.function_name(func_index) {
            return format!("{name} (idx={func_index}, depth={})", self.call_stack.len());
        }
        let export = self
            .module
            .exports
            .iter()
            .find_map(|(name, index)| (*index == func_index).then_some(name.as_str()));
        match export {
            Some(name) => format!("{name} (idx={func_index}, depth={})", self.call_stack.len()),
            None => format!("idx={func_index} (depth={})", self.call_stack.len()),
        }
    }

    pub(super) fn format_call_stack(&self) -> String {
        let mut buf = String::from("[");
        for (i, func_index) in self.call_stack.iter().copied().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(&self.wasm_context_for_function_index(func_index));
        }
        buf.push(']');
        buf
    }

    pub(crate) fn clone_string(
        &mut self,
        dest_ptr: u32,
        src_ptr: u32,
    ) -> Result<i32, WasmExecutionError> {
        if src_ptr == 0 {
            self.init_empty_string(dest_ptr)?;
            return Ok(0);
        }
        let src = self.read_string_repr(src_ptr)?;
        if src.len == 0 {
            self.init_empty_string(dest_ptr)?;
            return Ok(0);
        }
        let data_ptr = self.resolve_string_data_ptr(src_ptr, &src)?;
        let data = self.read_bytes(data_ptr, src.len)?;
        self.store_string_bytes(dest_ptr, &data)
    }

    pub(crate) fn clone_string_from_slice(
        &mut self,
        dest_ptr: u32,
        slice_ptr: u32,
        slice_len: u32,
    ) -> Result<i32, WasmExecutionError> {
        let data = self.read_bytes(slice_ptr, slice_len)?;
        self.store_string_bytes(dest_ptr, &data)
    }

    pub(crate) fn drop_string(&mut self, ptr: u32) -> Result<(), WasmExecutionError> {
        if ptr == 0 {
            return Ok(());
        }
        // The wasm executor uses a bump allocator; dropping strings only clears the header.
        self.write_string_repr(ptr, WasmStringRepr::default())
    }

    pub(crate) fn register_borrow(
        &mut self,
        borrow_id: i32,
        address: u32,
        kind: BorrowRuntimeKind,
    ) -> Result<(), WasmExecutionError> {
        if address == 0 {
            return Ok(());
        }
        let key = BorrowRuntimeKey {
            borrow_id,
            function: self.current_function.unwrap_or(0),
            frame_depth: self.call_depth.min(u32::MAX as usize) as u32,
        };
        if std::env::var("CHIC_DEBUG_WASM_BORROW").is_ok() {
            eprintln!(
                "[wasm-borrow] register id={} fn={} depth={} addr=0x{:08x} kind={:?} current={:?} stack={:?}",
                borrow_id,
                key.function,
                key.frame_depth,
                address,
                kind,
                self.current_future,
                self.call_stack
            );
        }
        if let Some(record) = self.borrow_records.get_mut(&key) {
            if record.address != address {
                match (record.kind, kind) {
                    (BorrowRuntimeKind::Unique, BorrowRuntimeKind::Unique) => {
                        return Err(WasmExecutionError {
                            message: format!(
                                "borrow {borrow_id} cannot be acquired more than once with {:?} access",
                                record.kind
                            ),
                        });
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: format!(
                                "borrow {borrow_id} acquired at 0x{:08x} but re-used for 0x{:08x}",
                                record.address, address
                            ),
                        });
                    }
                }
            }
            match (record.kind, kind) {
                (BorrowRuntimeKind::Shared, BorrowRuntimeKind::Shared) => {
                    record.ref_count = record.ref_count.saturating_add(1);
                    return Ok(());
                }
                (BorrowRuntimeKind::Unique, BorrowRuntimeKind::Unique) => {
                    return Err(WasmExecutionError {
                        message: format!(
                            "borrow {borrow_id} cannot be acquired more than once with {:?} access",
                            record.kind
                        ),
                    });
                }
                _ => {
                    return Err(WasmExecutionError {
                        message: format!(
                            "borrow {borrow_id} cannot be acquired more than once with {:?} access",
                            record.kind
                        ),
                    });
                }
            }
        }

        if matches!(kind, BorrowRuntimeKind::Unique) {
            for (other_key, record) in &self.borrow_records {
                if other_key.frame_depth != key.frame_depth {
                    continue;
                }
                if record.address == address && record.ref_count > 0 {
                    return Err(WasmExecutionError {
                        message: format!(
                            "cannot acquire {:?} borrow {borrow_id} while borrow {} holds access to 0x{:08x}",
                            kind, other_key.borrow_id, address
                        ),
                    });
                }
            }
        } else {
            for (other_key, record) in &self.borrow_records {
                if other_key.frame_depth != key.frame_depth {
                    continue;
                }
                if record.address == address
                    && record.ref_count > 0
                    && matches!(record.kind, BorrowRuntimeKind::Unique)
                {
                    return Err(WasmExecutionError {
                        message: format!(
                            "cannot acquire {:?} borrow {borrow_id} while borrow {} holds exclusive access to 0x{:08x}",
                            kind, other_key.borrow_id, address
                        ),
                    });
                }
            }
        }

        self.borrow_records.insert(
            key,
            BorrowRuntimeRecord {
                kind,
                address,
                ref_count: 1,
            },
        );
        Ok(())
    }

    pub(crate) fn release_borrow(&mut self, borrow_id: i32) -> Result<(), WasmExecutionError> {
        let key = BorrowRuntimeKey {
            borrow_id,
            function: self.current_function.unwrap_or(0),
            frame_depth: self.call_depth.min(u32::MAX as usize) as u32,
        };
        let record = if let Some(record) = self.borrow_records.get_mut(&key) {
            record
        } else {
            let mut candidates: Vec<BorrowRuntimeKey> = self
                .borrow_records
                .keys()
                .copied()
                .filter(|other| {
                    other.borrow_id == borrow_id && other.frame_depth == key.frame_depth
                })
                .collect();
            if candidates.len() != 1 {
                candidates = self
                    .borrow_records
                    .keys()
                    .copied()
                    .filter(|other| other.borrow_id == borrow_id)
                    .collect();
            }
            if candidates.len() == 1 {
                let fallback = candidates[0];
                self.borrow_records
                    .get_mut(&fallback)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!("borrow {borrow_id} released without being acquired"),
                    })?
            } else {
                return Err(WasmExecutionError {
                    message: format!(
                        "borrow {borrow_id} released without being acquired (fn={} depth={} candidates={})",
                        self.wasm_context_for_function_index(key.function),
                        key.frame_depth,
                        candidates.len()
                    ),
                });
            }
        };
        if std::env::var("CHIC_DEBUG_WASM_BORROW").is_ok() {
            eprintln!(
                "[wasm-borrow] release id={} fn={} depth={} addr=0x{:08x} kind={:?} ref_count={} current={:?}",
                borrow_id,
                key.function,
                key.frame_depth,
                record.address,
                record.kind,
                record.ref_count,
                self.current_future
            );
        }
        if record.ref_count == 0 {
            return Err(WasmExecutionError {
                message: format!("borrow {borrow_id} released after refcount reached zero"),
            });
        }
        record.ref_count -= 1;
        if record.ref_count == 0 {
            self.borrow_records.remove(&key);
        }
        Ok(())
    }

    pub(crate) fn drop_resource(&mut self, address: u32) -> Result<(), WasmExecutionError> {
        if let Some((borrow_id, _)) = self
            .borrow_records
            .iter()
            .find(|(_, record)| record.address == address && record.ref_count > 0)
        {
            return Err(WasmExecutionError {
                message: format!(
                    "resource at 0x{:08x} dropped while borrow {} is still active",
                    address, borrow_id.borrow_id
                ),
            });
        }
        Ok(())
    }

    fn persist_borrow_context(&mut self, future: Option<u32>) {
        let snapshot = mem::take(&mut self.borrow_records);
        if let Some(base) = future {
            self.async_nodes
                .entry(base)
                .or_insert_with(Default::default)
                .borrows = snapshot;
        } else {
            self.root_borrows = snapshot;
        }
    }

    fn load_borrow_context(&mut self, future: Option<u32>) {
        self.borrow_records = if let Some(base) = future {
            let entry = self
                .async_nodes
                .entry(base)
                .or_insert_with(Default::default);
            mem::take(&mut entry.borrows)
        } else {
            mem::take(&mut self.root_borrows)
        };
    }

    fn switch_borrow_context(&mut self, target: Option<u32>) -> Option<u32> {
        let prev = self.current_future;
        if prev == target {
            return prev;
        }
        self.persist_borrow_context(prev);
        self.load_borrow_context(target);
        self.current_future = target;
        prev
    }

    pub(crate) fn invoke_import(
        &mut self,
        import: &Import,
        params: Vec<Value>,
        tracer: &mut SchedulerTracer,
    ) -> Result<Vec<Value>, WasmExecutionError> {
        struct ImportGuard<'a> {
            exec: *mut Executor<'a>,
            prev: Option<(String, String)>,
        }

        impl<'a> Drop for ImportGuard<'a> {
            fn drop(&mut self) {
                unsafe {
                    (*self.exec).current_import = self.prev.take();
                }
            }
        }

        let prev_import = self.current_import.take();
        self.current_import = Some((import.module.clone(), import.name.clone()));
        let _import_guard = ImportGuard {
            exec: self as *mut Executor<'a>,
            prev: prev_import,
        };

        if import.module == "chic_rt" && import.name == "string_as_slice" {
            let [Value::I32(ptr)] = params.as_slice() else {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_as_slice expects a single i32 argument".into(),
                });
            };
            let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_as_slice received negative pointer".into(),
            })?;
            if base == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let repr = self.read_string_repr(base)?;
            let len = repr.len;
            let inline = self.is_inline_string(&repr);
            let slice_ptr = if len == 0 {
                0
            } else {
                self.resolve_string_data_ptr(base, &repr)?
            };
            if len > 0 {
                let mem_len = u32::try_from(self.memory_len()).map_err(|_| WasmExecutionError {
                    message: "wasm memory length exceeds addressable range for wasm32".into(),
                })?;
                let end = slice_ptr
                    .checked_add(len)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!(
                            "string_as_slice pointer overflow: base=0x{base:08X} slice_ptr=0x{slice_ptr:08X} len={len}"
                        ),
                    })?;
                if end > mem_len {
                    return Err(WasmExecutionError {
                        message: format!(
                            "string_as_slice out of bounds: base=0x{base:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline} slice_ptr=0x{slice_ptr:08X} end=0x{end:08X} mem_len=0x{mem_len:08X}",
                            ptr = repr.ptr,
                            cap = repr.cap
                        ),
                    });
                }
            }
            if std::env::var_os("CHIC_DEBUG_WASM_STRING_AS_SLICE").is_some() {
                let stride = self.ptr_stride();
                eprintln!(
                    "[wasm-string] as_slice base=0x{base:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline} slice_ptr=0x{slice_ptr:08X} mem_len={} stride={stride}",
                    self.memory_len(),
                    ptr = repr.ptr,
                    cap = repr.cap
                );
            }
            return Ok(vec![Value::I32(slice_ptr as i32), Value::I32(len as i32)]);
        }

        if import.module == "chic_rt" && import.name == "string_try_copy_utf8" {
            let [
                Value::I32(string_ptr),
                Value::I32(span_ptr),
                Value::I32(out_written_ptr),
            ] = params.as_slice()
            else {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_try_copy_utf8 expects (i32, i32, i32) arguments"
                        .into(),
                });
            };
            let string_ptr = u32::try_from(*string_ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_try_copy_utf8 received negative string pointer".into(),
            })?;
            let span_ptr = u32::try_from(*span_ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_try_copy_utf8 received negative span pointer".into(),
            })?;
            let out_written_ptr =
                u32::try_from(*out_written_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_try_copy_utf8 received negative out pointer".into(),
                })?;
            if out_written_ptr == 0 {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_try_copy_utf8 out pointer must be non-null".into(),
                });
            }

            let (dest_ptr, dest_len, elem_size, elem_align) = if span_ptr == 0 {
                (0, 0, 0, 0)
            } else {
                self.read_span_ptr(span_ptr)?
            };

            let (slice_ptr, len) = if string_ptr == 0 {
                (0, 0)
            } else {
                let repr = self.read_string_repr(string_ptr)?;
                if repr.len == 0 {
                    (0, 0)
                } else {
                    let slice_ptr = self.resolve_string_data_ptr(string_ptr, &repr)?;
                    (slice_ptr, repr.len)
                }
            };

            let mut written = 0u32;
            let ok = if len == 0 {
                true
            } else if span_ptr == 0 {
                false
            } else if elem_size != 1 || elem_align != 1 {
                false
            } else if dest_len < len {
                false
            } else if dest_ptr == 0 {
                false
            } else {
                let data = self.read_bytes(slice_ptr, len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                written = len;
                true
            };

            self.write_u32(out_written_ptr, written)?;
            return Ok(vec![Value::I32(if ok { 1 } else { 0 })]);
        }

        if import.module == "chic_rt" && import.name == "string_as_chars" {
            let [Value::I32(ptr)] = params.as_slice() else {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_as_chars expects a single i32 argument".into(),
                });
            };
            let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_as_chars received negative pointer".into(),
            })?;
            if base == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let repr = self.read_string_repr(base)?;
            if repr.len == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let slice_ptr = self.resolve_string_data_ptr(base, &repr)?;
            let data = self.read_bytes(slice_ptr, repr.len)?;
            let text = std::str::from_utf8(&data).map_err(|err| WasmExecutionError {
                message: format!("chic_rt.string_as_chars encountered invalid UTF-8: {err}"),
            })?;
            let mut utf16_units: Vec<u16> = Vec::new();
            for ch in text.chars() {
                let mut buf = [0u16; 2];
                let encoded = ch.encode_utf16(&mut buf);
                utf16_units.extend_from_slice(encoded);
            }
            if utf16_units.is_empty() {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let mut encoded = Vec::with_capacity(utf16_units.len() * 2);
            for unit in utf16_units {
                encoded.extend_from_slice(&unit.to_le_bytes());
            }
            let byte_len = u32::try_from(encoded.len()).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_as_chars output exceeds wasm32 addressable range".into(),
            })?;
            let out_ptr = self.allocate_heap_block(byte_len, 2)?;
            self.store_bytes(out_ptr, 0, &encoded)?;
            let char_len = i32::try_from(encoded.len() / 2).unwrap_or(i32::MAX);
            return Ok(vec![Value::I32(out_ptr as i32), Value::I32(char_len)]);
        }

        if import.module == "chic_rt" && import.name == "str_as_chars" {
            let [Value::I32(slice_ptr)] = params.as_slice() else {
                return Err(WasmExecutionError {
                    message: "chic_rt.str_as_chars expects a single i32 argument".into(),
                });
            };
            let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.str_as_chars received negative pointer".into(),
            })?;
            if slice_ptr == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let (ptr, len) = self.read_str_ptr(slice_ptr)?;
            if len == 0 || ptr == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let data = self.read_bytes(ptr, len)?;
            let text = std::str::from_utf8(&data).map_err(|err| WasmExecutionError {
                message: format!("chic_rt.str_as_chars encountered invalid UTF-8: {err}"),
            })?;
            let mut utf16_units: Vec<u16> = Vec::new();
            for ch in text.chars() {
                let mut buf = [0u16; 2];
                let encoded = ch.encode_utf16(&mut buf);
                utf16_units.extend_from_slice(encoded);
            }
            if utf16_units.is_empty() {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let mut encoded = Vec::with_capacity(utf16_units.len() * 2);
            for unit in utf16_units {
                encoded.extend_from_slice(&unit.to_le_bytes());
            }
            let byte_len = u32::try_from(encoded.len()).map_err(|_| WasmExecutionError {
                message: "chic_rt.str_as_chars output exceeds wasm32 addressable range".into(),
            })?;
            let out_ptr = self.allocate_heap_block(byte_len, 2)?;
            self.store_bytes(out_ptr, 0, &encoded)?;
            let char_len = i32::try_from(encoded.len() / 2).unwrap_or(i32::MAX);
            return Ok(vec![Value::I32(out_ptr as i32), Value::I32(char_len)]);
        }

        let result = (|| match (import.module.as_str(), import.name.as_str()) {
            ("chic_rt", "rounding_mode") => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "chic_rt.rounding_mode expects ()".into(),
                    });
                }
                let mode = rounding_mode() as i32;
                return Ok(Some(Value::I32(mode)));
            }
            ("chic_rt", "f32_rem") | ("chic_rt", "chic_rt_f32_rem") => {
                let [Value::F32(lhs), Value::F32(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.f32_rem expects (f32 lhs, f32 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F32(lhs % rhs)));
            }
            ("chic_rt", "f64_rem") | ("chic_rt", "chic_rt_f64_rem") => {
                let [Value::F64(lhs), Value::F64(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.f64_rem expects (f64 lhs, f64 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F64(lhs % rhs)));
            }
            ("chic_rt", name) if name.starts_with("math_") => {
                return Err(WasmExecutionError {
                    message: format!(
                        "math shim import `{name}` is unsupported; Std.Math lowers directly"
                    ),
                });
            }
            ("env", "write") => {
                let [Value::I32(fd), Value::I32(ptr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.write expects (i32 fd, i32 ptr, i32 len)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.write received negative pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.write received negative length".into(),
                })?;
                let written = self.host_write(*fd, ptr, len)?;
                return Ok(Some(Value::I32(written)));
            }
            ("env", "read") => {
                let [Value::I32(fd), Value::I32(ptr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.read expects (i32 fd, i32 ptr, i32 len)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.read received negative pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.read received negative length".into(),
                })?;
                let read = self.host_read(*fd, ptr, len)?;
                return Ok(Some(Value::I32(read)));
            }
            ("env", "isatty") => {
                let [Value::I32(fd)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.isatty expects (i32 fd)".into(),
                    });
                };
                let result = self.host_isatty(*fd);
                return Ok(Some(Value::I32(result)));
            }
            ("env", "monotonic_nanos") => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "env.monotonic_nanos expects ()".into(),
                    });
                }
                let value = self.host_monotonic_nanos()?;
                return Ok(Some(Value::I64(value)));
            }
            ("env", "sleep_millis") => {
                let [Value::I32(ms)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.sleep_millis expects (i32 millis)".into(),
                    });
                };
                let millis = u32::try_from(*ms).map_err(|_| WasmExecutionError {
                    message: "env.sleep_millis received negative millis".into(),
                })?;
                let code = self.host_sleep_millis(millis)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "malloc") => {
                let [size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.malloc expects (i32 size)".into(),
                    });
                };
                let size = value_as_u32(size, "env.malloc size")?;
                let ptr = self.allocate_heap_block(size, 8)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                }
                return Ok(Some(Value::I32(ptr as i32)));
            }
            ("env", "calloc") => {
                let [count, size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.calloc expects (i32 count, i32 size)".into(),
                    });
                };
                let count = value_as_u32(count, "env.calloc count")?;
                let size = value_as_u32(size, "env.calloc size")?;
                let total = match count.checked_mul(size) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                let ptr = self.allocate_heap_block(total, 8)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, total as usize);
                }
                return Ok(Some(Value::I32(ptr as i32)));
            }
            ("env", "realloc") => {
                let [ptr, size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.realloc expects (i32 ptr, i32 size)".into(),
                    });
                };
                let ptr = value_as_ptr_u32(ptr, "env.realloc ptr")?;
                let size = value_as_u32(size, "env.realloc size")?;
                if size == 0 {
                    if ptr != 0 {
                        self.heap_allocations.remove(&ptr);
                    }
                    return Ok(Some(Value::I32(0)));
                }
                if ptr == 0 {
                    let new_ptr = self.allocate_heap_block(size, 8)?;
                    if new_ptr != 0 {
                        self.heap_allocations.insert(new_ptr, size as usize);
                    }
                    return Ok(Some(Value::I32(new_ptr as i32)));
                }
                let old_size = self.heap_allocations.get(&ptr).copied().unwrap_or(0) as u32;
                let new_ptr = self.allocate_heap_block(size, 8)?;
                if new_ptr != 0 {
                    let copy_len = old_size.min(size);
                    if copy_len > 0 {
                        let data = self.read_bytes(ptr, copy_len)?;
                        self.store_bytes(new_ptr, 0, &data)?;
                    }
                    self.heap_allocations.remove(&ptr);
                    self.heap_allocations.insert(new_ptr, size as usize);
                }
                return Ok(Some(Value::I32(new_ptr as i32)));
            }
            ("env", "free") => {
                let [ptr] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.free expects (i32 ptr)".into(),
                    });
                };
                let ptr = value_as_ptr_u32(ptr, "env.free ptr")?;
                if ptr != 0 {
                    self.heap_allocations.remove(&ptr);
                }
                return Ok(None);
            }
            ("env", "posix_memalign") => {
                let [out_ptr, align, size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.posix_memalign expects (i32 out_ptr, i32 align, i32 size)"
                            .into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(out_ptr, "env.posix_memalign out_ptr")?;
                let align = value_as_u32(align, "env.posix_memalign align")?;
                let size = value_as_u32(size, "env.posix_memalign size")?;
                let ptr = self.allocate_heap_block(size, align)?;
                self.write_u32(out_ptr, ptr)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                    return Ok(Some(Value::I32(0)));
                }
                return Ok(Some(Value::I32(1)));
            }
            ("env", "memcpy") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.memcpy expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "env.memcpy dest")?;
                let src = value_as_ptr_u32(src, "env.memcpy src")?;
                let len = value_as_u32(len, "env.memcpy len")?;
                if len > 0 {
                    let data = self.read_bytes(src, len)?;
                    self.store_bytes(dest, 0, &data)?;
                }
                return Ok(None);
            }
            ("env", "memmove") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.memmove expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "env.memmove dest")?;
                let src = value_as_ptr_u32(src, "env.memmove src")?;
                let len = value_as_u32(len, "env.memmove len")?;
                if len > 0 {
                    let data = self.read_bytes(src, len)?;
                    self.store_bytes(dest, 0, &data)?;
                }
                return Ok(None);
            }
            ("env", "memset") => {
                let [dest, value, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.memset expects (i32 dest, i32 value, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "env.memset dest")?;
                let value = match value {
                    Value::I32(v) => *v as u8,
                    Value::I64(v) => *v as u8,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "env.memset value must be integer".into(),
                        });
                    }
                };
                let len = value_as_u32(len, "env.memset len")?;
                if len > 0 {
                    if std::env::var_os("CHIC_DEBUG_WASM_MEMSET").is_some() {
                        eprintln!(
                            "[wasm-mem] memset[env] dest=0x{dest:08X} value=0x{value:02X} len={len} mem_len={} caller={}",
                            self.memory_len(),
                            self.current_wasm_context(),
                        );
                    }
                    self.fill(dest, 0, len, value)?;
                }
                return Ok(None);
            }
            ("chic_rt", "alloc") | ("chic_rt", "chic_rt_alloc") => {
                let [Value::I32(out_ptr), size, align] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.alloc expects (i32 out_ptr, i32 size, i32 align)".into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let size = value_as_u32(size, "chic_rt.alloc size")?;
                let align = value_as_u32(align, "chic_rt.alloc align")?.max(1);
                let ptr = self.allocate_heap_block(size, align)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                    if size > 0 {
                        self.fill(ptr, 0, size, 0)?;
                    }
                }
                self.write_value_ptr(out_ptr, ptr, size, align)?;
                if std::env::var_os("CHIC_DEBUG_WASM_ALLOC").is_some() {
                    let (check_ptr, check_size, check_align) = self
                        .read_value_ptr(out_ptr)
                        .unwrap_or((u32::MAX, u32::MAX, u32::MAX));
                    eprintln!(
                        "[wasm-alloc] alloc out=0x{out_ptr:08X} wrote={{ptr=0x{check_ptr:08X} size={check_size} align={check_align}}} req={{size={size} align={align}}} heap_ptr=0x{ptr:08X} heap_cursor=0x{:08X} mem_len={}",
                        self.heap_cursor,
                        self.memory_len()
                    );
                }
                return Ok(Some(Value::I32(out_ptr as i32)));
            }
            ("chic_rt", "alloc_zeroed") | ("chic_rt", "chic_rt_alloc_zeroed") => {
                let [Value::I32(out_ptr), size, align] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.alloc_zeroed expects (i32 out_ptr, i32 size, i32 align)"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let size = value_as_u32(size, "chic_rt.alloc_zeroed size")?;
                let align = value_as_u32(align, "chic_rt.alloc_zeroed align")?.max(1);
                let ptr = self.allocate_heap_block(size, align)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                }
                self.write_value_ptr(out_ptr, ptr, size, align)?;
                if std::env::var_os("CHIC_DEBUG_WASM_ALLOC").is_some() {
                    let (check_ptr, check_size, check_align) = self
                        .read_value_ptr(out_ptr)
                        .unwrap_or((u32::MAX, u32::MAX, u32::MAX));
                    eprintln!(
                        "[wasm-alloc] alloc_zeroed out=0x{out_ptr:08X} wrote={{ptr=0x{check_ptr:08X} size={check_size} align={check_align}}} req={{size={size} align={align}}} heap_ptr=0x{ptr:08X} heap_cursor=0x{:08X} mem_len={}",
                        self.heap_cursor,
                        self.memory_len()
                    );
                }
                return Ok(Some(Value::I32(out_ptr as i32)));
            }
            ("chic_rt", "realloc") | ("chic_rt", "chic_rt_realloc") => {
                let [Value::I32(out_ptr), ptr, old_size, new_size, align] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.realloc expects (i32 out_ptr, i32 ptr, i32 old_size, i32 new_size, i32 align)"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let ptr = value_as_ptr_u32(ptr, "chic_rt.realloc ptr")?;
                let old_size = value_as_u32(old_size, "chic_rt.realloc old_size")?;
                let new_size = value_as_u32(new_size, "chic_rt.realloc new_size")?;
                let align = value_as_u32(align, "chic_rt.realloc align")?.max(1);
                let (base_ptr, _, _) = self.read_value_ptr(ptr)?;
                if new_size == 0 {
                    if base_ptr != 0 && old_size != 0 {
                        self.heap_allocations.remove(&base_ptr);
                    }
                    self.write_value_ptr(out_ptr, 0, 0, align)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let new_ptr = self.allocate_heap_block(new_size, align)?;
                if new_ptr != 0 && base_ptr != 0 {
                    let copy_len = old_size.min(new_size);
                    if copy_len > 0 {
                        let data = self.read_bytes(base_ptr, copy_len)?;
                        self.store_bytes(new_ptr, 0, &data)?;
                    }
                }
                if base_ptr != 0 {
                    self.heap_allocations.remove(&base_ptr);
                }
                if new_ptr != 0 {
                    self.heap_allocations.insert(new_ptr, new_size as usize);
                }
                self.write_value_ptr(out_ptr, new_ptr, new_size, align)?;
                return Ok(Some(Value::I32(out_ptr as i32)));
            }
            ("chic_rt", "free") | ("chic_rt", "chic_rt_free") => {
                let [ptr] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.free expects (i32 ptr)".into(),
                    });
                };
                let ptr = value_as_ptr_u32(ptr, "chic_rt.free ptr")?;
                let (base_ptr, _, _) = self.read_value_ptr(ptr)?;
                if base_ptr != 0 {
                    self.heap_allocations.remove(&base_ptr);
                }
                return Ok(None);
            }
            ("chic_rt", "memcpy") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.memcpy expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt.memcpy dest")?;
                let src = value_as_ptr_u32(src, "chic_rt.memcpy src")?;
                let len = value_as_u32(len, "chic_rt.memcpy len")?;
                if len == 0 {
                    return Ok(None);
                }
                if dest == 0 || src == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memcpy[raw] dest=0x{dest:08X} src=0x{src:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                let data = self.read_bytes(src, len)?;
                self.store_bytes(dest, 0, &data)?;
                return Ok(None);
            }
            ("chic_rt", "chic_rt_memcpy") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_memcpy expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt_memcpy dest")?;
                let src = value_as_ptr_u32(src, "chic_rt_memcpy src")?;
                let len = value_as_u32(len, "chic_rt_memcpy len")?;
                if len == 0 {
                    return Ok(None);
                }
                let (dest_ptr, _, _) = self.read_value_ptr(dest)?;
                let (src_ptr, _, _) = self.read_value_ptr(src)?;
                if dest_ptr == 0 || src_ptr == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memcpy[valueptr] dest=0x{dest:08X}->0x{dest_ptr:08X} src=0x{src:08X}->0x{src_ptr:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                let data = self.read_bytes(src_ptr, len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                return Ok(None);
            }
            ("chic_rt", "memmove") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.memmove expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt.memmove dest").map_err(|err| {
                    WasmExecutionError {
                        message: format!("{} (ctx={})", err.message, self.current_wasm_context()),
                    }
                })?;
                let src = value_as_ptr_u32(src, "chic_rt.memmove src").map_err(|err| {
                    WasmExecutionError {
                        message: format!("{} (ctx={})", err.message, self.current_wasm_context()),
                    }
                })?;
                let len = value_as_u32(len, "chic_rt.memmove len")?;
                if len == 0 {
                    return Ok(None);
                }
                if dest == 0 || src == 0 {
                    if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some()
                        && std::env::var_os("CHIC_DEBUG_WASM_MEM_NULLS").is_some()
                    {
                        eprintln!(
                            "[wasm-mem] memmove[raw] skipped dest=0x{dest:08X} src=0x{src:08X} len={len} caller={}",
                            self.current_wasm_context(),
                        );
                    }
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memmove[raw] dest=0x{dest:08X} src=0x{src:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                if len == 12 && std::env::var_os("CHIC_DEBUG_WASM_VALUEPTR_MOVES").is_some() {
                    if let (Ok(p0), Ok(p1), Ok(p2)) = (
                        self.read_u32(src),
                        self.read_u32(src + 4),
                        self.read_u32(src + 8),
                    ) {
                        eprintln!(
                            "[wasm-valueptr] memmove src=0x{src:08X} {{ptr=0x{p0:08X} size={p1} align={p2}}} -> dest=0x{dest:08X} caller={}",
                            self.current_wasm_context(),
                        );
                    }
                }
                let data = self.read_bytes(src, len)?;
                self.store_bytes(dest, 0, &data)?;
                if len == 12 && std::env::var_os("CHIC_DEBUG_WASM_VALUEPTR_MOVES").is_some() {
                    if let (Ok(p0), Ok(p1), Ok(p2)) = (
                        self.read_u32(dest),
                        self.read_u32(dest + 4),
                        self.read_u32(dest + 8),
                    ) {
                        eprintln!(
                            "[wasm-valueptr] memmove dest=0x{dest:08X} now {{ptr=0x{p0:08X} size={p1} align={p2}}} caller={}",
                            self.current_wasm_context(),
                        );
                    }
                }
                return Ok(None);
            }
            ("chic_rt", "chic_rt_memmove") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_memmove expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt_memmove dest")?;
                let src = value_as_ptr_u32(src, "chic_rt_memmove src")?;
                let len = value_as_u32(len, "chic_rt_memmove len")?;
                if len == 0 {
                    return Ok(None);
                }
                let (dest_ptr, _, _) = self.read_value_ptr(dest)?;
                let (src_ptr, _, _) = self.read_value_ptr(src)?;
                if dest_ptr == 0 || src_ptr == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memmove[valueptr] dest=0x{dest:08X}->0x{dest_ptr:08X} src=0x{src:08X}->0x{src_ptr:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                let data = self.read_bytes(src_ptr, len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                return Ok(None);
            }
            ("chic_rt", "memset") => {
                let [dest, value, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.memset expects (i32 dest, i32 value, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt.memset dest")?;
                let value = match value {
                    Value::I32(v) => *v as u8,
                    Value::I64(v) => *v as u8,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "chic_rt.memset value must be integer".into(),
                        });
                    }
                };
                let len = value_as_u32(len, "chic_rt.memset len")?;
                if len == 0 {
                    return Ok(None);
                }
                if dest == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memset[raw] dest=0x{dest:08X} value=0x{value:02X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                self.fill(dest, 0, len, value)?;
                return Ok(None);
            }
            ("chic_rt", "chic_rt_memset") => {
                let [dest, value, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_memset expects (i32 dest, i32 value, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt_memset dest")?;
                let value = match value {
                    Value::I32(v) => *v as u8,
                    Value::I64(v) => *v as u8,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "chic_rt_memset value must be integer".into(),
                        });
                    }
                };
                let len = value_as_u32(len, "chic_rt_memset len")?;
                if len == 0 {
                    return Ok(None);
                }
                let (dest_ptr, _, _) = self.read_value_ptr(dest)?;
                if dest_ptr == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memset[valueptr] dest=0x{dest:08X}->0x{dest_ptr:08X} value=0x{value:02X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                self.fill(dest_ptr, 0, len, value)?;
                return Ok(None);
            }
            ("env", "fmodf") => {
                let [Value::F32(lhs), Value::F32(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fmodf expects (f32 lhs, f32 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F32(lhs % rhs)));
            }
            ("env", "fmod") => {
                let [Value::F64(lhs), Value::F64(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fmod expects (f64 lhs, f64 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F64(lhs % rhs)));
            }
            ("env", "fopen") => {
                let [Value::I32(path), Value::I32(mode)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fopen expects (i32 path, i32 mode)".into(),
                    });
                };
                let path_ptr = u32::try_from(*path).map_err(|_| WasmExecutionError {
                    message: "env.fopen received negative path pointer".into(),
                })?;
                let mode_ptr = u32::try_from(*mode).map_err(|_| WasmExecutionError {
                    message: "env.fopen received negative mode pointer".into(),
                })?;
                let handle = self.host_fopen(path_ptr, mode_ptr)?;
                return Ok(Some(Value::I32(handle)));
            }
            ("env", "fread") => {
                let [
                    Value::I32(ptr),
                    Value::I32(size),
                    Value::I32(count),
                    Value::I32(stream),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.fread expects (i32 ptr, i32 size, i32 count, i32 stream)"
                            .into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.fread received negative pointer".into(),
                })?;
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "env.fread received negative element size".into(),
                })?;
                let count = u32::try_from(*count).map_err(|_| WasmExecutionError {
                    message: "env.fread received negative count".into(),
                })?;
                let read = self.host_fread(*stream, ptr, size, count)?;
                return Ok(Some(Value::I32(read)));
            }
            ("env", "fwrite") => {
                let [
                    Value::I32(ptr),
                    Value::I32(size),
                    Value::I32(count),
                    Value::I32(stream),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.fwrite expects (i32 ptr, i32 size, i32 count, i32 stream)"
                            .into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.fwrite received negative pointer".into(),
                })?;
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "env.fwrite received negative element size".into(),
                })?;
                let count = u32::try_from(*count).map_err(|_| WasmExecutionError {
                    message: "env.fwrite received negative count".into(),
                })?;
                let written = self.host_fwrite(*stream, ptr, size, count)?;
                return Ok(Some(Value::I32(written)));
            }
            ("env", "fflush") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fflush expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fflush(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "fclose") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fclose expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fclose(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "fileno") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fileno expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fileno(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "ftell") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.ftell expects (i32 stream)".into(),
                    });
                };
                let pos = self.host_ftell(*stream)?;
                return Ok(Some(Value::I64(pos)));
            }
            ("env", "ftruncate") => {
                let [Value::I32(stream), length] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.ftruncate expects (i32 stream, i64 length)".into(),
                    });
                };
                let length = value_as_i64(length, "env.ftruncate length")?;
                let code = self.host_ftruncate(*stream, length)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "fprintf") => {
                if params.len() < 2 {
                    return Err(WasmExecutionError {
                        message: "env.fprintf expects (i32 stream, i32 fmt, ...)".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            ("env", "snprintf") => {
                if params.len() < 3 {
                    return Err(WasmExecutionError {
                        message: "env.snprintf expects (i32 buffer, i32 size, i32 fmt, ...)".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            ("env", "fputc") => {
                let [Value::I32(ch), Value::I32(_stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fputc expects (i32 ch, i32 stream)".into(),
                    });
                };
                return Ok(Some(Value::I32(*ch)));
            }
            ("env", "pthread_mutex_init") => {
                let [Value::I32(_mutex), Value::I32(_attr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_init expects (i32 mutex, i32 attr)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_mutex_lock") => {
                let [Value::I32(_mutex)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_lock expects (i32 mutex)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_mutex_unlock") => {
                let [Value::I32(_mutex)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_unlock expects (i32 mutex)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_create") => {
                let [thread_ptr, _attrs, entry, arg] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_create expects (i32 thread_ptr, i32 attrs, i64 entry, i32 arg)".into(),
                    });
                };
                let thread_ptr = value_as_ptr_u32(thread_ptr, "env.pthread_create thread_ptr")?;
                let entry = value_as_i64(entry, "env.pthread_create entry")?;
                let arg = value_as_ptr_u32(arg, "env.pthread_create arg")?;
                let func_index = entry as i32;
                if func_index < 0 {
                    return Err(WasmExecutionError {
                        message: "env.pthread_create received negative function pointer".into(),
                    });
                }
                self.invoke(func_index as u32, &[Value::I32(arg as i32)])?;
                let thread_id = self.allocate_thread_id();
                self.write_u32(thread_ptr, thread_id)?;
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_join") => {
                let [_thread, _retval] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_join expects (i32 thread, i32 retval)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_detach") => {
                let [_thread] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_detach expects (i32 thread)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_setname_np") => {
                let [_thread, _name] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_setname_np expects (i32 thread, i32 name)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "sched_yield") => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "env.sched_yield expects ()".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            ("env", "clock_gettime") => {
                let [Value::I32(clock_id), Value::I32(ts_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.clock_gettime expects (i32 clock_id, i32 ts_ptr)".into(),
                    });
                };
                let ts_ptr = u32::try_from(*ts_ptr).map_err(|_| WasmExecutionError {
                    message: "env.clock_gettime received negative timespec pointer".into(),
                })?;
                let code = self.host_clock_gettime(*clock_id, ts_ptr)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "nanosleep") => {
                let [Value::I32(req_ptr), Value::I32(rem_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.nanosleep expects (i32 req_ptr, i32 rem_ptr)".into(),
                    });
                };
                let req = u32::try_from(*req_ptr).map_err(|_| WasmExecutionError {
                    message: "env.nanosleep received negative request pointer".into(),
                })?;
                let rem = if *rem_ptr == 0 {
                    None
                } else {
                    Some(u32::try_from(*rem_ptr).map_err(|_| WasmExecutionError {
                        message: "env.nanosleep received negative remainder pointer".into(),
                    })?)
                };
                let code = self.host_nanosleep(req, rem)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "accept") => {
                let [_fd, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.accept expects (i32 fd, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "bind") => {
                let [_fd, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.bind expects (i32 fd, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "recvfrom") => {
                let [_fd, _ptr, _len, _flags, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.recvfrom expects (i32 fd, i32 ptr, i32 len, i32 flags, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "sendto") => {
                let [_fd, _ptr, _len, _flags, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.sendto expects (i32 fd, i32 ptr, i32 len, i32 flags, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "chic_thread_invoke") => {
                let [_ctx] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.chic_thread_invoke expects (i32 ctx)".into(),
                    });
                };
                return Ok(None);
            }
            ("env", "chic_thread_drop") => {
                let [_ctx] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.chic_thread_drop expects (i32 ctx)".into(),
                    });
                };
                return Ok(None);
            }
            ("env", "socket") => {
                let [Value::I32(domain), Value::I32(typ), Value::I32(proto)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.socket expects (i32 domain, i32 type, i32 proto)".into(),
                    });
                };
                let fd = self.host_socket(*domain, *typ, *proto)?;
                return Ok(Some(Value::I32(fd)));
            }
            ("env", "connect") => {
                let [Value::I32(fd), Value::I32(addr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.connect expects (i32 fd, i32 sockaddr, i32 len)".into(),
                    });
                };
                let addr_ptr = u32::try_from(*addr).map_err(|_| WasmExecutionError {
                    message: "env.connect received negative sockaddr pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.connect received negative sockaddr length".into(),
                })?;
                let code = self.host_connect(*fd, addr_ptr, len)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "recv") => {
                let [
                    Value::I32(fd),
                    Value::I32(ptr),
                    Value::I32(len),
                    Value::I32(_flags),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.recv expects (i32 fd, i32 ptr, i32 len, i32 flags)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.recv received negative buffer pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.recv received negative length".into(),
                })?;
                let read = self.host_recv(*fd, ptr, len)?;
                return Ok(Some(Value::I32(read)));
            }
            ("env", "send") => {
                let [
                    Value::I32(fd),
                    Value::I32(ptr),
                    Value::I32(len),
                    Value::I32(_flags),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.send expects (i32 fd, i32 ptr, i32 len, i32 flags)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.send received negative buffer pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.send received negative length".into(),
                })?;
                let written = self.host_send(*fd, ptr, len)?;
                return Ok(Some(Value::I32(written)));
            }
            ("env", "shutdown") => {
                let [Value::I32(fd), Value::I32(how)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.shutdown expects (i32 fd, i32 how)".into(),
                    });
                };
                let code = self.host_shutdown_socket(*fd, *how)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "close") => {
                let [Value::I32(fd)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.close expects (i32 fd)".into(),
                    });
                };
                let code = self.host_close_socket(*fd)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "htons") => {
                let [Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.htons expects (i32 value)".into(),
                    });
                };
                let v = u16::try_from(*value as i32).map_err(|_| WasmExecutionError {
                    message: "env.htons received negative value".into(),
                })?;
                let converted = self.host_htons(v);
                return Ok(Some(Value::I32(converted)));
            }
            ("env", "inet_pton") => {
                let [Value::I32(af), Value::I32(src), Value::I32(dst)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.inet_pton expects (i32 af, i32 src, i32 dst)".into(),
                    });
                };
                let src_ptr = u32::try_from(*src).map_err(|_| WasmExecutionError {
                    message: "env.inet_pton received negative src pointer".into(),
                })?;
                let dst_ptr = u32::try_from(*dst).map_err(|_| WasmExecutionError {
                    message: "env.inet_pton received negative dst pointer".into(),
                })?;
                let code = self.host_inet_pton(*af, src_ptr, dst_ptr)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "chic_rt_thread_sleep_ms") | ("chic_rt", "chic_rt_thread_sleep_ms") => {
                let [Value::I64(ms)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_sleep_ms expects (i64 millis)".into(),
                    });
                };
                let millis = u64::try_from(*ms).unwrap_or(0);
                let _ = self.host_sleep_millis(millis.min(u64::from(u32::MAX)) as u32)?;
                return Ok(None);
            }
            ("env", "chic_rt_thread_yield") | ("chic_rt", "chic_rt_thread_yield") => {
                let _ = self.host_sleep_millis(0)?;
                return Ok(None);
            }
            ("env", "chic_rt_thread_spin_wait") | ("chic_rt", "chic_rt_thread_spin_wait") => {
                let [Value::I32(iters)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_spin_wait expects (i32 iterations)".into(),
                    });
                };
                let iterations = u32::try_from(*iters).unwrap_or(0);
                for _ in 0..iterations {
                    std::hint::spin_loop();
                }
                return Ok(None);
            }
            ("env", "chic_rt_thread_spawn") | ("chic_rt", "chic_rt_thread_spawn") => {
                let [Value::I32(_start), Value::I32(handle_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_spawn expects (i32 start, i32 handle_ptr)".into(),
                    });
                };
                if *handle_ptr != 0 {
                    let slot = u32::try_from(*handle_ptr).map_err(|_| WasmExecutionError {
                        message: "chic_rt_thread_spawn received negative handle pointer".into(),
                    })?;
                    let _ = self.store_i32(slot, 0, 0);
                }
                return Ok(Some(Value::I32(1)));
            }
            ("env", "chic_rt_thread_join") | ("chic_rt", "chic_rt_thread_join") => {
                let [Value::I32(_handle_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_join expects (i32 handle_ptr)".into(),
                    });
                };
                return Ok(Some(Value::I32(1)));
            }
            ("env", "chic_rt_thread_detach") | ("chic_rt", "chic_rt_thread_detach") => {
                let [Value::I32(_handle_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_detach expects (i32 handle_ptr)".into(),
                    });
                };
                return Ok(Some(Value::I32(1)));
            }
            ("chic_rt", "arc_new") | ("chic_rt", "chic_rt_arc_new") => {
                let [
                    Value::I32(dest_ptr),
                    Value::I32(src_ptr),
                    Value::I32(size),
                    Value::I32(align),
                    Value::I32(drop_fn),
                    Value::I64(type_id),
                ] = params.as_slice()
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
                    if value_ptr < super::scheduler::LINEAR_MEMORY_HEAP_BASE {
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
            ("chic_rt", "arc_clone") | ("chic_rt", "chic_rt_arc_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
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
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if src >= super::scheduler::LINEAR_MEMORY_HEAP_BASE {
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
            ("chic_rt", "arc_drop") | ("chic_rt", "chic_rt_arc_drop") => {
                let [Value::I32(target_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_drop expects (i32 target)".into(),
                    });
                };
                let target = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_drop received negative target pointer".into(),
                })?;
                if target != 0 {
                    if let Ok(mut header) = self.read_u32(target) {
                        if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "arc_get") | ("chic_rt", "chic_rt_arc_get") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_get expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_get received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "arc_get_mut") | ("chic_rt", "chic_rt_arc_get_mut") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_get_mut expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_get_mut received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "arc_downgrade") | ("chic_rt", "chic_rt_arc_downgrade") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
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
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "weak_clone") | ("chic_rt", "chic_rt_weak_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
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
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "weak_drop") | ("chic_rt", "chic_rt_weak_drop") => {
                let [Value::I32(target_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_drop expects (i32 target)".into(),
                    });
                };
                let target = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_drop received negative target pointer".into(),
                })?;
                if target != 0 {
                    if let Ok(mut header) = self.read_u32(target) {
                        if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "weak_upgrade") | ("chic_rt", "chic_rt_weak_upgrade") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
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
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "arc_strong_count") | ("chic_rt", "chic_rt_arc_strong_count") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_strong_count expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_strong_count received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
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
            ("chic_rt", "arc_weak_count") | ("chic_rt", "chic_rt_arc_weak_count") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_weak_count expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_weak_count received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let count = self.read_u32(header + ARC_WEAK_OFFSET).unwrap_or_default() as i32;
                return Ok(Some(Value::I32(count)));
            }
            ("chic_rt", "object_new") => {
                let [Value::I64(type_id)] = params.as_slice() else {
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
            ("chic_rt", "panic") => {
                let [Value::I32(code)] = params.as_slice() else {
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
            ("chic_rt", "abort") => {
                let [Value::I32(code)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.abort expects a single i32 argument".into(),
                    });
                };
                Err(abort_trap(*code))
            }
            ("chic_rt", "coverage_hit") => {
                let [Value::I64(id)] = params.as_slice() else {
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
            ("chic_rt", "has_pending_exception") => {
                if let Some(func_index) = self
                    .module
                    .exports
                    .get("chic_rt_has_pending_exception")
                    .copied()
                {
                    let mut values = self.invoke(func_index, &[])?;
                    if values.len() != 1 {
                        return Err(WasmExecutionError {
                            message:
                                "chic_rt_has_pending_exception returned unexpected value count"
                                    .into(),
                        });
                    }
                    let value = values.pop().unwrap_or(Value::I32(0));
                    let result = match value {
                        Value::I32(v) => v,
                        Value::I64(v) => v as i32,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "chic_rt_has_pending_exception returned non-integer"
                                    .into(),
                            });
                        }
                    };
                    Ok(Some(Value::I32(result)))
                } else {
                    Ok(Some(Value::I32(i32::from(
                        self.pending_exception.is_some(),
                    ))))
                }
            }
            ("chic_rt", "take_pending_exception") => {
                let [Value::I32(payload_ptr), Value::I32(type_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.take_pending_exception expects two i32 out-pointers"
                            .into(),
                    });
                };
                if let Some(func_index) = self
                    .module
                    .exports
                    .get("chic_rt_take_pending_exception")
                    .copied()
                {
                    let mut values = self.invoke(
                        func_index,
                        &[Value::I32(*payload_ptr), Value::I32(*type_ptr)],
                    )?;
                    self.pending_exception = None;
                    if values.len() != 1 {
                        return Err(WasmExecutionError {
                            message:
                                "chic_rt_take_pending_exception returned unexpected value count"
                                    .into(),
                        });
                    }
                    let value = values.pop().unwrap_or(Value::I32(0));
                    match value {
                        Value::I32(v) => Ok(Some(Value::I32(v))),
                        Value::I64(v) => Ok(Some(Value::I32(v as i32))),
                        _ => Err(WasmExecutionError {
                            message: "chic_rt_take_pending_exception returned non-integer".into(),
                        }),
                    }
                } else if let Some(exc) = self.pending_exception.take() {
                    if *payload_ptr != 0 {
                        self.write_u64(*payload_ptr as u32, exc.payload)?;
                    }
                    if *type_ptr != 0 {
                        self.write_u64(*type_ptr as u32, exc.type_id)?;
                    }
                    Ok(Some(Value::I32(1)))
                } else {
                    Ok(Some(Value::I32(0)))
                }
            }
            ("chic_rt", "startup_call_entry_async") => {
                let [
                    Value::I64(function_ptr),
                    Value::I32(_flags),
                    Value::I32(_argc),
                    Value::I64(_argv),
                    Value::I64(_envp),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.startup_call_entry_async expects (i64, i32, i32, i64, i64) arguments".into(),
                    });
                };
                let func_index = *function_ptr as i32;
                if func_index < 0 {
                    return Err(WasmExecutionError {
                        message: "startup_call_entry_async received negative function pointer"
                            .into(),
                    });
                }
                let task_ptr = self.call_async_function(func_index as u32, &[])? as i64;
                Ok(Some(Value::I64(task_ptr)))
            }
            ("chic_rt", "startup_complete_entry_async") => {
                let [Value::I64(task_ptr), Value::I32(_flags)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.startup_complete_entry_async expects (i64, i32) arguments"
                                .into(),
                    });
                };
                let ptr = *task_ptr as i32;
                if ptr == 0 {
                    return Ok(Some(Value::I32(1)));
                }
                let value = self.await_future_blocking(ptr as u32, None)?;
                Ok(Some(Value::I32(value)))
            }
            ("chic_rt", "startup_call_testcase_async") => {
                let [Value::I64(function_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.startup_call_testcase_async expects a single i64 argument"
                                .into(),
                    });
                };
                let func_index = *function_ptr as i32;
                if func_index < 0 {
                    return Err(WasmExecutionError {
                        message: "startup_call_testcase_async received negative function pointer"
                            .into(),
                    });
                }
                let task_ptr = self.call_async_function(func_index as u32, &[])? as i64;
                Ok(Some(Value::I64(task_ptr)))
            }
            ("chic_rt", "startup_complete_testcase_async") => {
                let [Value::I64(task_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.startup_complete_testcase_async expects a single i64 argument"
                                .into(),
                    });
                };
                let ptr = *task_ptr as i32;
                if ptr == 0 {
                    return Ok(Some(Value::I32(1)));
                }
                let value =
                    self.await_future_blocking(ptr as u32, Some(self.async_layout.bool_size))?;
                Ok(Some(Value::I32(value)))
            }
            ("chic_rt", "async_cancel") => {
                let [Value::I32(future)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_cancel expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*future).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_cancel received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_cancel ptr={:#x}", ptr);
                }
                self.cancel_future(ptr)?;
                Ok(Some(Value::I32(AwaitStatus::Ready as i32)))
            }
            ("chic_rt", "async_scope") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_scope expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_scope received negative pointer".into(),
                })?;
                let status = self.await_future_once(base)?;
                if matches!(status, AwaitStatus::Ready) {
                    return Ok(Some(Value::I32(status as i32)));
                }
                let status = self.await_future_blocking(base, None)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "async_block_on") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_block_on expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_block_on received negative pointer".into(),
                })?;
                let status = self.await_future_blocking(base, None)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "async_spawn_local") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_spawn_local expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_spawn_local received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_spawn_local ptr={:#x}", base);
                }
                let status = self.await_future_once(base)?;
                Ok(Some(Value::I32(status as i32)))
            }
            ("chic_rt", "async_spawn") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_spawn expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_spawn received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_spawn ptr={:#x}", base);
                }
                let status = self.await_future_once(base)?;
                Ok(Some(Value::I32(status as i32)))
            }
            ("chic_rt", "async_task_header") => {
                let [Value::I32(task_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_task_header expects a single i32 argument".into(),
                    });
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_task_header {:#x}", task_ptr);
                }
                Ok(Some(Value::I32(*task_ptr)))
            }
            ("chic_rt", "async_task_result") => {
                let [
                    Value::I32(src_ptr),
                    Value::I32(out_ptr),
                    Value::I32(out_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_task_result expects (i32 src, i32 ptr, i32 len)"
                            .into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_task_result received negative src pointer".into(),
                })?;
                let out = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_task_result received negative out pointer".into(),
                })?;
                let len = u32::try_from(*out_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_task_result received negative length".into(),
                })?;
                let align = if self.options.async_result_len == Some(len) {
                    self.options.async_result_align
                } else {
                    None
                };
                let result_offset = self.async_layout.result_offset(len, align);
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!(
                        "[wasm-async] async_task_result src={:#x} dst={:#x} len={} result_offset={} current={:?}",
                        src, out, len, result_offset, self.current_future
                    );
                }
                let layout = self.async_layout;
                let mut observed_offset = None;
                let mut inferred_base = self
                    .async_nodes
                    .keys()
                    .copied()
                    .filter(|base| src >= *base)
                    .map(|base| (base, src - base))
                    .filter(|(_, diff)| *diff < 512)
                    .min_by_key(|(_, diff)| *diff)
                    .map(|(base, diff)| {
                        observed_offset = Some(diff);
                        base
                    });
                if inferred_base.is_none() {
                    inferred_base = src
                        .checked_sub(layout.task_inner_future_offset + result_offset)
                        .or_else(|| src.checked_sub(result_offset));
                    if inferred_base
                        .map(|base| !self.async_nodes.contains_key(&base))
                        .unwrap_or(true)
                    {
                        let mut best: Option<(u32, u32)> = None;
                        for base in self.async_nodes.keys().copied() {
                            if src > base {
                                let diff = src - base;
                                if diff < 512 {
                                    if best.map(|(_, d)| diff < d).unwrap_or(true) {
                                        best = Some((base, diff));
                                    }
                                }
                            }
                        }
                        if let Some((base, diff)) = best {
                            inferred_base = Some(base);
                            observed_offset = Some(diff);
                        }
                    }
                }
                if let Some(base) = inferred_base {
                    if observed_offset.is_none() {
                        observed_offset = src.checked_sub(base);
                    }
                }
                let invalid_src = src == 0 || inferred_base == Some(0);
                let data = if invalid_src {
                    vec![0u8; len as usize]
                } else {
                    match self.read_bytes(src, len) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            let export_value = inferred_base.and_then(|base| {
                                if len <= 4 {
                                    if let Some(idx) =
                                        self.module.exports.get("chic_rt_async_task_bool_result")
                                    {
                                        if let Ok(values) =
                                            self.invoke(*idx, &[Value::I32(base as i32)])
                                        {
                                            if let Some(Value::I32(value)) = values.first().copied()
                                            {
                                                return Some(if len == 1 && value != 0 {
                                                    vec![1u8]
                                                } else {
                                                    value.to_le_bytes().to_vec()
                                                });
                                            }
                                        }
                                    }
                                    if let Some(idx) =
                                        self.module.exports.get("chic_rt_async_task_int_result")
                                    {
                                        if let Ok(values) =
                                            self.invoke(*idx, &[Value::I32(base as i32)])
                                        {
                                            if let Some(Value::I32(value)) = values.first().copied()
                                            {
                                                return Some(value.to_le_bytes().to_vec());
                                            }
                                        }
                                    }
                                }
                                None
                            });
                            export_value.unwrap_or_else(|| vec![0u8; len as usize])
                        }
                    }
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    let window_start = src.saturating_sub(32);
                    if let Ok(window) = self.read_bytes(window_start, 96) {
                        eprintln!(
                            "[wasm-async] mem[{:#x}..{:#x}]={:?}",
                            window_start,
                            window_start + 96,
                            window
                        );
                    }
                }
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!(
                        "[wasm-async] async_task_result bytes={:?} inferred_base={:?}",
                        data, inferred_base
                    );
                }
                self.store_bytes(out, 0, &data)?;
                if invalid_src {
                    return Ok(Some(Value::I32(AwaitStatus::Ready as i32)));
                }
                if let Some(current) = self.current_future {
                    // Populate the result slot on the current future so awaiters can read it directly.
                    let offset = observed_offset.unwrap_or(result_offset);
                    let _ = self.store_bytes(current + offset, 0, &data);
                    let _ = self.store_bytes(
                        current + layout.task_inner_future_offset + offset,
                        0,
                        &data,
                    );
                    if let Some(node) = self.async_nodes.get_mut(&current) {
                        node.result_offset = Some(offset);
                    }
                } else {
                    let candidates = inferred_base.into_iter().chain(
                        [
                            src.checked_sub(result_offset),
                            src.checked_sub(layout.task_inner_future_offset + result_offset),
                        ]
                        .into_iter()
                        .flatten(),
                    );
                    for base in candidates {
                        if self.async_nodes.contains_key(&base)
                            || self
                                .read_u32(base + layout.future_header_flags_offset)
                                .is_ok()
                        {
                            let offset = observed_offset.unwrap_or(result_offset);
                            let _ = self.store_bytes(base + offset, 0, &data);
                            let _ = self.store_bytes(
                                base + layout.task_inner_future_offset + offset,
                                0,
                                &data,
                            );
                            if let Some(node) = self.async_nodes.get_mut(&base) {
                                node.result_offset = Some(offset);
                            }
                            if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                                eprintln!(
                                    "[wasm-async] async_task_result inferred base={:#x} from src={:#x} offset={}",
                                    base, src, offset
                                );
                            }
                            break;
                        }
                    }
                }
                Ok(Some(Value::I32(AwaitStatus::Ready as i32)))
            }
            ("chic_rt", "async_token_new") => {
                let ptr = self.allocate_heap_block(1, 1)?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_new -> {:#x}", ptr);
                }
                self.store_bytes(ptr, 0, &[0])?;
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "async_token_state") => {
                let [Value::I32(state_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_token_state expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*state_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_token_state received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_state ptr={:#x}", ptr);
                }
                let value = self.read_bytes(ptr, 1)?.first().copied().unwrap_or(0);
                Ok(Some(Value::I32(value as i32)))
            }
            ("chic_rt", "async_token_cancel") => {
                let [Value::I32(state_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_token_cancel expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*state_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_token_cancel received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_cancel ptr={:#x}", ptr);
                }
                self.store_bytes(ptr, 0, &[1])?;
                Ok(Some(Value::I32(1)))
            }
            ("chic_rt", "throw") => {
                let [Value::I32(payload), Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_throw expects (i32, i64) arguments".into(),
                    });
                };
                let payload_u32 = payload.saturating_abs() as u32;
                let type_id_bits = *type_id as u64;
                if std::env::var_os("CHIC_DEBUG_WASM_THROW").is_some() {
                    eprintln!(
                        "[wasm-throw] {} payload=0x{payload_u32:08x} type=0x{type_id_bits:016x}",
                        self.current_wasm_context()
                    );
                }
                let thrown = crate::runtime::error::RuntimeThrownException {
                    payload: payload_u32 as u64,
                    type_id: type_id_bits,
                };
                self.pending_exception = Some(thrown);
                if let Some(hook) = &self.options.error_hook {
                    hook(thrown);
                }
                if let Some(func_index) = self.module.exports.get("chic_rt_throw").copied() {
                    let _ = self.invoke(
                        func_index,
                        &[
                            Value::I64(payload_u32 as i64),
                            Value::I64(type_id_bits as i64),
                        ],
                    )?;
                }
                Ok(None)
            }
            ("chic_rt", "await") => {
                let start = Instant::now();
                let [Value::I32(_ctx), Value::I32(future)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.await expects (i32, i32) arguments".into(),
                    });
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] await hook future_ptr={:#x}", future);
                    if let Ok(base) = u32::try_from(*future) {
                        let window_start = base.saturating_sub(16);
                        if let Ok(window) = self.read_bytes(window_start, 96) {
                            eprintln!(
                                "[wasm-async] await hook mem[{:#x}..{:#x}]={:?}",
                                window_start,
                                window_start + 96,
                                window
                            );
                        }
                    }
                }
                let prev = self.switch_borrow_context(Some(*future as u32));
                let status = self.await_future_once(*future as u32)?;
                self.switch_borrow_context(prev);
                tracer.record_await("chic_rt.await", start.elapsed())?;
                Ok(Some(Value::I32(status as i32)))
            }
            ("chic_rt", "yield") => {
                let [Value::I32(_ctx)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.yield expects a single i32 argument".into(),
                    });
                };
                let status = self.yield_current();
                Ok(Some(Value::I32(status as i32)))
            }
            ("chic_rt", "borrow_shared") => {
                let [Value::I32(borrow_id), Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_shared expects (i32, i32) arguments".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.borrow_shared received negative address".into(),
                })?;
                self.register_borrow(*borrow_id, address, BorrowRuntimeKind::Shared)?;
                Ok(None)
            }
            ("chic_rt", "borrow_unique") => {
                let [Value::I32(borrow_id), Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_unique expects (i32, i32) arguments".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.borrow_unique received negative address".into(),
                })?;
                self.register_borrow(*borrow_id, address, BorrowRuntimeKind::Unique)?;
                Ok(None)
            }
            ("chic_rt", "borrow_release") => {
                let [Value::I32(borrow_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_release expects a single i32 argument".into(),
                    });
                };
                self.release_borrow(*borrow_id)?;
                Ok(None)
            }
            ("chic_rt", "drop_invoke") => {
                let [Value::I32(func), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_invoke expects (i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *value == 0 {
                    return Ok(None);
                }
                let func_index = value_as_u32(&Value::I32(*func), "chic_rt.drop_invoke func")?;
                let value_ptr = value_as_ptr_u32(&Value::I32(*value), "chic_rt.drop_invoke value")?;
                if std::env::var_os("CHIC_DEBUG_WASM_DROP_INVOKE").is_some() {
                    let total = self.module.imports.len() + self.module.functions.len();
                    let mut current_exports = Vec::new();
                    if let Some(current) = self.current_function {
                        for (name, &idx) in &self.module.exports {
                            if idx == current {
                                current_exports.push(name.as_str());
                            }
                        }
                    }
                    eprintln!(
                        "[wasm-drop-invoke] func_index={} value_ptr=0x{value_ptr:08x} total_funcs={} current={:?} current_exports={:?} call_stack={:?}",
                        func_index, total, self.current_function, current_exports, self.call_stack
                    );
                }
                let _ = self.invoke(func_index, &[Value::I32(value_ptr as i32)])?;
                Ok(None)
            }
            ("chic_rt", "hash_invoke") => {
                let [Value::I32(func), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hash_invoke expects (i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *value == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let func_index = u32::try_from(*func).map_err(|_| WasmExecutionError {
                    message: "chic_rt.hash_invoke received negative function pointer".into(),
                })?;
                let value_ptr = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.hash_invoke received negative value pointer".into(),
                })?;
                let result = self.invoke(func_index, &[Value::I32(value_ptr as i32)])?;
                let value = result.first().copied();
                match value {
                    Some(Value::I64(hash)) => Ok(Some(Value::I64(hash))),
                    Some(other) => Err(WasmExecutionError {
                        message: format!(
                            "chic_rt.hash_invoke expected i64 result, found {other:?}"
                        ),
                    }),
                    None => Ok(Some(Value::I64(0))),
                }
            }
            ("chic_rt", "eq_invoke") => {
                let [Value::I32(func), Value::I32(left), Value::I32(right)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.eq_invoke expects (i32, i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *left == 0 || *right == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let func_index = u32::try_from(*func).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative function pointer".into(),
                })?;
                let left_ptr = u32::try_from(*left).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative left pointer".into(),
                })?;
                let right_ptr = u32::try_from(*right).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative right pointer".into(),
                })?;
                let result = self.invoke(
                    func_index,
                    &[Value::I32(left_ptr as i32), Value::I32(right_ptr as i32)],
                )?;
                let value = result.first().copied();
                match value {
                    Some(Value::I32(value)) => Ok(Some(Value::I32(value))),
                    Some(other) => Err(WasmExecutionError {
                        message: format!("chic_rt.eq_invoke expected i32 result, found {other:?}"),
                    }),
                    None => Ok(Some(Value::I32(0))),
                }
            }
            ("chic_rt", "drop_resource") => {
                let [Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_resource expects a single i32 argument".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.drop_resource received negative address".into(),
                })?;
                self.drop_resource(address)?;
                Ok(None)
            }
            ("chic_rt", "type_metadata") | ("chic_rt", "chic_rt_type_metadata") => {
                let [Value::I64(type_id), Value::I32(out_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_metadata expects (i64 type_id, i32 out_ptr)".into(),
                    });
                };
                let out = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.type_metadata received negative out pointer".into(),
                })?;
                if out == 0 {
                    return Ok(Some(Value::I32(2)));
                }
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let Some(meta) = self
                    .type_metadata
                    .get(&key)
                    .or_else(|| self.type_metadata.get(&swapped))
                else {
                    return Ok(Some(Value::I32(1)));
                };
                let size = meta.size as i32;
                let align = meta.align as i32;
                let _ = self.store_i32(out, 0, size);
                let _ = self.store_i32(out, 4, align);
                let _ = self.store_i32(out, 8, 0);
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "type_size") | ("chic_rt", "chic_rt_type_size") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_size expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let size = self
                    .type_metadata
                    .get(&key)
                    .or_else(|| self.type_metadata.get(&swapped))
                    .map(|meta| meta.size as i32)
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] size[{idx}] type_id=0x{:016X} -> {} (meta_entries={})",
                            key,
                            size,
                            self.type_metadata.len()
                        );
                    }
                }
                Ok(Some(Value::I32(size)))
            }
            ("chic_rt", "type_align") | ("chic_rt", "chic_rt_type_align") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_align expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let align = self
                    .type_metadata
                    .get(&key)
                    .or_else(|| self.type_metadata.get(&swapped))
                    .map(|meta| meta.align as i32)
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] align[{idx}] type_id=0x{:016X} -> {} (meta_entries={})",
                            key,
                            align,
                            self.type_metadata.len()
                        );
                    }
                }
                Ok(Some(Value::I32(align)))
            }
            ("chic_rt", "type_drop_glue") | ("chic_rt", "chic_rt_type_drop_glue") => {
                let [Value::I64(_type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_drop_glue expects a single i64 argument".into(),
                    });
                };
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "type_clone_glue") | ("chic_rt", "chic_rt_type_clone_glue") => {
                let [Value::I64(_type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_clone_glue expects a single i64 argument".into(),
                    });
                };
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "type_hash_glue") | ("chic_rt", "chic_rt_type_hash_glue") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_hash_glue expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let glue = self
                    .hash_glue
                    .get(&key)
                    .or_else(|| self.hash_glue.get(&swapped))
                    .copied()
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] hash_glue[{idx}] type_id=0x{:016X} -> 0x{glue:08X} (hash_entries={})",
                            key,
                            self.hash_glue.len()
                        );
                    }
                }
                Ok(Some(Value::I32(glue as i32)))
            }
            ("chic_rt", "type_eq_glue") | ("chic_rt", "chic_rt_type_eq_glue") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_eq_glue expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let glue = self
                    .eq_glue
                    .get(&key)
                    .or_else(|| self.eq_glue.get(&swapped))
                    .copied()
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] eq_glue[{idx}] type_id=0x{:016X} -> 0x{glue:08X} (eq_entries={})",
                            key,
                            self.eq_glue.len()
                        );
                    }
                }
                Ok(Some(Value::I32(glue as i32)))
            }
            ("chic_rt", "trace_enter") => {
                let [
                    Value::I64(trace_id),
                    Value::I32(label_ptr),
                    Value::I64(len),
                    Value::I64(_cpu_budget),
                    Value::I64(_mem_budget),
                    Value::I64(_gpu_budget),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.trace_enter expects (i64, i32, i64, i64, i64, i64) arguments (budgets currently ignored)"
                                .into(),
                    });
                };
                let ptr = u32::try_from(*label_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.trace_enter received negative label pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.trace_enter received negative label length".into(),
                })?;
                let label = self.read_bytes(ptr, len)?;
                unsafe {
                    chic_rt_trace_enter(*trace_id as u64, label.as_ptr(), label.len() as u64);
                }
                Ok(None)
            }
            ("chic_rt", "trace_exit") => {
                let [Value::I64(trace_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.trace_exit expects a single i64 argument".into(),
                    });
                };
                unsafe {
                    chic_rt_trace_exit(*trace_id as u64);
                }
                Ok(None)
            }
            ("chic_rt", "trace_flush") => {
                let [Value::I32(path_ptr), Value::I64(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.trace_flush expects (i32, i64) arguments".into(),
                    });
                };
                let path = if *path_ptr == 0 || *len == 0 {
                    None
                } else {
                    let ptr = *path_ptr as u32;
                    let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                        message: "chic_rt.trace_flush received negative path length".into(),
                    })?;
                    Some(self.read_bytes(ptr, len)?)
                };
                let status = if let Some(bytes) = path {
                    unsafe { chic_rt_trace_flush(bytes.as_ptr(), bytes.len() as u64) }
                } else {
                    unsafe { chic_rt_trace_flush(std::ptr::null(), 0) }
                };
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "span_from_raw_mut") | ("chic_rt", "chic_rt_span_from_raw_mut") => {
                let [Value::I32(out_ptr), data_ptr, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_from_raw_mut expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let data_ptr = value_as_ptr_u32(data_ptr, "chic_rt.span_from_raw_mut data")?;
                let len = value_as_u32(len, "chic_rt.span_from_raw_mut length")?;
                let (data_ptr, elem_size, elem_align) = if data_ptr == 0 {
                    (0, 0, 1)
                } else {
                    let (ptr, size, align) = self.read_value_ptr(data_ptr)?;
                    if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                        eprintln!(
                            "[wasm-span] from_raw_mut handle=0x{data_ptr:08x} ptr=0x{ptr:08x} size={} align={} len={}",
                            size, align, len
                        );
                    }
                    (ptr, size, align)
                };
                self.write_span_ptr(out_ptr, data_ptr, len, elem_size, elem_align)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "span_from_raw_const") | ("chic_rt", "chic_rt_span_from_raw_const") => {
                let [Value::I32(out_ptr), data_ptr, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_from_raw_const expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let data_ptr = value_as_ptr_u32(data_ptr, "chic_rt.span_from_raw_const data")?;
                let len = value_as_u32(len, "chic_rt.span_from_raw_const length")?;
                let (data_ptr, elem_size, elem_align) = if data_ptr == 0 {
                    (0, 0, 1)
                } else {
                    let (ptr, size, align) = self.read_value_ptr(data_ptr)?;
                    if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                        eprintln!(
                            "[wasm-span] from_raw_const handle=0x{data_ptr:08x} ptr=0x{ptr:08x} size={} align={} len={}",
                            size, align, len
                        );
                    }
                    (ptr, size, align)
                };
                self.write_span_ptr(out_ptr, data_ptr, len, elem_size, elem_align)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "span_slice_mut") | ("chic_rt", "chic_rt_span_slice_mut") => {
                let [Value::I32(source_ptr), start, length, Value::I32(dest_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_slice_mut expects (i32, i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_mut received negative source pointer".into(),
                })?;
                let start = value_as_u32(start, "chic_rt.span_slice_mut start")?;
                let length = value_as_u32(length, "chic_rt.span_slice_mut length")?;
                let dest_ptr = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_mut received negative destination pointer".into(),
                })?;
                if source_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                }
                let (data_ptr, span_len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if start > span_len {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                if length > span_len - start {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                let validation = span_validate_stride(elem_size, elem_align);
                if validation != SpanError::Success {
                    return Ok(Some(Value::I32(validation as i32)));
                }
                let mut new_ptr = data_ptr;
                if elem_size != 0 {
                    let offset = match start.checked_mul(elem_size) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                    new_ptr = match data_ptr.checked_add(offset) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                }
                if length == 0 || elem_size == 0 {
                    new_ptr = SPAN_DANGLING_PTR;
                } else if new_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                } else if elem_align > 1 && new_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(SpanError::InvalidStride as i32)));
                }
                self.write_span_ptr(dest_ptr, new_ptr, length, elem_size, elem_align)?;
                Ok(Some(Value::I32(SpanError::Success as i32)))
            }
            ("chic_rt", "span_slice_readonly") | ("chic_rt", "chic_rt_span_slice_readonly") => {
                let [Value::I32(source_ptr), start, length, Value::I32(dest_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.span_slice_readonly expects (i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_readonly received negative source pointer".into(),
                })?;
                let start = value_as_u32(start, "chic_rt.span_slice_readonly start")?;
                let length = value_as_u32(length, "chic_rt.span_slice_readonly length")?;
                let dest_ptr = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_readonly received negative destination pointer"
                        .into(),
                })?;
                if source_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                }
                let (data_ptr, span_len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if start > span_len {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                if length > span_len - start {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                let validation = span_validate_stride(elem_size, elem_align);
                if validation != SpanError::Success {
                    return Ok(Some(Value::I32(validation as i32)));
                }
                let mut new_ptr = data_ptr;
                if elem_size != 0 {
                    let offset = match start.checked_mul(elem_size) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                    new_ptr = match data_ptr.checked_add(offset) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                }
                if length == 0 || elem_size == 0 {
                    new_ptr = SPAN_DANGLING_PTR;
                } else if new_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                } else if elem_align > 1 && new_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(SpanError::InvalidStride as i32)));
                }
                self.write_span_ptr(dest_ptr, new_ptr, length, elem_size, elem_align)?;
                Ok(Some(Value::I32(SpanError::Success as i32)))
            }
            ("chic_rt", "span_to_readonly") | ("chic_rt", "chic_rt_span_to_readonly") => {
                let [Value::I32(out_ptr), Value::I32(source_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_to_readonly expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_to_readonly received negative return pointer".into(),
                })?;
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_to_readonly received negative source pointer".into(),
                })?;
                if source_ptr == 0 {
                    self.write_span_ptr(out_ptr, 0, 0, 0, 0)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let (data_ptr, len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                self.write_span_ptr(out_ptr, data_ptr, len, elem_size, elem_align)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "span_ptr_at_mut") | ("chic_rt", "chic_rt_span_ptr_at_mut") => {
                let [Value::I32(source_ptr), index] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_ptr_at_mut expects (i32, i32) arguments".into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_ptr_at_mut received negative source pointer".into(),
                })?;
                let index = value_as_u32(index, "chic_rt.span_ptr_at_mut index")?;
                if source_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (data_ptr, len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                    eprintln!(
                        "[wasm-span] ptr_at_mut span=0x{source_ptr:08x} index={} data=0x{data_ptr:08x} len={} elem_size={} elem_align={}",
                        index, len, elem_size, elem_align
                    );
                }
                if index >= len {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_size == 0 {
                    return Ok(Some(Value::I32(SPAN_DANGLING_PTR as i32)));
                }
                if data_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_align > 1 && data_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let offset = match index.checked_mul(elem_size) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                let ptr = match data_ptr.checked_add(offset) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "span_ptr_at_readonly") | ("chic_rt", "chic_rt_span_ptr_at_readonly") => {
                let [Value::I32(source_ptr), index] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_ptr_at_readonly expects (i32, i32) arguments".into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_ptr_at_readonly received negative source pointer".into(),
                })?;
                let index = value_as_u32(index, "chic_rt.span_ptr_at_readonly index")?;
                if source_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (data_ptr, len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                    eprintln!(
                        "[wasm-span] ptr_at_readonly span=0x{source_ptr:08x} index={} data=0x{data_ptr:08x} len={} elem_size={} elem_align={}",
                        index, len, elem_size, elem_align
                    );
                }
                if index >= len {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_size == 0 {
                    return Ok(Some(Value::I32(SPAN_DANGLING_PTR as i32)));
                }
                if data_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_align > 1 && data_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let offset = match index.checked_mul(elem_size) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                let ptr = match data_ptr.checked_add(offset) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "span_copy_to") => {
                let [
                    Value::I32(src_ptr),
                    Value::I32(src_len),
                    Value::I32(src_size),
                    Value::I32(src_align),
                    Value::I32(dest_ptr),
                    Value::I32(dest_len),
                    Value::I32(dest_size),
                    Value::I32(dest_align),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_copy_to expects (i32, i32, i32, i32, i32, i32, i32, i32) arguments".into(),
                    });
                };
                let src_ptr = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source pointer".into(),
                })?;
                let src_len = u32::try_from(*src_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source length".into(),
                })?;
                let src_size = u32::try_from(*src_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source element size".into(),
                })?;
                let _src_align = u32::try_from(*src_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source alignment".into(),
                })?;
                let dest_ptr = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination pointer".into(),
                })?;
                let dest_len = u32::try_from(*dest_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination length".into(),
                })?;
                let dest_size = u32::try_from(*dest_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination element size"
                        .into(),
                })?;
                let _dest_align = u32::try_from(*dest_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination alignment".into(),
                })?;

                if src_len > dest_len {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                if src_size != dest_size {
                    return Ok(Some(Value::I32(SpanError::InvalidStride as i32)));
                }
                if src_len == 0 || src_size == 0 {
                    return Ok(Some(Value::I32(SpanError::Success as i32)));
                }
                if src_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                }
                let byte_len = match src_len.checked_mul(src_size) {
                    Some(len) => len,
                    None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                };
                let data = self.read_bytes(src_ptr, byte_len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                Ok(Some(Value::I32(SpanError::Success as i32)))
            }
            ("chic_rt", "string_new") => {
                let [Value::I32(out_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_new expects a single i32 argument".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_new received negative return pointer".into(),
                })?;
                self.init_empty_string(out_ptr)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_with_capacity") => {
                let [Value::I32(out_ptr), Value::I32(capacity)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_with_capacity expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_with_capacity received negative return pointer".into(),
                })?;
                let capacity = u32::try_from(*capacity).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_with_capacity received negative capacity".into(),
                })?;
                if capacity == 0 {
                    self.init_empty_string(out_ptr)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let ptr = self.allocate_heap_block(capacity, 1)?;
                self.write_string_repr(
                    out_ptr,
                    WasmStringRepr {
                        ptr,
                        len: 0,
                        cap: capacity,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_from_slice") => {
                let [Value::I32(out_ptr), Value::I32(slice_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_from_slice expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_slice received negative return pointer".into(),
                })?;
                let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_slice received negative slice pointer".into(),
                })?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    let stride = self.ptr_stride();
                    let raw_ptr = self.read_u32(slice_ptr).ok();
                    let raw_len = self.read_u32(slice_ptr + stride).ok();
                    let raw_extra = self.read_u32(slice_ptr + stride * 2).ok();
                    let raw_tail = self.read_u32(slice_ptr + stride * 3).ok();
                    let raw_u64_lo = self.read_u64(slice_ptr).ok();
                    let raw_u64_hi = self.read_u64(slice_ptr + 8).ok();
                    eprintln!(
                        "[wasm-string] string_from_slice out=0x{out_ptr:08X} slice_ptr=0x{slice_ptr:08X} stride={} mem_len={} raw_ptr={raw_ptr:?} raw_len={raw_len:?} raw_extra={raw_extra:?} raw_tail={raw_tail:?} raw_u64_lo={raw_u64_lo:?} raw_u64_hi={raw_u64_hi:?}",
                        stride,
                        self.memory_len(),
                    );
                }
                let (ptr, len) = self.read_str_ptr(slice_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    eprintln!(
                        "[wasm-string] string_from_slice data_ptr=0x{ptr:08X} len={len} mem_len={}",
                        self.memory_len()
                    );
                }
                if len == 0 || ptr == 0 {
                    self.init_empty_string(out_ptr)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let data = self.read_bytes(ptr, len)?;
                self.store_string_bytes(out_ptr, &data)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_from_char") => {
                let [Value::I32(out_ptr), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_from_char expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_char received negative return pointer".into(),
                })?;
                let code = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_char received negative value".into(),
                })?;
                let mut buf = [0u8; 4];
                let slice = if let Some(ch) = char::from_u32(code) {
                    let encoded = ch.encode_utf8(&mut buf);
                    encoded.as_bytes()
                } else {
                    &[][..]
                };
                self.store_string_bytes(out_ptr, slice)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_push_slice") => {
                let [Value::I32(target_ptr), Value::I32(slice_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_push_slice expects (i32, i32) arguments".into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_push_slice received negative target pointer".into(),
                })?;
                let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_push_slice received negative slice pointer".into(),
                })?;
                let (ptr, len) = self.read_str_ptr(slice_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    eprintln!(
                        "[wasm-string] string_push_slice target=0x{target_ptr:08X} slice_ptr=0x{slice_ptr:08X} data_ptr=0x{ptr:08X} len={len} mem_len={}",
                        self.memory_len()
                    );
                }
                let data = if len == 0 || ptr == 0 {
                    Vec::new()
                } else {
                    self.read_bytes(ptr, len)?
                };
                let status = self.append_string_bytes(target_ptr, &data)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_truncate") => {
                let [Value::I32(target_ptr), Value::I32(new_len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_truncate expects (i32, i32) arguments".into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_truncate received negative target pointer".into(),
                })?;
                let new_len = u32::try_from(*new_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_truncate received negative length".into(),
                })?;
                let mut repr = self.read_string_repr(target_ptr)?;
                let inline = self.is_inline_string(&repr);
                if repr.len > 0 && repr.ptr == 0 && !inline {
                    return Ok(Some(Value::I32(STRING_INVALID_POINTER)));
                }
                if new_len > repr.len {
                    return Ok(Some(Value::I32(STRING_OUT_OF_BOUNDS)));
                }
                repr.len = new_len;
                self.write_string_repr(target_ptr, repr)?;
                Ok(Some(Value::I32(STRING_SUCCESS)))
            }
            ("chic_rt", "string_reserve") => {
                let [Value::I32(target_ptr), Value::I32(additional)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_reserve expects (i32, i32) arguments".into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_reserve received negative target pointer".into(),
                })?;
                let additional = u32::try_from(*additional).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_reserve received negative additional".into(),
                })?;
                if additional == 0 {
                    return Ok(Some(Value::I32(STRING_SUCCESS)));
                }
                let mut repr = self.read_string_repr(target_ptr)?;
                let inline = self.is_inline_string(&repr);
                if repr.len > 0 && repr.ptr == 0 && !inline {
                    return Ok(Some(Value::I32(STRING_INVALID_POINTER)));
                }
                let new_len = match repr.len.checked_add(additional) {
                    Some(len) => len,
                    None => return Ok(Some(Value::I32(STRING_CAPACITY_OVERFLOW))),
                };
                let capacity = if inline {
                    STRING_INLINE_CAPACITY
                } else {
                    repr.cap
                };
                if new_len > capacity {
                    let new_ptr = self.allocate_heap_block(new_len, 1)?;
                    if repr.len > 0 {
                        let existing_ptr = if inline {
                            self.inline_string_ptr(target_ptr)?
                        } else {
                            repr.ptr
                        };
                        let existing = self.read_bytes(existing_ptr, repr.len)?;
                        self.store_bytes(new_ptr, 0, &existing)?;
                    }
                    repr.ptr = new_ptr;
                    repr.cap = new_len;
                }
                self.write_string_repr(target_ptr, repr)?;
                Ok(Some(Value::I32(STRING_SUCCESS)))
            }
            ("chic_rt", "string_error_message") => {
                let [Value::I32(out_ptr), Value::I32(code)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_error_message expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_error_message received negative return pointer".into(),
                })?;
                let message = match *code {
                    STRING_UTF8 => "utf8",
                    STRING_CAPACITY_OVERFLOW => "capacity overflow",
                    STRING_ALLOCATION_FAILED => "allocation failed",
                    STRING_INVALID_POINTER => "invalid pointer",
                    STRING_OUT_OF_BOUNDS => "out of bounds",
                    _ => "success",
                };
                let bytes = message.as_bytes();
                let len = u32::try_from(bytes.len()).unwrap_or(0);
                if len == 0 {
                    self.write_str_ptr(out_ptr, 0, 0)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let ptr = self.allocate_heap_block(len, 1)?;
                self.store_bytes(ptr, 0, bytes)?;
                self.write_str_ptr(out_ptr, ptr, len)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_debug_ping") => Ok(Some(Value::I32(42))),
            ("chic_rt", "string_get_ptr") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_get_ptr expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_get_ptr received negative pointer".into(),
                })?;
                let repr = self.read_string_repr(base)?;
                if repr.len == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let data_ptr = if self.is_inline_string(&repr) {
                    self.inline_string_ptr(base)?
                } else {
                    repr.ptr
                };
                Ok(Some(Value::I32(data_ptr as i32)))
            }
            ("chic_rt", "string_get_len") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_get_len expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_get_len received negative pointer".into(),
                })?;
                let repr = self.read_string_repr(base)?;
                Ok(Some(Value::I32(repr.len as i32)))
            }
            ("chic_rt", "string_get_cap") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_get_cap expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_get_cap received negative pointer".into(),
                })?;
                let repr = self.read_string_repr(base)?;
                Ok(Some(Value::I32(repr.cap as i32)))
            }
            ("chic_rt", "string_set_ptr") => {
                let [Value::I32(ptr), Value::I32(data_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_set_ptr expects (i32, i32) arguments".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_ptr received negative pointer".into(),
                })?;
                let data_ptr = u32::try_from(*data_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_ptr received negative data pointer".into(),
                })?;
                let mut repr = self.read_string_repr(base)?;
                repr.ptr = data_ptr;
                if data_ptr != 0 {
                    repr.cap &= !STRING_INLINE_TAG;
                }
                self.write_string_repr(base, repr)?;
                Ok(None)
            }
            ("chic_rt", "string_set_len") => {
                let [Value::I32(ptr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_set_len expects (i32, i32) arguments".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_len received negative pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_len received negative length".into(),
                })?;
                let mut repr = self.read_string_repr(base)?;
                repr.len = len;
                self.write_string_repr(base, repr)?;
                Ok(None)
            }
            ("chic_rt", "string_set_cap") => {
                let [Value::I32(ptr), Value::I32(cap)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_set_cap expects (i32, i32) arguments".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_cap received negative pointer".into(),
                })?;
                let cap = u32::try_from(*cap).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_cap received negative cap".into(),
                })?;
                let mut repr = self.read_string_repr(base)?;
                repr.cap = cap;
                self.write_string_repr(base, repr)?;
                Ok(None)
            }
            ("chic_rt", "string_inline_ptr") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_inline_ptr expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_inline_ptr received negative pointer".into(),
                })?;
                let inline_ptr = self.inline_string_ptr(base)?;
                Ok(Some(Value::I32(inline_ptr as i32)))
            }
            ("chic_rt", "string_inline_capacity") => {
                Ok(Some(Value::I32(STRING_INLINE_CAPACITY as i32)))
            }
            ("chic_rt", "string_append_slice") => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(slice_ptr),
                    Value::I32(slice_len),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_slice expects (i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_slice received negative target pointer".into(),
                })?;
                let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_slice received negative slice pointer".into(),
                })?;
                let slice_len = u32::try_from(*slice_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_slice received negative slice length".into(),
                })?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    eprintln!(
                        "[wasm-string] string_append_slice target=0x{target_ptr:08X} slice_ptr=0x{slice_ptr:08X} slice_len={slice_len} mem_len={}",
                        self.memory_len()
                    );
                }
                let data = if slice_len == 0 || slice_ptr == 0 {
                    Vec::new()
                } else {
                    self.read_bytes(slice_ptr, slice_len)?
                };
                let status = self.append_string_bytes(target_ptr, &data)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_bool") => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_bool expects (i32, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_bool received negative target pointer".into(),
                })?;
                let text = if *value != 0 { "true" } else { "false" };
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_char") => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_char expects (i32, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_char received negative target pointer".into(),
                })?;
                let code = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_char received negative value".into(),
                })?;
                let mut buf = [0u8; 4];
                let bytes = if let Some(ch) = char::from_u32(code) {
                    let encoded = ch.encode_utf8(&mut buf);
                    encoded.as_bytes()
                } else {
                    &[][..]
                };
                let status = self.append_string_bytes(target_ptr, bytes)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_signed") => {
                let [
                    Value::I32(target_ptr),
                    Value::I64(low),
                    Value::I64(high),
                    Value::I32(_bits),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_signed expects (i32, i64, i64, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_signed received negative target pointer".into(),
                })?;
                let value = ((*high as i128) << 64) | (*low as u64 as i128);
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_unsigned") => {
                let [
                    Value::I32(target_ptr),
                    Value::I64(low),
                    Value::I64(high),
                    Value::I32(_bits),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_unsigned expects (i32, i64, i64, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_unsigned received negative target pointer"
                        .into(),
                })?;
                let value = ((u128::from(*high as u64)) << 64) | u128::from(*low as u64);
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_f32") => {
                let [
                    Value::I32(target_ptr),
                    Value::F32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_f32 expects (i32, f32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_f32 received negative target pointer".into(),
                })?;
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_f64") => {
                let [
                    Value::I32(target_ptr),
                    Value::F64(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_f64 expects (i32, f64, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_f64 received negative target pointer".into(),
                })?;
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_clone expects (i32, i32) arguments".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone received negative source pointer".into(),
                })?;
                let status = self.clone_string(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_clone_slice") => {
                let [
                    Value::I32(dest_ptr),
                    Value::I32(slice_ptr),
                    Value::I32(slice_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_clone_slice expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone_slice received negative destination pointer"
                        .into(),
                })?;
                let slice_base = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone_slice received negative slice pointer".into(),
                })?;
                let slice_len = u32::try_from(*slice_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone_slice received negative slice length".into(),
                })?;
                let status = self.clone_string_from_slice(dest, slice_base, slice_len)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_drop") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_drop expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_drop received negative pointer".into(),
                })?;
                self.drop_string(ptr)?;
                Ok(None)
            }
            ("chic_rt", "closure_env_alloc") => {
                let [Value::I32(size), Value::I32(align)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.closure_env_alloc expects (i32, i32) arguments".into(),
                    });
                };
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_alloc received negative size".into(),
                })?;
                let align = u32::try_from(*align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_alloc received negative alignment".into(),
                })?;
                let ptr = self.allocate_heap_block(size, align.max(1))?;
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "closure_env_clone") => {
                let [Value::I32(src), Value::I32(size), Value::I32(align)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.closure_env_clone expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let src = u32::try_from(*src).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_clone received negative source pointer".into(),
                })?;
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_clone received negative size".into(),
                })?;
                let align = u32::try_from(*align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_clone received negative alignment".into(),
                })?;
                let dest = self.allocate_heap_block(size, align.max(1))?;
                if size != 0 {
                    let data = self.read_bytes(src, size)?;
                    self.store_bytes(dest, 0, &data)?;
                }
                Ok(Some(Value::I32(dest as i32)))
            }
            ("chic_rt", "closure_env_free") => {
                let [Value::I32(_ptr), Value::I32(_size), Value::I32(_align)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.closure_env_free expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                Ok(None)
            }
            ("chic_rt", "drop_missing") => {
                let [Value::I32(_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_missing expects a single i32 argument".into(),
                    });
                };
                Ok(None)
            }
            ("chic_rt", "vec_drop") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_drop expects a single i32 argument".into(),
                    });
                };
                let ptr = value_as_ptr_u32(&Value::I32(*ptr), "chic_rt.vec_drop ptr")?;
                self.drop_vec(ptr)?;
                Ok(None)
            }
            ("chic_rt", "vec_with_capacity") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(capacity),
                    Value::I32(drop_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_with_capacity expects (i32 out, i32 elem_size, i32 elem_align, i32 capacity, i32 drop_fn) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative return pointer".into(),
                })?;
                let elem_size = u32::try_from(*elem_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative element size".into(),
                })?;
                let elem_align = u32::try_from(*elem_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative element alignment".into(),
                })?;
                let capacity = u32::try_from(*capacity).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative capacity".into(),
                })?;
                let drop_fn = u32::try_from(*drop_fn).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative drop function index"
                        .into(),
                })?;

                let base_align = elem_align.max(1);
                if capacity == 0 || elem_size == 0 {
                    self.write_vec_repr(
                        out_ptr,
                        WasmVecRepr {
                            ptr: 0,
                            len: 0,
                            cap: 0,
                            elem_size,
                            elem_align: base_align,
                            drop_fn,
                        },
                    )?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }

                let bytes = capacity
                    .checked_mul(elem_size)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_with_capacity length overflow".into(),
                    })?;
                let ptr = self.allocate_heap_block(bytes, base_align)?;
                self.write_vec_repr(
                    out_ptr,
                    WasmVecRepr {
                        ptr,
                        len: 0,
                        cap: capacity,
                        elem_size,
                        elem_align: base_align,
                        drop_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "vec_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_clone expects (i32, i32) arguments".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_clone received negative source pointer".into(),
                })?;
                let status = self.clone_vec(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "vec_into_array") | ("chic_rt", "array_into_vec") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_into_array expects (i32 dest, i32 src) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_into_array received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_into_array received negative source pointer".into(),
                })?;
                self.move_vec(dest, src)?;
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "vec_copy_to_array") | ("chic_rt", "array_copy_to_vec") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_copy_to_array expects (i32 dest, i32 src) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_copy_to_array received negative destination pointer"
                        .into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_copy_to_array received negative source pointer".into(),
                })?;
                let status = self.clone_vec(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "hashset_new") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(drop_fn),
                    Value::I32(eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_new expects (i32 out, i32 elem_size, i32 elem_align, i32 drop_fn, i32 eq_fn) arguments"
                                .into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_new out")?;
                let elem_size =
                    value_as_u32(&Value::I32(*elem_size), "chic_rt.hashset_new elem_size")?;
                let elem_align =
                    value_as_u32(&Value::I32(*elem_align), "chic_rt.hashset_new elem_align")?;
                let drop_fn = value_as_u32(&Value::I32(*drop_fn), "chic_rt.hashset_new drop_fn")?;
                let eq_fn = value_as_u32(&Value::I32(*eq_fn), "chic_rt.hashset_new eq_fn")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if std::env::var_os("CHIC_DEBUG_WASM_HASHSET").is_some() {
                    let idx = HASHSET_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 20 {
                        let total = self.module.imports.len() + self.module.functions.len();
                        eprintln!(
                            "[wasm-hashset] new[{idx}] out=0x{out_ptr:08x} elem_size={elem_size} elem_align={elem_align} drop_fn={drop_fn} eq_fn={eq_fn} total_funcs={total}"
                        );
                    }
                }
                self.write_hashset_repr(
                    out_ptr,
                    WasmHashSetRepr {
                        entries: 0,
                        states: 0,
                        hashes: 0,
                        len: 0,
                        cap: 0,
                        tombstones: 0,
                        elem_size,
                        elem_align: elem_align.max(1),
                        drop_fn,
                        eq_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashset_with_capacity") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(capacity),
                    Value::I32(drop_fn),
                    Value::I32(eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_with_capacity expects (i32 out, i32 elem_size, i32 elem_align, i32 cap, i32 drop_fn, i32 eq_fn) arguments"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_with_capacity out")?;
                let elem_size = value_as_u32(
                    &Value::I32(*elem_size),
                    "chic_rt.hashset_with_capacity elem_size",
                )?;
                let elem_align = value_as_u32(
                    &Value::I32(*elem_align),
                    "chic_rt.hashset_with_capacity elem_align",
                )?;
                let capacity =
                    value_as_u32(&Value::I32(*capacity), "chic_rt.hashset_with_capacity cap")?;
                let drop_fn = value_as_u32(
                    &Value::I32(*drop_fn),
                    "chic_rt.hashset_with_capacity drop_fn",
                )?;
                let eq_fn =
                    value_as_u32(&Value::I32(*eq_fn), "chic_rt.hashset_with_capacity eq_fn")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let elem_align = elem_align.max(1);
                let mut repr = WasmHashSetRepr {
                    entries: 0,
                    states: 0,
                    hashes: 0,
                    len: 0,
                    cap: 0,
                    tombstones: 0,
                    elem_size,
                    elem_align,
                    drop_fn,
                    eq_fn,
                };
                let normalized = if capacity == 0 {
                    0
                } else {
                    self.table_round_up_pow2(capacity).unwrap_or(0)
                };
                if normalized != 0 {
                    let entry_bytes = normalized.saturating_mul(elem_size);
                    let hash_bytes = normalized.saturating_mul(8);
                    repr.entries = self.allocate_heap_block(entry_bytes, elem_align)?;
                    repr.states = self.allocate_heap_block(normalized, 1)?;
                    repr.hashes = self.allocate_heap_block(hash_bytes, 8)?;
                    repr.cap = normalized;
                }
                self.write_hashset_repr(out_ptr, repr)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashset_len") => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_len expects (i32 set)".into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_len set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                Ok(Some(Value::I32(repr.len as i32)))
            }
            ("chic_rt", "hashset_capacity") => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_capacity expects (i32 set)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_capacity set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                Ok(Some(Value::I32(repr.cap as i32)))
            }
            ("chic_rt", "hashset_tombstones") => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_tombstones expects (i32 set)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_tombstones set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                Ok(Some(Value::I32(repr.tombstones as i32)))
            }
            ("chic_rt", "hashset_clear") => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_clear expects (i32 set)".into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_clear set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 {
                    repr.len = 0;
                    repr.tombstones = 0;
                    self.write_hashset_repr(set_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.states != 0 && repr.entries != 0 && repr.elem_size != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashset_entry_ptr(repr.entries, repr.elem_size, idx)?;
                            self.hashset_drop_value(&repr, entry_ptr)?;
                        }
                    }
                    self.fill(repr.states, 0, repr.cap, TABLE_STATE_EMPTY)?;
                    if repr.hashes != 0 {
                        let hash_bytes = repr.cap.saturating_mul(8);
                        self.fill(repr.hashes, 0, hash_bytes, 0)?;
                    }
                }
                repr.len = 0;
                repr.tombstones = 0;
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_drop") => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_drop expects (i32 set)".into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_drop set")?;
                if set_ptr == 0 {
                    return Ok(None);
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap != 0 && repr.states != 0 && repr.entries != 0 && repr.elem_size != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashset_entry_ptr(repr.entries, repr.elem_size, idx)?;
                            self.hashset_drop_value(&repr, entry_ptr)?;
                        }
                    }
                }
                repr.entries = 0;
                repr.states = 0;
                repr.hashes = 0;
                repr.len = 0;
                repr.cap = 0;
                repr.tombstones = 0;
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(None)
            }
            ("chic_rt", "hashset_reserve") => {
                let [Value::I32(set_ptr), Value::I32(additional)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_reserve expects (i32 set, i32 additional)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_reserve set")?;
                let additional = value_as_u32(
                    &Value::I32(*additional),
                    "chic_rt.hashset_reserve additional",
                )?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                if !self.table_should_grow(repr.len, repr.tombstones, repr.cap, additional) {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let needed = match repr.len.checked_add(additional) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let doubled = match needed.checked_add(needed) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let desired = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let Some(target) = self.table_round_up_pow2(desired).filter(|v| *v != 0) else {
                    return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                };
                let rebuilt = self.hashset_rehash(&repr, target)?;
                self.write_hashset_repr(set_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_shrink_to") => {
                let [Value::I32(set_ptr), Value::I32(min_capacity)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_shrink_to expects (i32 set, i32 min_capacity)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_shrink_to set")?;
                let min_capacity = value_as_u32(
                    &Value::I32(*min_capacity),
                    "chic_rt.hashset_shrink_to min_capacity",
                )?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                let min_cap = if min_capacity == 0 {
                    0
                } else {
                    self.table_round_up_pow2(min_capacity)
                        .ok_or_else(|| WasmExecutionError {
                            message: "hashset shrink_to min capacity overflow".into(),
                        })?
                };
                let mut desired = min_cap;
                if repr.len != 0 {
                    let doubled = match repr.len.checked_add(repr.len) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let expanded = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    desired = match self.table_round_up_pow2(expanded) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                }
                let target = desired.max(min_cap);
                if target >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if target == 0 {
                    repr.entries = 0;
                    repr.states = 0;
                    repr.hashes = 0;
                    repr.len = 0;
                    repr.cap = 0;
                    repr.tombstones = 0;
                    self.write_hashset_repr(set_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let rebuilt = self.hashset_rehash(&repr, target)?;
                self.write_hashset_repr(set_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_insert") => {
                let [
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(value_ptr),
                    Value::I32(inserted_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_insert expects (i32 set, i64 hash, i32 value, i32 inserted)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_insert set")?;
                let value_ptr =
                    value_as_ptr_u32(&Value::I32(*value_ptr), "chic_rt.hashset_insert value")?;
                let inserted_ptr = value_as_ptr_u32(
                    &Value::I32(*inserted_ptr),
                    "chic_rt.hashset_insert inserted",
                )?;
                if inserted_ptr != 0 {
                    let _ = self.write_u32(inserted_ptr, 0);
                }
                if set_ptr == 0 || value_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (value_data, _, _) = self.read_value_ptr(value_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || self.table_should_grow(repr.len, repr.tombstones, repr.cap, 1) {
                    let status = {
                        let Some(needed) = repr.len.checked_add(1) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(doubled) = needed.checked_add(needed) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(desired) = doubled.checked_add(TABLE_MIN_CAPACITY) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let target = self.table_round_up_pow2(desired).unwrap_or(0);
                        if target == 0 {
                            TABLE_CAPACITY_OVERFLOW
                        } else {
                            let rebuilt = self.hashset_rehash(&repr, target)?;
                            self.write_hashset_repr(set_ptr, rebuilt)?;
                            TABLE_SUCCESS
                        }
                    };
                    if status != TABLE_SUCCESS {
                        return Ok(Some(Value::I32(status)));
                    }
                    repr = self.read_hashset_repr(set_ptr)?;
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, value_data)?;
                if found {
                    if inserted_ptr != 0 {
                        let _ = self.write_u32(inserted_ptr, 0);
                    }
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let dest_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if repr.elem_size != 0 && dest_ptr != 0 && value_data != 0 {
                    let bytes = self.read_bytes(value_data, repr.elem_size)?;
                    self.store_bytes(dest_ptr, 0, &bytes)?;
                }
                let hash_slot = self.hashset_hash_slot(repr.hashes, index)?;
                if hash_slot != 0 {
                    self.write_u64(hash_slot, *hash as u64)?;
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let prior = self.read_u8(state_addr)?;
                if prior == TABLE_STATE_TOMBSTONE && repr.tombstones != 0 {
                    repr.tombstones -= 1;
                }
                self.write_u8(state_addr, TABLE_STATE_FULL)?;
                repr.len = repr.len.saturating_add(1);
                if inserted_ptr != 0 {
                    let _ = self.write_u32(inserted_ptr, 1);
                }
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_replace") => {
                let [
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(value_ptr),
                    Value::I32(dest_ptr),
                    Value::I32(replaced_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_replace expects (i32 set, i64 hash, i32 value, i32 dest, i32 replaced)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_replace set")?;
                let value_ptr =
                    value_as_ptr_u32(&Value::I32(*value_ptr), "chic_rt.hashset_replace value")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_replace dest")?;
                let replaced_ptr = value_as_ptr_u32(
                    &Value::I32(*replaced_ptr),
                    "chic_rt.hashset_replace replaced",
                )?;
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                if set_ptr == 0 || value_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (value_data, _, _) = self.read_value_ptr(value_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || self.table_should_grow(repr.len, repr.tombstones, repr.cap, 1) {
                    let status = {
                        let Some(needed) = repr.len.checked_add(1) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(doubled) = needed.checked_add(needed) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(desired) = doubled.checked_add(TABLE_MIN_CAPACITY) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let target = self.table_round_up_pow2(desired).unwrap_or(0);
                        if target == 0 {
                            TABLE_CAPACITY_OVERFLOW
                        } else {
                            let rebuilt = self.hashset_rehash(&repr, target)?;
                            self.write_hashset_repr(set_ptr, rebuilt)?;
                            TABLE_SUCCESS
                        }
                    };
                    if status != TABLE_SUCCESS {
                        return Ok(Some(Value::I32(status)));
                    }
                    repr = self.read_hashset_repr(set_ptr)?;
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, value_data)?;
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if found {
                    if dest_ptr != 0 {
                        let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                        if out_data != 0 && repr.elem_size != 0 && entry_ptr != 0 {
                            let bytes = self.read_bytes(entry_ptr, repr.elem_size)?;
                            self.store_bytes(out_data, 0, &bytes)?;
                        }
                    }
                    self.hashset_drop_value(&repr, entry_ptr)?;
                    if repr.elem_size != 0 && entry_ptr != 0 && value_data != 0 {
                        let bytes = self.read_bytes(value_data, repr.elem_size)?;
                        self.store_bytes(entry_ptr, 0, &bytes)?;
                    }
                    let hash_slot = self.hashset_hash_slot(repr.hashes, index)?;
                    if hash_slot != 0 {
                        self.write_u64(hash_slot, *hash as u64)?;
                    }
                    if replaced_ptr != 0 {
                        let _ = self.write_u32(replaced_ptr, 1);
                    }
                    self.write_hashset_repr(set_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.elem_size != 0 && entry_ptr != 0 && value_data != 0 {
                    let bytes = self.read_bytes(value_data, repr.elem_size)?;
                    self.store_bytes(entry_ptr, 0, &bytes)?;
                }
                let hash_slot = self.hashset_hash_slot(repr.hashes, index)?;
                if hash_slot != 0 {
                    self.write_u64(hash_slot, *hash as u64)?;
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let prior = self.read_u8(state_addr)?;
                if prior == TABLE_STATE_TOMBSTONE && repr.tombstones != 0 {
                    repr.tombstones -= 1;
                }
                self.write_u8(state_addr, TABLE_STATE_FULL)?;
                repr.len = repr.len.saturating_add(1);
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_contains") => {
                let [Value::I32(set_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_contains expects (i32 set, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_contains set")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_contains key")?;
                if set_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                if key_data == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                let (found, _) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                Ok(Some(Value::I32(if found { 1 } else { 0 })))
            }
            ("chic_rt", "hashset_get_ptr") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_get_ptr expects (i32 out, i32 set, i64 hash, i32 key)"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_get_ptr out")?;
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_get_ptr set")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_get_ptr key")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if set_ptr == 0 || key_ptr == 0 {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                if key_data == 0 || repr.cap == 0 {
                    self.write_value_ptr(out_ptr, 0, repr.elem_size, repr.elem_align.max(1))?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    self.write_value_ptr(out_ptr, 0, repr.elem_size, repr.elem_align.max(1))?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                self.write_value_ptr(out_ptr, entry_ptr, repr.elem_size, repr.elem_align.max(1))?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashset_take") => {
                let [
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                    Value::I32(dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_take expects (i32 set, i64 hash, i32 key, i32 dest)"
                                .into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_take set")?;
                let key_ptr = value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_take key")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_take dest")?;
                if set_ptr == 0 || key_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if out_data != 0 && repr.elem_size != 0 && entry_ptr != 0 {
                    let bytes = self.read_bytes(entry_ptr, repr.elem_size)?;
                    self.store_bytes(out_data, 0, &bytes)?;
                }
                self.hashset_drop_value(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_remove") => {
                let [Value::I32(set_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_remove expects (i32 set, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_remove set")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_remove key")?;
                if set_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(0)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                self.hashset_drop_value(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(1)))
            }
            ("chic_rt", "hashset_take_at") => {
                let [Value::I32(set_ptr), Value::I32(index), Value::I32(dest_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_take_at expects (i32 set, i32 index, i32 dest)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_take_at set")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashset_take_at index")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_take_at dest")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_HASHSET").is_some() {
                    let idx = HASHSET_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 120 {
                        if idx < 5 {
                            if let Ok(bytes) = self.read_bytes(set_ptr, 40) {
                                let mut hex = String::new();
                                for (i, b) in bytes.iter().enumerate() {
                                    if i != 0 {
                                        hex.push(' ');
                                    }
                                    hex.push_str(&format!("{b:02x}"));
                                }
                                eprintln!("[wasm-hashset] take_at[{idx}] raw={hex}");
                                let maybe_ptr =
                                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                                if maybe_ptr != 0 {
                                    if let Ok(deref) = self.read_bytes(maybe_ptr, 40) {
                                        let mut hex2 = String::new();
                                        for (i, b) in deref.iter().enumerate() {
                                            if i != 0 {
                                                hex2.push(' ');
                                            }
                                            hex2.push_str(&format!("{b:02x}"));
                                        }
                                        eprintln!(
                                            "[wasm-hashset] take_at[{idx}] maybe_ptr=0x{maybe_ptr:08x} deref40={hex2}"
                                        );
                                    }
                                }
                            }
                        }
                        eprintln!(
                            "[wasm-hashset] take_at[{idx}] set=0x{set_ptr:08x} index={index} cap={} len={} tombstones={} drop_fn={} eq_fn={}",
                            repr.cap, repr.len, repr.tombstones, repr.drop_fn, repr.eq_fn
                        );
                    }
                }
                if repr.cap == 0 || index >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let state = self.read_u8(state_addr)?;
                if state != TABLE_STATE_FULL {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if dest_ptr != 0 {
                    let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                    if out_data != 0 && repr.elem_size != 0 && entry_ptr != 0 {
                        let bytes = self.read_bytes(entry_ptr, repr.elem_size)?;
                        self.store_bytes(out_data, 0, &bytes)?;
                    }
                }
                self.hashset_drop_value(&repr, entry_ptr)?;
                self.write_u8(state_addr, TABLE_STATE_TOMBSTONE)?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashset_bucket_state") => {
                let [Value::I32(set_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_bucket_state expects (i32 set, i32 index)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_bucket_state set")?;
                let index =
                    value_as_u32(&Value::I32(*index), "chic_rt.hashset_bucket_state index")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.states == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let state = self.read_u8(repr.states.checked_add(index).unwrap_or(0))?;
                Ok(Some(Value::I32(i32::from(state))))
            }
            ("chic_rt", "hashset_bucket_hash") => {
                let [Value::I32(set_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_bucket_hash expects (i32 set, i32 index)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_bucket_hash set")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashset_bucket_hash index")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.hashes == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let slot = self.hashset_hash_slot(repr.hashes, index)?;
                let value = if slot == 0 { 0 } else { self.read_u64(slot)? };
                Ok(Some(Value::I64(value as i64)))
            }
            ("chic_rt", "hashset_iter") => {
                let [Value::I32(out_ptr), Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_iter expects (i32 out, i32 set)".into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_iter out")?;
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_iter set")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if set_ptr == 0 {
                    self.write_hashset_iter_repr(
                        out_ptr,
                        WasmHashSetIterRepr {
                            entries: 0,
                            states: 0,
                            index: 0,
                            cap: 0,
                            elem_size: 0,
                            elem_align: 1,
                        },
                    )?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                self.write_hashset_iter_repr(
                    out_ptr,
                    WasmHashSetIterRepr {
                        entries: repr.entries,
                        states: repr.states,
                        index: 0,
                        cap: repr.cap,
                        elem_size: repr.elem_size,
                        elem_align: repr.elem_align.max(1),
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashset_iter_next") => {
                let [Value::I32(iter_ptr), Value::I32(dest_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_iter_next expects (i32 iter, i32 dest)".into(),
                    });
                };
                let iter_ptr =
                    value_as_ptr_u32(&Value::I32(*iter_ptr), "chic_rt.hashset_iter_next iter")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_iter_next dest")?;
                if iter_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut iter = self.read_hashset_iter_repr(iter_ptr)?;
                if iter.cap == 0 || iter.states == 0 {
                    return Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)));
                }
                while iter.index < iter.cap {
                    let idx = iter.index;
                    iter.index = idx.saturating_add(1);
                    let state = self.read_u8(iter.states.checked_add(idx).unwrap_or(0))?;
                    if state == TABLE_STATE_FULL {
                        let entry_ptr =
                            self.hashset_entry_ptr(iter.entries, iter.elem_size, idx)?;
                        let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                        if out_data != 0 && iter.elem_size != 0 && entry_ptr != 0 {
                            let bytes = self.read_bytes(entry_ptr, iter.elem_size)?;
                            self.store_bytes(out_data, 0, &bytes)?;
                        }
                        self.write_hashset_iter_repr(iter_ptr, iter)?;
                        return Ok(Some(Value::I32(TABLE_SUCCESS)));
                    }
                }
                self.write_hashset_iter_repr(iter_ptr, iter)?;
                Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)))
            }
            ("chic_rt", "hashset_iter_next_ptr") => {
                let [Value::I32(out_ptr), Value::I32(iter_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_iter_next_ptr expects (i32 out, i32 iter)".into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_iter_next_ptr out")?;
                let iter_ptr =
                    value_as_ptr_u32(&Value::I32(*iter_ptr), "chic_rt.hashset_iter_next_ptr iter")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if iter_ptr == 0 {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let mut iter = self.read_hashset_iter_repr(iter_ptr)?;
                if iter.cap == 0 || iter.states == 0 {
                    self.write_value_ptr(out_ptr, 0, iter.elem_size, iter.elem_align.max(1))?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                while iter.index < iter.cap {
                    let idx = iter.index;
                    iter.index = idx.saturating_add(1);
                    let state = self.read_u8(iter.states.checked_add(idx).unwrap_or(0))?;
                    if state == TABLE_STATE_FULL {
                        let entry_ptr =
                            self.hashset_entry_ptr(iter.entries, iter.elem_size, idx)?;
                        self.write_hashset_iter_repr(iter_ptr, iter)?;
                        self.write_value_ptr(
                            out_ptr,
                            entry_ptr,
                            iter.elem_size,
                            iter.elem_align.max(1),
                        )?;
                        return Ok(Some(Value::I32(out_ptr as i32)));
                    }
                }
                self.write_hashset_iter_repr(iter_ptr, iter)?;
                self.write_value_ptr(out_ptr, 0, iter.elem_size, iter.elem_align.max(1))?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashmap_new") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(key_size),
                    Value::I32(key_align),
                    Value::I32(value_size),
                    Value::I32(value_align),
                    Value::I32(key_drop_fn),
                    Value::I32(value_drop_fn),
                    Value::I32(key_eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_new expects (i32 out, i32 key_size, i32 key_align, i32 value_size, i32 value_align, i32 key_drop_fn, i32 value_drop_fn, i32 key_eq_fn)"
                                .into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_new out")?;
                let key_size =
                    value_as_u32(&Value::I32(*key_size), "chic_rt.hashmap_new key_size")?;
                let key_align =
                    value_as_u32(&Value::I32(*key_align), "chic_rt.hashmap_new key_align")?.max(1);
                let value_size =
                    value_as_u32(&Value::I32(*value_size), "chic_rt.hashmap_new value_size")?;
                let value_align =
                    value_as_u32(&Value::I32(*value_align), "chic_rt.hashmap_new value_align")?
                        .max(1);
                let key_drop_fn =
                    value_as_u32(&Value::I32(*key_drop_fn), "chic_rt.hashmap_new key_drop_fn")?;
                let value_drop_fn = value_as_u32(
                    &Value::I32(*value_drop_fn),
                    "chic_rt.hashmap_new value_drop_fn",
                )?;
                let key_eq_fn =
                    value_as_u32(&Value::I32(*key_eq_fn), "chic_rt.hashmap_new key_eq_fn")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let offset = self.align_up(key_size, value_align).unwrap_or(key_size);
                self.write_hashmap_repr(
                    out_ptr,
                    WasmHashMapRepr {
                        entries: 0,
                        states: 0,
                        hashes: 0,
                        len: 0,
                        cap: 0,
                        tombstones: 0,
                        key_size,
                        key_align,
                        value_size,
                        value_align,
                        entry_size: offset.saturating_add(value_size),
                        value_offset: offset,
                        key_drop_fn,
                        value_drop_fn,
                        key_eq_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashmap_with_capacity") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(key_size),
                    Value::I32(key_align),
                    Value::I32(value_size),
                    Value::I32(value_align),
                    Value::I32(capacity),
                    Value::I32(key_drop_fn),
                    Value::I32(value_drop_fn),
                    Value::I32(key_eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_with_capacity expects (i32 out, i32 key_size, i32 key_align, i32 value_size, i32 value_align, i32 cap, i32 key_drop_fn, i32 value_drop_fn, i32 key_eq_fn)"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_with_capacity out")?;
                let key_size = value_as_u32(
                    &Value::I32(*key_size),
                    "chic_rt.hashmap_with_capacity key_size",
                )?;
                let key_align = value_as_u32(
                    &Value::I32(*key_align),
                    "chic_rt.hashmap_with_capacity key_align",
                )?
                .max(1);
                let value_size = value_as_u32(
                    &Value::I32(*value_size),
                    "chic_rt.hashmap_with_capacity value_size",
                )?;
                let value_align = value_as_u32(
                    &Value::I32(*value_align),
                    "chic_rt.hashmap_with_capacity value_align",
                )?
                .max(1);
                let capacity =
                    value_as_u32(&Value::I32(*capacity), "chic_rt.hashmap_with_capacity cap")?;
                let key_drop_fn = value_as_u32(
                    &Value::I32(*key_drop_fn),
                    "chic_rt.hashmap_with_capacity key_drop_fn",
                )?;
                let value_drop_fn = value_as_u32(
                    &Value::I32(*value_drop_fn),
                    "chic_rt.hashmap_with_capacity value_drop_fn",
                )?;
                let key_eq_fn = value_as_u32(
                    &Value::I32(*key_eq_fn),
                    "chic_rt.hashmap_with_capacity key_eq_fn",
                )?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 16 {
                        eprintln!(
                            "[wasm-hashmap] with_capacity[{idx}] out=0x{out_ptr:08X} key_size={key_size} key_align={key_align} value_size={value_size} value_align={value_align} cap={capacity} key_drop=0x{key_drop_fn:08X} value_drop=0x{value_drop_fn:08X} key_eq=0x{key_eq_fn:08X}",
                        );
                    }
                }
                let offset = self.align_up(key_size, value_align).unwrap_or(key_size);
                let entry_size = offset.saturating_add(value_size);
                let mut repr = WasmHashMapRepr {
                    entries: 0,
                    states: 0,
                    hashes: 0,
                    len: 0,
                    cap: 0,
                    tombstones: 0,
                    key_size,
                    key_align,
                    value_size,
                    value_align,
                    entry_size,
                    value_offset: offset,
                    key_drop_fn,
                    value_drop_fn,
                    key_eq_fn,
                };
                let normalized = if capacity == 0 {
                    0
                } else {
                    self.table_round_up_pow2(capacity).unwrap_or(0)
                };
                if normalized != 0 {
                    let max_align = key_align.max(value_align).max(1);
                    let entry_bytes = normalized.saturating_mul(entry_size);
                    let hash_bytes = normalized.saturating_mul(8);
                    repr.entries = self.allocate_heap_block(entry_bytes, max_align)?;
                    repr.states = self.allocate_heap_block(normalized, 1)?;
                    repr.hashes = self.allocate_heap_block(hash_bytes, 8)?;
                    repr.cap = normalized;
                }
                self.write_hashmap_repr(out_ptr, repr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER
                        .load(Ordering::Relaxed)
                        .saturating_sub(1);
                    if idx < 16 {
                        let written = self.read_hashmap_repr(out_ptr).unwrap_or_default();
                        eprintln!(
                            "[wasm-hashmap] with_capacity[{idx}] wrote out=0x{out_ptr:08X} cap={} len={} key_size={} value_size={} entry_size={} entries=0x{:08X} states=0x{:08X} hashes=0x{:08X}",
                            written.cap,
                            written.len,
                            written.key_size,
                            written.value_size,
                            written.entry_size,
                            written.entries,
                            written.states,
                            written.hashes
                        );
                    }
                }
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashmap_drop") => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_drop expects (i32 map)".into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_drop map")?;
                if map_ptr == 0 {
                    return Ok(None);
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap != 0 && repr.states != 0 && repr.entries != 0 && repr.entry_size != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashmap_entry_ptr(repr.entries, repr.entry_size, idx)?;
                            self.hashmap_drop_entry(&repr, entry_ptr)?;
                        }
                    }
                }
                repr.entries = 0;
                repr.states = 0;
                repr.hashes = 0;
                repr.len = 0;
                repr.cap = 0;
                repr.tombstones = 0;
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(None)
            }
            ("chic_rt", "hashmap_clear") => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_clear expects (i32 map)".into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_clear map")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 {
                    repr.len = 0;
                    repr.tombstones = 0;
                    self.write_hashmap_repr(map_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.states != 0 && repr.entries != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashmap_entry_ptr(repr.entries, repr.entry_size, idx)?;
                            self.hashmap_drop_entry(&repr, entry_ptr)?;
                        }
                    }
                    self.fill(repr.states, 0, repr.cap, TABLE_STATE_EMPTY)?;
                    if repr.hashes != 0 {
                        let hash_bytes = repr.cap.saturating_mul(8);
                        self.fill(repr.hashes, 0, hash_bytes, 0)?;
                    }
                }
                repr.len = 0;
                repr.tombstones = 0;
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashmap_len") => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_len expects (i32 map)".into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_len map")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                Ok(Some(Value::I32(repr.len as i32)))
            }
            ("chic_rt", "hashmap_capacity") => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_capacity expects (i32 map)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_capacity map")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                Ok(Some(Value::I32(repr.cap as i32)))
            }
            ("chic_rt", "hashmap_reserve") => {
                let [Value::I32(map_ptr), Value::I32(additional)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_reserve expects (i32 map, i32 additional)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_reserve map")?;
                let additional = value_as_u32(
                    &Value::I32(*additional),
                    "chic_rt.hashmap_reserve additional",
                )?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                if !self.table_should_grow(repr.len, repr.tombstones, repr.cap, additional) {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let needed = match repr.len.checked_add(additional) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let doubled = match needed.checked_add(needed) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let desired = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let Some(target) = self.table_round_up_pow2(desired).filter(|v| *v != 0) else {
                    return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                };
                let rebuilt = self.hashmap_rehash(&repr, target)?;
                self.write_hashmap_repr(map_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashmap_shrink_to") => {
                let [Value::I32(map_ptr), Value::I32(min_capacity)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_shrink_to expects (i32 map, i32 min_capacity)"
                            .into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_shrink_to map")?;
                let min_capacity = value_as_u32(
                    &Value::I32(*min_capacity),
                    "chic_rt.hashmap_shrink_to min_capacity",
                )?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                let min_cap = if min_capacity == 0 {
                    0
                } else {
                    match self.table_round_up_pow2(min_capacity) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    }
                };
                let mut desired = min_cap;
                if repr.len != 0 {
                    let doubled = match repr.len.checked_add(repr.len) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let expanded = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    desired = match self.table_round_up_pow2(expanded) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                }
                let target = desired.max(min_cap);
                if target >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if target == 0 {
                    repr.entries = 0;
                    repr.states = 0;
                    repr.hashes = 0;
                    repr.len = 0;
                    repr.cap = 0;
                    repr.tombstones = 0;
                    self.write_hashmap_repr(map_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let rebuilt = self.hashmap_rehash(&repr, target)?;
                self.write_hashmap_repr(map_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashmap_insert") => {
                let [
                    Value::I32(map_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                    Value::I32(value_ptr),
                    Value::I32(previous_value_ptr),
                    Value::I32(replaced_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_insert expects (i32 map, i64 hash, i32 key, i32 value, i32 prev, i32 replaced)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_insert map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_insert key")?;
                let value_ptr =
                    value_as_ptr_u32(&Value::I32(*value_ptr), "chic_rt.hashmap_insert value")?;
                let previous_value_ptr = value_as_ptr_u32(
                    &Value::I32(*previous_value_ptr),
                    "chic_rt.hashmap_insert prev",
                )?;
                let replaced_ptr = value_as_ptr_u32(
                    &Value::I32(*replaced_ptr),
                    "chic_rt.hashmap_insert replaced",
                )?;
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                if map_ptr == 0 || key_ptr == 0 || value_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 16 {
                        let key_words = (
                            self.read_u32(key_ptr).unwrap_or(0),
                            self.read_u32(key_ptr + 4).unwrap_or(0),
                            self.read_u32(key_ptr + 8).unwrap_or(0),
                        );
                        let value_words = (
                            self.read_u32(value_ptr).unwrap_or(0),
                            self.read_u32(value_ptr + 4).unwrap_or(0),
                            self.read_u32(value_ptr + 8).unwrap_or(0),
                        );
                        eprintln!(
                            "[wasm-hashmap] insert[{idx}] map=0x{map_ptr:08X} hash=0x{hash:016X} key_handle=0x{key_ptr:08X} key={{ptr=0x{:08X} size={} align={}}} value_handle=0x{value_ptr:08X} value={{ptr=0x{:08X} size={} align={}}} prev=0x{previous_value_ptr:08X} replaced=0x{replaced_ptr:08X}",
                            key_words.0,
                            key_words.1,
                            key_words.2,
                            value_words.0,
                            value_words.1,
                            value_words.2
                        );
                    }
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (value_data, _, _) = self.read_value_ptr(value_ptr)?;
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER
                        .load(Ordering::Relaxed)
                        .saturating_sub(1);
                    if idx < 16 {
                        let key_preview = self.read_bytes(key_data, 32).unwrap_or_default();
                        let value_preview = self.read_bytes(value_data, 32).unwrap_or_default();
                        eprintln!(
                            "[wasm-hashmap] insert[{idx}] repr cap={} len={} tomb={} key_size={} key_align={} value_size={} value_align={} entry_size={} value_off={} entries=0x{:08X} states=0x{:08X} hashes=0x{:08X} key_eq=0x{:08X} key_data=0x{key_data:08X} key_bytes={:02X?} value_data=0x{value_data:08X} value_bytes={:02X?}",
                            repr.cap,
                            repr.len,
                            repr.tombstones,
                            repr.key_size,
                            repr.key_align,
                            repr.value_size,
                            repr.value_align,
                            repr.entry_size,
                            repr.value_offset,
                            repr.entries,
                            repr.states,
                            repr.hashes,
                            repr.key_eq_fn,
                            key_preview,
                            value_preview
                        );
                    }
                }
                if repr.cap == 0 || self.table_should_grow(repr.len, repr.tombstones, repr.cap, 1) {
                    let needed = match repr.len.checked_add(1) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let doubled = match needed.checked_add(needed) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let desired = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let Some(target) = self.table_round_up_pow2(desired).filter(|v| *v != 0) else {
                        return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                    };
                    let rebuilt = self.hashmap_rehash(&repr, target)?;
                    self.write_hashmap_repr(map_ptr, rebuilt)?;
                    repr = self.read_hashmap_repr(map_ptr)?;
                }
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                if found {
                    if previous_value_ptr != 0 {
                        let (prev_data, prev_size, _) = self.read_value_ptr(previous_value_ptr)?;
                        if prev_data != 0 && prev_size != 0 && repr.value_size != 0 {
                            let value_src = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                            let bytes = self.read_bytes(value_src, repr.value_size)?;
                            self.store_bytes(prev_data, 0, &bytes)?;
                        } else if prev_size == 0 && repr.value_drop_fn != 0 {
                            let value_src = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                            let _ =
                                self.invoke(repr.value_drop_fn, &[Value::I32(value_src as i32)])?;
                        }
                    } else if repr.value_drop_fn != 0 {
                        let value_src = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                        let _ = self.invoke(repr.value_drop_fn, &[Value::I32(value_src as i32)])?;
                    }
                    if repr.key_size != 0 && key_data != 0 {
                        let key_bytes = self.read_bytes(key_data, repr.key_size)?;
                        self.store_bytes(entry_ptr, 0, &key_bytes)?;
                    }
                    if repr.value_size != 0 && value_data != 0 {
                        let value_dst = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                        let value_bytes = self.read_bytes(value_data, repr.value_size)?;
                        self.store_bytes(value_dst, 0, &value_bytes)?;
                    }
                    if replaced_ptr != 0 {
                        let _ = self.write_u32(replaced_ptr, 1);
                    }
                    self.write_hashmap_repr(map_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.key_size != 0 && key_data != 0 {
                    let key_bytes = self.read_bytes(key_data, repr.key_size)?;
                    self.store_bytes(entry_ptr, 0, &key_bytes)?;
                }
                if repr.value_size != 0 && value_data != 0 {
                    let value_dst = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                    let value_bytes = self.read_bytes(value_data, repr.value_size)?;
                    self.store_bytes(value_dst, 0, &value_bytes)?;
                }
                let hash_slot = self.hashmap_hash_slot(repr.hashes, index)?;
                if hash_slot != 0 {
                    self.write_u64(hash_slot, *hash as u64)?;
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let prior = self.read_u8(state_addr)?;
                if prior == TABLE_STATE_TOMBSTONE && repr.tombstones != 0 {
                    repr.tombstones -= 1;
                }
                self.write_u8(state_addr, TABLE_STATE_FULL)?;
                repr.len = repr.len.saturating_add(1);
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashmap_contains") => {
                let [Value::I32(map_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_contains expects (i32 map, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_contains map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_contains key")?;
                if map_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let repr = self.read_hashmap_repr(map_ptr)?;
                let (found, _) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                Ok(Some(Value::I32(if found { 1 } else { 0 })))
            }
            ("chic_rt", "hashmap_get_ptr") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(map_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_get_ptr expects (i32 out, i32 map, i64 hash, i32 key)"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_get_ptr out")?;
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_get_ptr map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_get_ptr key")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if map_ptr == 0 || key_ptr == 0 {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                self.write_value_ptr(out_ptr, value_ptr, repr.value_size, repr.value_align.max(1))?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashmap_take") => {
                let [
                    Value::I32(map_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                    Value::I32(dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_take expects (i32 map, i64 hash, i32 key, i32 dest)"
                                .into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_take map")?;
                let key_ptr = value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_take key")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashmap_take dest")?;
                if map_ptr == 0 || key_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (out_data, out_size, _) = self.read_value_ptr(dest_ptr)?;
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                if out_size != 0 && out_data != 0 && repr.value_size != 0 {
                    let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                    let bytes = self.read_bytes(value_ptr, repr.value_size)?;
                    self.store_bytes(out_data, 0, &bytes)?;
                }
                self.hashmap_drop_entry(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashmap_remove") => {
                let [Value::I32(map_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_remove expects (i32 map, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_remove map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_remove key")?;
                if map_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(0)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                self.hashmap_drop_entry(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(1)))
            }
            ("chic_rt", "hashmap_bucket_state") => {
                let [Value::I32(map_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_bucket_state expects (i32 map, i32 index)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_bucket_state map")?;
                let index =
                    value_as_u32(&Value::I32(*index), "chic_rt.hashmap_bucket_state index")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.states == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let state = self.read_u8(repr.states.checked_add(index).unwrap_or(0))?;
                Ok(Some(Value::I32(i32::from(state))))
            }
            ("chic_rt", "hashmap_bucket_hash") => {
                let [Value::I32(map_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_bucket_hash expects (i32 map, i32 index)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_bucket_hash map")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashmap_bucket_hash index")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.hashes == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let slot = self.hashmap_hash_slot(repr.hashes, index)?;
                let value = if slot == 0 { 0 } else { self.read_u64(slot)? };
                Ok(Some(Value::I64(value as i64)))
            }
            ("chic_rt", "hashmap_take_at") => {
                let [
                    Value::I32(map_ptr),
                    Value::I32(index),
                    Value::I32(key_dest_ptr),
                    Value::I32(value_dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_take_at expects (i32 map, i32 index, i32 key_dest, i32 value_dest)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_take_at map")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashmap_take_at index")?;
                let key_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*key_dest_ptr),
                    "chic_rt.hashmap_take_at key_dest",
                )?;
                let value_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*value_dest_ptr),
                    "chic_rt.hashmap_take_at value_dest",
                )?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 || index >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let state = self.read_u8(state_addr)?;
                if state != TABLE_STATE_FULL {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                if key_dest_ptr != 0 {
                    let (out_data, _, _) = self.read_value_ptr(key_dest_ptr)?;
                    if out_data != 0 && repr.key_size != 0 {
                        let bytes = self.read_bytes(entry_ptr, repr.key_size)?;
                        self.store_bytes(out_data, 0, &bytes)?;
                    }
                }
                if value_dest_ptr != 0 {
                    let (out_data, _, _) = self.read_value_ptr(value_dest_ptr)?;
                    if out_data != 0 && repr.value_size != 0 {
                        let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                        let bytes = self.read_bytes(value_ptr, repr.value_size)?;
                        self.store_bytes(out_data, 0, &bytes)?;
                    }
                }
                self.hashmap_drop_entry(&repr, entry_ptr)?;
                self.write_u8(state_addr, TABLE_STATE_TOMBSTONE)?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            ("chic_rt", "hashmap_iter") => {
                let [Value::I32(out_ptr), Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_iter expects (i32 out, i32 map)".into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_iter out")?;
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_iter map")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if map_ptr == 0 {
                    self.write_hashmap_iter_repr(out_ptr, WasmHashMapIterRepr::default())?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                self.write_hashmap_iter_repr(
                    out_ptr,
                    WasmHashMapIterRepr {
                        entries: repr.entries,
                        states: repr.states,
                        index: 0,
                        cap: repr.cap,
                        entry_size: repr.entry_size,
                        key_size: repr.key_size,
                        key_align: repr.key_align,
                        value_size: repr.value_size,
                        value_align: repr.value_align,
                        value_offset: repr.value_offset,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "hashmap_iter_next") => {
                let [
                    Value::I32(iter_ptr),
                    Value::I32(key_dest_ptr),
                    Value::I32(value_dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_iter_next expects (i32 iter, i32 key_dest, i32 value_dest)".into(),
                    });
                };
                let iter_ptr =
                    value_as_ptr_u32(&Value::I32(*iter_ptr), "chic_rt.hashmap_iter_next iter")?;
                let key_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*key_dest_ptr),
                    "chic_rt.hashmap_iter_next key_dest",
                )?;
                let value_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*value_dest_ptr),
                    "chic_rt.hashmap_iter_next value_dest",
                )?;
                if iter_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut iter = self.read_hashmap_iter_repr(iter_ptr)?;
                if iter.cap == 0 || iter.states == 0 {
                    return Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)));
                }
                while iter.index < iter.cap {
                    let idx = iter.index;
                    iter.index = idx.saturating_add(1);
                    let state = self.read_u8(iter.states.checked_add(idx).unwrap_or(0))?;
                    if state == TABLE_STATE_FULL {
                        let entry_ptr =
                            self.hashmap_entry_ptr(iter.entries, iter.entry_size, idx)?;
                        if key_dest_ptr != 0 {
                            let (out_data, _, _) = self.read_value_ptr(key_dest_ptr)?;
                            if out_data != 0 && iter.key_size != 0 {
                                let bytes = self.read_bytes(entry_ptr, iter.key_size)?;
                                self.store_bytes(out_data, 0, &bytes)?;
                            }
                        }
                        if value_dest_ptr != 0 {
                            let (out_data, _, _) = self.read_value_ptr(value_dest_ptr)?;
                            if out_data != 0 && iter.value_size != 0 {
                                let value_ptr =
                                    self.hashmap_value_ptr(entry_ptr, iter.value_offset)?;
                                let bytes = self.read_bytes(value_ptr, iter.value_size)?;
                                self.store_bytes(out_data, 0, &bytes)?;
                            }
                        }
                        self.write_hashmap_iter_repr(iter_ptr, iter)?;
                        return Ok(Some(Value::I32(TABLE_SUCCESS)));
                    }
                }
                self.write_hashmap_iter_repr(iter_ptr, iter)?;
                Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)))
            }
            ("chic_rt", "mmio_read") => {
                let [Value::I64(address), Value::I32(width), Value::I32(flags)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.mmio_read expects (i64, i32, i32) arguments".into(),
                    });
                };
                let value = self.read_mmio(*address as u64, *width as u32, *flags)?;
                Ok(Some(Value::I64(value as i64)))
            }
            ("chic_rt", "mmio_write") => {
                let [
                    Value::I64(address),
                    Value::I64(value),
                    Value::I32(width),
                    Value::I32(flags),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.mmio_write expects (i64, i64, i32, i32) arguments".into(),
                    });
                };
                self.write_mmio(*address as u64, *value as u64, *width as u32, *flags)?;
                Ok(None)
            }
            ("chic_rt", name) if name.starts_with("i128_") || name.starts_with("u128_") => {
                self.invoke_int128_import(name, params.as_slice())
            }
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import {}::{} encountered during execution",
                    import.module, import.name
                ),
            }),
        })()?;

        Ok(result.into_iter().collect())
    }

    fn invoke_int128_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "i128_add" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_add expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_add out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_add lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_add rhs")?;
                let result = self.read_i128(lhs)?.wrapping_add(self.read_i128(rhs)?);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_sub" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_sub expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_sub out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_sub lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_sub rhs")?;
                let result = self.read_i128(lhs)?.wrapping_sub(self.read_i128(rhs)?);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_mul" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_mul expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_mul out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_mul lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_mul rhs")?;
                let result = self.read_i128(lhs)?.wrapping_mul(self.read_i128(rhs)?);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_div" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_div expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_div out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_div lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_div rhs")?;
                let rhs_value = self.read_i128(rhs)?;
                let Some(result) = self.read_i128(lhs)?.checked_div(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_div division error".into(),
                    });
                };
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_rem" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_rem expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_rem out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_rem lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_rem rhs")?;
                let rhs_value = self.read_i128(rhs)?;
                let Some(result) = self.read_i128(lhs)?.checked_rem(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_rem division error".into(),
                    });
                };
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_eq" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_eq expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_eq lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_eq rhs")?;
                let result = i32::from(self.read_i128(lhs)? == self.read_i128(rhs)?);
                Ok(Some(Value::I32(result)))
            }
            "i128_cmp" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_cmp expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_cmp lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_cmp rhs")?;
                let lhs_value = self.read_i128(lhs)?;
                let rhs_value = self.read_i128(rhs)?;
                let result = match lhs_value.cmp(&rhs_value) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                Ok(Some(Value::I32(result)))
            }
            "i128_neg" => {
                let [Value::I32(out_ptr), Value::I32(value_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_neg expects (i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_neg out")?;
                let value = self.checked_ptr(*value_ptr, "chic_rt.i128_neg value")?;
                let result = self.read_i128(value)?.wrapping_neg();
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_not" => {
                let [Value::I32(out_ptr), Value::I32(value_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_not expects (i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_not out")?;
                let value = self.checked_ptr(*value_ptr, "chic_rt.i128_not value")?;
                let result = !self.read_i128(value)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_and" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_and expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_and out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_and lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_and rhs")?;
                let result = self.read_i128(lhs)? & self.read_i128(rhs)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_or" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_or expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_or out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_or lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_or rhs")?;
                let result = self.read_i128(lhs)? | self.read_i128(rhs)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_xor" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_xor expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_xor out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_xor lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.i128_xor rhs")?;
                let result = self.read_i128(lhs)? ^ self.read_i128(rhs)?;
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_shl" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_shl expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_shl out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_shl lhs")?;
                let shift = *amount as u32;
                let result = self.read_i128(lhs)?.wrapping_shl(shift);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "i128_shr" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.i128_shr expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.i128_shr out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.i128_shr lhs")?;
                let shift = *amount as u32;
                let result = self.read_i128(lhs)?.wrapping_shr(shift);
                self.write_i128(out, result)?;
                Ok(None)
            }
            "u128_add" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_add expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_add out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_add lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_add rhs")?;
                let result = self.read_u128(lhs)?.wrapping_add(self.read_u128(rhs)?);
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_sub" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_sub expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_sub out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_sub lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_sub rhs")?;
                let result = self.read_u128(lhs)?.wrapping_sub(self.read_u128(rhs)?);
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_mul" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_mul expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_mul out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_mul lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_mul rhs")?;
                let result = self.read_u128(lhs)?.wrapping_mul(self.read_u128(rhs)?);
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_div" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_div expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_div out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_div lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_div rhs")?;
                let rhs_value = self.read_u128(rhs)?;
                let Some(result) = self.read_u128(lhs)?.checked_div(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_div division error".into(),
                    });
                };
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_rem" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_rem expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_rem out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_rem lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_rem rhs")?;
                let rhs_value = self.read_u128(rhs)?;
                let Some(result) = self.read_u128(lhs)?.checked_rem(rhs_value) else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_rem division error".into(),
                    });
                };
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_eq" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_eq expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_eq lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_eq rhs")?;
                let lhs_value = self.read_u128(lhs)?;
                let rhs_value = self.read_u128(rhs)?;
                let result = i32::from(lhs_value == rhs_value);
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] eq lhs=0x{lhs_value:032x} rhs=0x{rhs_value:032x} -> {result} caller={}",
                        self.current_wasm_context()
                    );
                }
                Ok(Some(Value::I32(result)))
            }
            "u128_cmp" => {
                let [Value::I32(lhs_ptr), Value::I32(rhs_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_cmp expects (i32, i32) arguments".into(),
                    });
                };
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_cmp lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_cmp rhs")?;
                let lhs_value = self.read_u128(lhs)?;
                let rhs_value = self.read_u128(rhs)?;
                let result = match lhs_value.cmp(&rhs_value) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                Ok(Some(Value::I32(result)))
            }
            "u128_not" => {
                let [Value::I32(out_ptr), Value::I32(value_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_not expects (i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_not out")?;
                let value = self.checked_ptr(*value_ptr, "chic_rt.u128_not value")?;
                let result = !self.read_u128(value)?;
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_and" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_and expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_and out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_and lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_and rhs")?;
                let lhs_value = self.read_u128(lhs)?;
                let rhs_value = self.read_u128(rhs)?;
                let result = lhs_value & rhs_value;
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] and lhs=0x{lhs_value:032x} rhs=0x{rhs_value:032x} -> 0x{result:032x} caller={}",
                        self.current_wasm_context()
                    );
                }
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_or" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_or expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_or out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_or lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_or rhs")?;
                let result = self.read_u128(lhs)? | self.read_u128(rhs)?;
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_xor" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(lhs_ptr),
                    Value::I32(rhs_ptr),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_xor expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_xor out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_xor lhs")?;
                let rhs = self.checked_ptr(*rhs_ptr, "chic_rt.u128_xor rhs")?;
                let result = self.read_u128(lhs)? ^ self.read_u128(rhs)?;
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_shl" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_shl expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_shl out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_shl lhs")?;
                let shift = *amount as u32;
                let lhs_value = self.read_u128(lhs)?;
                let result = lhs_value.wrapping_shl(shift);
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] shl lhs=0x{lhs_value:032x} shift={shift} -> 0x{result:032x} caller={}",
                        self.current_wasm_context()
                    );
                }
                self.write_u128(out, result)?;
                Ok(None)
            }
            "u128_shr" => {
                let [Value::I32(out_ptr), Value::I32(lhs_ptr), Value::I32(amount)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.u128_shr expects (i32, i32, i32) arguments".into(),
                    });
                };
                let out = self.checked_ptr(*out_ptr, "chic_rt.u128_shr out")?;
                let lhs = self.checked_ptr(*lhs_ptr, "chic_rt.u128_shr lhs")?;
                let shift = *amount as u32;
                let lhs_value = self.read_u128(lhs)?;
                let result = lhs_value.wrapping_shr(shift);
                if std::env::var_os("CHIC_DEBUG_WASM_U128").is_some() {
                    eprintln!(
                        "[wasm-u128] shr lhs=0x{lhs_value:032x} shift={shift} -> 0x{result:032x} caller={}",
                        self.current_wasm_context()
                    );
                }
                self.write_u128(out, result)?;
                Ok(None)
            }
            _ => Err(WasmExecutionError {
                message: format!("unsupported chic_rt int128 import `{name}`"),
            }),
        }
    }

    fn checked_ptr(&self, value: i32, context: &str) -> Result<u32, WasmExecutionError> {
        u32::try_from(value).map_err(|_| WasmExecutionError {
            message: format!("{context} received negative pointer"),
        })
    }

    fn read_u128(&self, ptr: u32) -> Result<u128, WasmExecutionError> {
        let lo = self.read_u64(ptr)?;
        let hi = self.read_u64(ptr + 8)?;
        Ok((u128::from(hi) << 64) | u128::from(lo))
    }

    fn write_u128(&mut self, ptr: u32, value: u128) -> Result<(), WasmExecutionError> {
        let lo = value as u64;
        let hi = (value >> 64) as u64;
        self.write_u64(ptr, lo)?;
        self.write_u64(ptr + 8, hi)?;
        Ok(())
    }

    fn read_i128(&self, ptr: u32) -> Result<i128, WasmExecutionError> {
        let lo = self.read_u64(ptr)?;
        let hi = self.read_u64(ptr + 8)?;
        let hi = i64::from_le_bytes(hi.to_le_bytes());
        Ok(((hi as i128) << 64) | lo as i128)
    }

    fn write_i128(&mut self, ptr: u32, value: i128) -> Result<(), WasmExecutionError> {
        let lo = value as u128 as u64;
        let hi = (value >> 64) as i64;
        self.write_u64(ptr, lo)?;
        self.write_u64(ptr + 8, hi as u64)?;
        Ok(())
    }

    fn read_str_ptr(&self, ptr: u32) -> Result<(u32, u32), WasmExecutionError> {
        let stride = self.ptr_stride();
        let data_ptr = self.read_u32(ptr)?;
        let len = self.read_u32(ptr + stride)?;
        Ok((data_ptr, len))
    }

    fn write_str_ptr(
        &mut self,
        ptr: u32,
        data_ptr: u32,
        len: u32,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.ptr_stride();
        self.write_u32(ptr, data_ptr)?;
        self.write_u32(ptr + stride, len)
    }

    fn read_value_ptr(&self, ptr: u32) -> Result<(u32, u32, u32), WasmExecutionError> {
        let stride = self.ptr_stride();
        let data_ptr = self.read_u32(ptr)?;
        let size = self.read_u32(ptr + stride)?;
        let align = self.read_u32(ptr + stride * 2)?;
        Ok((data_ptr, size, align))
    }

    fn write_value_ptr(
        &mut self,
        ptr: u32,
        data_ptr: u32,
        size: u32,
        align: u32,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.ptr_stride();
        self.write_u32(ptr, data_ptr)?;
        self.write_u32(ptr + stride, size)?;
        self.write_u32(ptr + stride * 2, align)
    }

    fn read_span_ptr(&self, ptr: u32) -> Result<(u32, u32, u32, u32), WasmExecutionError> {
        let stride = self.ptr_stride();
        let (data_ptr, elem_size, elem_align) = self.read_value_ptr(ptr)?;
        // WASM SpanPtr ABI matches `Std.Runtime.Collections.{SpanPtr,ReadOnlySpanPtr}`:
        //   ValuePtr data { ptr, size, align } (3 * stride)
        //   usize length
        //   usize elementSize
        //   usize elementAlignment
        let length_offset = stride * 3;
        let len = self.read_u32(ptr + length_offset)?;
        Ok((data_ptr, len, elem_size, elem_align))
    }

    fn write_span_ptr(
        &mut self,
        ptr: u32,
        data_ptr: u32,
        len: u32,
        elem_size: u32,
        elem_align: u32,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.ptr_stride();
        let length_offset = stride * 3;
        self.write_value_ptr(ptr, data_ptr, elem_size, elem_align)?;
        self.write_u32(ptr + length_offset, len)?;
        self.write_u32(ptr + length_offset + stride, elem_size)?;
        self.write_u32(ptr + length_offset + stride * 2, elem_align)
    }

    fn ptr_stride(&self) -> u32 {
        self.async_layout.ptr_size
    }

    fn inline_string_ptr(&self, base: u32) -> Result<u32, WasmExecutionError> {
        base.checked_add(self.ptr_stride() * 3)
            .ok_or_else(|| WasmExecutionError {
                message: "string inline pointer overflow".into(),
            })
    }

    fn is_inline_string(&self, repr: &WasmStringRepr) -> bool {
        if repr.len == 0 || repr.len > STRING_INLINE_CAPACITY {
            return false;
        }
        if (repr.cap & STRING_INLINE_TAG) != 0 {
            return true;
        }
        repr.ptr == 0
    }

    fn init_empty_string(&mut self, dest_ptr: u32) -> Result<(), WasmExecutionError> {
        self.write_string_repr(dest_ptr, WasmStringRepr::default())
    }

    fn resolve_string_data_ptr(
        &self,
        base: u32,
        repr: &WasmStringRepr,
    ) -> Result<u32, WasmExecutionError> {
        if repr.len == 0 {
            return Ok(STRING_EMPTY_PTR);
        }
        if self.is_inline_string(repr) {
            return self.inline_string_ptr(base);
        }
        if repr.ptr == 0 {
            return Err(WasmExecutionError {
                message: format!(
                    "string at 0x{base:08X} has len {} cap=0x{:08X} but null pointer",
                    repr.len, repr.cap
                ),
            });
        }
        Ok(repr.ptr)
    }

    fn store_string_bytes(
        &mut self,
        dest_ptr: u32,
        data: &[u8],
    ) -> Result<i32, WasmExecutionError> {
        let len = u32::try_from(data.len()).map_err(|_| WasmExecutionError {
            message: "string data exceeds addressable range for wasm32".into(),
        })?;
        if len == 0 {
            self.init_empty_string(dest_ptr)?;
            return Ok(0);
        }
        if len <= STRING_INLINE_CAPACITY {
            let inline_ptr = self.inline_string_ptr(dest_ptr)?;
            self.store_bytes(inline_ptr, 0, data)?;
            self.write_string_repr(
                dest_ptr,
                WasmStringRepr {
                    ptr: 0,
                    len,
                    cap: STRING_INLINE_TAG | STRING_INLINE_CAPACITY,
                },
            )?;
            return Ok(0);
        }
        let ptr = self.allocate_heap_block(len, 1)?;
        self.store_bytes(ptr, 0, data)?;
        self.write_string_repr(dest_ptr, WasmStringRepr { ptr, len, cap: len })?;
        Ok(0)
    }

    fn append_string_bytes(
        &mut self,
        target_ptr: u32,
        data: &[u8],
    ) -> Result<i32, WasmExecutionError> {
        if target_ptr == 0 {
            return Ok(STRING_INVALID_POINTER);
        }
        if data.is_empty() {
            return Ok(STRING_SUCCESS);
        }
        let mut repr = self.read_string_repr(target_ptr)?;
        let inline = self.is_inline_string(&repr);
        if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
            eprintln!(
                "[wasm-string] append_bytes target=0x{target_ptr:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline} data_len={} mem_len={}",
                data.len(),
                self.memory_len(),
                ptr = repr.ptr,
                len = repr.len,
                cap = repr.cap
            );
        }
        if repr.len > 0 && repr.ptr == 0 && !inline {
            if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                eprintln!(
                    "[wasm-string] append_bytes invalid target=0x{target_ptr:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline}",
                    ptr = repr.ptr,
                    len = repr.len,
                    cap = repr.cap
                );
            }
            return Ok(STRING_INVALID_POINTER);
        }
        let data_len = u32::try_from(data.len()).map_err(|_| WasmExecutionError {
            message: "string append length exceeds wasm32 range".into(),
        })?;
        let new_len = match repr.len.checked_add(data_len) {
            Some(len) => len,
            None => return Ok(STRING_CAPACITY_OVERFLOW),
        };
        let capacity = if inline {
            STRING_INLINE_CAPACITY
        } else {
            repr.cap & !STRING_INLINE_TAG
        };

        if inline && new_len <= STRING_INLINE_CAPACITY {
            let inline_ptr = self.inline_string_ptr(target_ptr)?;
            self.store_bytes(inline_ptr, repr.len, data)?;
            repr.len = new_len;
            self.write_string_repr(target_ptr, repr)?;
            return Ok(STRING_SUCCESS);
        }

        let needs_alloc = new_len > capacity || repr.ptr == 0;
        if needs_alloc {
            let new_ptr = self.allocate_heap_block(new_len, 1)?;
            if repr.len > 0 {
                let existing_ptr = if inline {
                    self.inline_string_ptr(target_ptr)?
                } else {
                    repr.ptr
                };
                let existing = self.read_bytes(existing_ptr, repr.len)?;
                self.store_bytes(new_ptr, 0, &existing)?;
            }
            self.store_bytes(new_ptr, repr.len, data)?;
            repr.ptr = new_ptr;
            repr.cap = new_len;
        } else {
            self.store_bytes(repr.ptr, repr.len, data)?;
        }

        repr.len = new_len;
        self.write_string_repr(target_ptr, repr)?;
        Ok(STRING_SUCCESS)
    }

    fn drop_vec(&mut self, ptr: u32) -> Result<(), WasmExecutionError> {
        if ptr == 0 {
            return Ok(());
        }
        let repr = self.read_vec_repr(ptr)?;
        if repr.ptr == 0 || repr.len == 0 || repr.elem_size == 0 {
            self.write_vec_repr(ptr, WasmVecRepr::default())?;
            return Ok(());
        }

        if repr.drop_fn != 0 {
            let func_index = repr.drop_fn;
            let elem_size = repr.elem_size;
            for index in 0..repr.len {
                let offset = index
                    .checked_mul(elem_size)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_drop element offset overflow".into(),
                    })?;
                let elem_ptr = repr
                    .ptr
                    .checked_add(offset)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_drop element pointer overflow".into(),
                    })?;
                let _ = self.invoke(func_index, &[Value::I32(elem_ptr as i32)])?;
            }
        }

        self.write_vec_repr(ptr, WasmVecRepr::default())?;
        Ok(())
    }

    fn clone_vec(&mut self, dest_ptr: u32, src_ptr: u32) -> Result<i32, WasmExecutionError> {
        if dest_ptr == 0 || src_ptr == 0 {
            return Ok(1);
        }

        self.drop_vec(dest_ptr)?;

        let src = self.read_vec_repr(src_ptr)?;
        if src.ptr == 0 || src.len == 0 || src.elem_size == 0 {
            self.write_vec_repr(dest_ptr, WasmVecRepr::default())?;
            return Ok(0);
        }

        let bytes = src
            .len
            .checked_mul(src.elem_size)
            .ok_or_else(|| WasmExecutionError {
                message: "vec_clone length overflow".into(),
            })?;
        let base_align = src.elem_align.max(1);
        let data = self.read_bytes(src.ptr, bytes)?;
        let new_ptr = self.allocate_heap_block(bytes, base_align)?;
        self.store_bytes(new_ptr, 0, &data)?;
        self.write_vec_repr(
            dest_ptr,
            WasmVecRepr {
                ptr: new_ptr,
                len: src.len,
                cap: src.len,
                elem_size: src.elem_size,
                elem_align: base_align,
                drop_fn: src.drop_fn,
            },
        )?;
        Ok(0)
    }

    fn move_vec(&mut self, dest_ptr: u32, src_ptr: u32) -> Result<(), WasmExecutionError> {
        if dest_ptr == 0 || src_ptr == 0 {
            return Ok(());
        }
        self.drop_vec(dest_ptr)?;
        let src = self.read_vec_repr(src_ptr)?;
        self.write_vec_repr(
            dest_ptr,
            WasmVecRepr {
                cap: src.len,
                ..src
            },
        )?;
        self.write_vec_repr(src_ptr, WasmVecRepr::default())?;
        Ok(())
    }

    fn table_round_up_pow2(&self, value: u32) -> Option<u32> {
        if value == 0 {
            return Some(0);
        }
        if value <= TABLE_MIN_CAPACITY {
            return Some(TABLE_MIN_CAPACITY);
        }
        let mut cap = TABLE_MIN_CAPACITY;
        while cap < value {
            cap = cap.checked_add(cap)?;
        }
        Some(cap)
    }

    fn table_should_grow(&self, len: u32, tombstones: u32, cap: u32, additional: u32) -> bool {
        if cap == 0 {
            return true;
        }
        let filled = len as u64 + tombstones as u64;
        let needed = filled + additional as u64;
        needed.saturating_mul(TABLE_LOAD_DEN) > (cap as u64).saturating_mul(TABLE_LOAD_NUM)
    }

    fn align_up(&self, value: u32, align: u32) -> Option<u32> {
        if align <= 1 {
            return Some(value);
        }
        let mask = align - 1;
        value.checked_add(mask).map(|v| v & !mask)
    }

    fn hashset_entry_ptr(
        &self,
        entries: u32,
        elem_size: u32,
        index: u32,
    ) -> Result<u32, WasmExecutionError> {
        if entries == 0 {
            return Ok(0);
        }
        let offset = index
            .checked_mul(elem_size)
            .ok_or_else(|| WasmExecutionError {
                message: "hashset entry offset overflow".into(),
            })?;
        entries
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashset entry pointer overflow".into(),
            })
    }

    fn hashset_hash_slot(&self, hashes: u32, index: u32) -> Result<u32, WasmExecutionError> {
        if hashes == 0 {
            return Ok(0);
        }
        let offset = index.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashset hash offset overflow".into(),
        })?;
        hashes
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashset hash pointer overflow".into(),
            })
    }

    fn hashset_drop_value(
        &mut self,
        repr: &WasmHashSetRepr,
        entry_ptr: u32,
    ) -> Result<(), WasmExecutionError> {
        if repr.drop_fn == 0 || entry_ptr == 0 {
            return Ok(());
        }
        if std::env::var_os("CHIC_DEBUG_WASM_HASHSET").is_some() {
            let idx = HASHSET_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
            if idx < 80 {
                let total = self.module.imports.len() + self.module.functions.len();
                eprintln!(
                    "[wasm-hashset] drop[{idx}] entry_ptr=0x{entry_ptr:08x} drop_fn={} total_funcs={}",
                    repr.drop_fn, total
                );
            }
        }
        let _ = self.invoke(repr.drop_fn, &[Value::I32(entry_ptr as i32)])?;
        Ok(())
    }

    fn hashset_find_slot(
        &mut self,
        repr: &WasmHashSetRepr,
        hash: u64,
        key_ptr: u32,
    ) -> Result<(bool, u32), WasmExecutionError> {
        if repr.cap == 0 || repr.states == 0 {
            return Ok((false, 0));
        }
        let mask = repr.cap - 1;
        let start = (hash as u32) & mask;
        let mut first_tombstone = 0u32;
        let mut has_tombstone = false;
        let mut current = start;
        let mut probes = 0u32;
        while probes < repr.cap {
            let state = self.read_u8(repr.states.checked_add(current).unwrap_or(0))?;
            if state == TABLE_STATE_EMPTY {
                return Ok((
                    false,
                    if has_tombstone {
                        first_tombstone
                    } else {
                        current
                    },
                ));
            }
            if state == TABLE_STATE_TOMBSTONE {
                if !has_tombstone {
                    first_tombstone = current;
                    has_tombstone = true;
                }
            } else {
                let hash_slot = self.hashset_hash_slot(repr.hashes, current)?;
                let stored_hash = if hash_slot == 0 {
                    0
                } else {
                    self.read_u64(hash_slot)?
                };
                if stored_hash == hash && key_ptr != 0 && repr.entries != 0 && repr.eq_fn != 0 {
                    let entry_ptr =
                        self.hashset_entry_ptr(repr.entries, repr.elem_size, current)?;
                    let result = self.invoke(
                        repr.eq_fn,
                        &[Value::I32(entry_ptr as i32), Value::I32(key_ptr as i32)],
                    )?;
                    if matches!(result.first(), Some(Value::I32(v)) if *v != 0) {
                        return Ok((true, current));
                    }
                }
            }
            current = (current + 1) & mask;
            probes += 1;
        }
        Ok((false, 0))
    }

    fn hashset_rehash(
        &mut self,
        repr: &WasmHashSetRepr,
        new_cap: u32,
    ) -> Result<WasmHashSetRepr, WasmExecutionError> {
        if new_cap == 0 {
            return Ok(WasmHashSetRepr {
                entries: 0,
                states: 0,
                hashes: 0,
                cap: 0,
                tombstones: 0,
                len: 0,
                ..*repr
            });
        }
        let elem_align = repr.elem_align.max(1);
        let entry_bytes =
            new_cap
                .checked_mul(repr.elem_size)
                .ok_or_else(|| WasmExecutionError {
                    message: "hashset rehash entry buffer overflow".into(),
                })?;
        let hash_bytes = new_cap.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashset rehash hash buffer overflow".into(),
        })?;
        let entries = self.allocate_heap_block(entry_bytes, elem_align)?;
        let states = self.allocate_heap_block(new_cap, 1)?;
        let hashes = self.allocate_heap_block(hash_bytes, 8)?;

        let mut rebuilt = WasmHashSetRepr {
            entries,
            states,
            hashes,
            cap: new_cap,
            tombstones: 0,
            len: 0,
            ..*repr
        };

        if repr.cap != 0 && repr.states != 0 {
            let mask = new_cap - 1;
            for idx in 0..repr.cap {
                let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                if state != TABLE_STATE_FULL {
                    continue;
                }
                let old_hash_slot = self.hashset_hash_slot(repr.hashes, idx)?;
                let hash_value = if old_hash_slot == 0 {
                    0
                } else {
                    self.read_u64(old_hash_slot)?
                };
                let mut insert_index = (hash_value as u32) & mask;
                while self.read_u8(states.checked_add(insert_index).unwrap_or(0))?
                    == TABLE_STATE_FULL
                {
                    insert_index = (insert_index + 1) & mask;
                }
                let src_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, idx)?;
                let dest_ptr = self.hashset_entry_ptr(entries, repr.elem_size, insert_index)?;
                if repr.elem_size != 0 && src_ptr != 0 && dest_ptr != 0 {
                    let data = self.read_bytes(src_ptr, repr.elem_size)?;
                    self.store_bytes(dest_ptr, 0, &data)?;
                }
                let new_hash_slot = self.hashset_hash_slot(hashes, insert_index)?;
                if new_hash_slot != 0 {
                    self.write_u64(new_hash_slot, hash_value)?;
                }
                self.write_u8(
                    states.checked_add(insert_index).unwrap_or(0),
                    TABLE_STATE_FULL,
                )?;
                rebuilt.len = rebuilt.len.saturating_add(1);
            }
        }
        Ok(rebuilt)
    }

    fn hashmap_entry_ptr(
        &self,
        entries: u32,
        entry_size: u32,
        index: u32,
    ) -> Result<u32, WasmExecutionError> {
        if entries == 0 {
            return Ok(0);
        }
        let offset = index
            .checked_mul(entry_size)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap entry offset overflow".into(),
            })?;
        entries
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap entry pointer overflow".into(),
            })
    }

    fn hashmap_hash_slot(&self, hashes: u32, index: u32) -> Result<u32, WasmExecutionError> {
        if hashes == 0 {
            return Ok(0);
        }
        let offset = index.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashmap hash offset overflow".into(),
        })?;
        hashes
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap hash pointer overflow".into(),
            })
    }

    fn hashmap_value_ptr(
        &self,
        entry_ptr: u32,
        value_offset: u32,
    ) -> Result<u32, WasmExecutionError> {
        if entry_ptr == 0 {
            return Ok(0);
        }
        entry_ptr
            .checked_add(value_offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap value pointer overflow".into(),
            })
    }

    fn hashmap_drop_entry(
        &mut self,
        repr: &WasmHashMapRepr,
        entry_ptr: u32,
    ) -> Result<(), WasmExecutionError> {
        if entry_ptr == 0 {
            return Ok(());
        }
        if repr.key_drop_fn != 0 {
            let _ = self.invoke(repr.key_drop_fn, &[Value::I32(entry_ptr as i32)])?;
        }
        if repr.value_drop_fn != 0 {
            let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
            let _ = self.invoke(repr.value_drop_fn, &[Value::I32(value_ptr as i32)])?;
        }
        Ok(())
    }

    fn hashmap_find_slot(
        &mut self,
        repr: &WasmHashMapRepr,
        hash: u64,
        key_ptr: u32,
    ) -> Result<(bool, u32), WasmExecutionError> {
        if repr.cap == 0 || repr.states == 0 {
            return Ok((false, 0));
        }
        let mask = repr.cap - 1;
        let start = (hash as u32) & mask;
        let mut first_tombstone = 0u32;
        let mut has_tombstone = false;
        let mut current = start;
        let mut probes = 0u32;
        while probes < repr.cap {
            let state = self.read_u8(repr.states.checked_add(current).unwrap_or(0))?;
            if state == TABLE_STATE_EMPTY {
                return Ok((
                    false,
                    if has_tombstone {
                        first_tombstone
                    } else {
                        current
                    },
                ));
            }
            if state == TABLE_STATE_TOMBSTONE {
                if !has_tombstone {
                    first_tombstone = current;
                    has_tombstone = true;
                }
            } else {
                let hash_slot = self.hashmap_hash_slot(repr.hashes, current)?;
                let stored_hash = if hash_slot == 0 {
                    0
                } else {
                    self.read_u64(hash_slot)?
                };
                if stored_hash == hash && key_ptr != 0 && repr.entries != 0 && repr.key_eq_fn != 0 {
                    let entry_ptr =
                        self.hashmap_entry_ptr(repr.entries, repr.entry_size, current)?;
                    let result = self.invoke(
                        repr.key_eq_fn,
                        &[Value::I32(entry_ptr as i32), Value::I32(key_ptr as i32)],
                    )?;
                    if matches!(result.first(), Some(Value::I32(v)) if *v != 0) {
                        return Ok((true, current));
                    }
                }
            }
            current = (current + 1) & mask;
            probes += 1;
        }
        Ok((false, 0))
    }

    fn hashmap_rehash(
        &mut self,
        repr: &WasmHashMapRepr,
        new_cap: u32,
    ) -> Result<WasmHashMapRepr, WasmExecutionError> {
        if new_cap == 0 {
            return Ok(WasmHashMapRepr {
                entries: 0,
                states: 0,
                hashes: 0,
                cap: 0,
                tombstones: 0,
                len: 0,
                ..*repr
            });
        }
        let entry_bytes =
            new_cap
                .checked_mul(repr.entry_size)
                .ok_or_else(|| WasmExecutionError {
                    message: "hashmap rehash entry buffer overflow".into(),
                })?;
        let hash_bytes = new_cap.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashmap rehash hash buffer overflow".into(),
        })?;
        let max_align = repr.key_align.max(repr.value_align).max(1);
        let entries = self.allocate_heap_block(entry_bytes, max_align)?;
        let states = self.allocate_heap_block(new_cap, 1)?;
        let hashes = self.allocate_heap_block(hash_bytes, 8)?;

        let mut rebuilt = WasmHashMapRepr {
            entries,
            states,
            hashes,
            cap: new_cap,
            tombstones: 0,
            len: 0,
            ..*repr
        };
        if repr.cap != 0 && repr.states != 0 {
            let mask = new_cap - 1;
            for idx in 0..repr.cap {
                let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                if state != TABLE_STATE_FULL {
                    continue;
                }
                let old_hash_slot = self.hashmap_hash_slot(repr.hashes, idx)?;
                let hash_value = if old_hash_slot == 0 {
                    0
                } else {
                    self.read_u64(old_hash_slot)?
                };
                let mut insert_index = (hash_value as u32) & mask;
                while self.read_u8(states.checked_add(insert_index).unwrap_or(0))?
                    == TABLE_STATE_FULL
                {
                    insert_index = (insert_index + 1) & mask;
                }
                let src_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, idx)?;
                let dest_ptr = self.hashmap_entry_ptr(entries, repr.entry_size, insert_index)?;
                if repr.entry_size != 0 && src_ptr != 0 && dest_ptr != 0 {
                    let data = self.read_bytes(src_ptr, repr.entry_size)?;
                    self.store_bytes(dest_ptr, 0, &data)?;
                }
                let new_hash_slot = self.hashmap_hash_slot(hashes, insert_index)?;
                if new_hash_slot != 0 {
                    self.write_u64(new_hash_slot, hash_value)?;
                }
                self.write_u8(
                    states.checked_add(insert_index).unwrap_or(0),
                    TABLE_STATE_FULL,
                )?;
                rebuilt.len = rebuilt.len.saturating_add(1);
            }
        }
        Ok(rebuilt)
    }

    fn call_async_function(
        &mut self,
        func_index: u32,
        user_args: &[Value],
    ) -> Result<u32, WasmExecutionError> {
        let import_count = self.module.imports.len();
        let func_slot =
            func_index
                .checked_sub(import_count as u32)
                .ok_or_else(|| WasmExecutionError {
                    message: format!(
                        "function index {func_index} out of range (ctx={} stack={})",
                        self.current_wasm_context(),
                        self.format_call_stack()
                    ),
                })? as usize;
        let function = self
            .module
            .functions
            .get(func_slot)
            .ok_or_else(|| WasmExecutionError {
                message: format!(
                    "function index {func_index} out of range (ctx={} stack={})",
                    self.current_wasm_context(),
                    self.format_call_stack()
                ),
            })?;
        let sig = self
            .module
            .types
            .get(function.type_index as usize)
            .ok_or_else(|| WasmExecutionError {
                message: format!("type index {} out of range", function.type_index),
            })?;
        let ptr = if let Some(ValueType::I32) = sig.results.first() {
            let result = self.invoke(func_index, user_args)?;
            result
                .first()
                .and_then(|value| value.as_i32().ok())
                .unwrap_or(0)
                .max(0) as u32
        } else {
            if !sig.results.is_empty() {
                return Err(WasmExecutionError {
                    message: "async entry/testcase must return i32 task pointer in wasm backend"
                        .into(),
                });
            }
            if sig.params.is_empty() || sig.params[0] != ValueType::I32 {
                return Err(WasmExecutionError {
                    message:
                        "async entry/testcase must accept an i32 return slot parameter in wasm backend"
                            .into(),
                });
            }
            let out_ptr = self.allocate_heap_block(128, 4)?;
            let mut params = Vec::with_capacity(sig.params.len());
            params.push(Value::I32(out_ptr as i32));
            for _ in 1..sig.params.len() {
                params.push(Value::I32(0));
            }
            if !user_args.is_empty() {
                params.extend_from_slice(user_args);
            }
            let _ = self.invoke(func_index, &params)?;
            out_ptr
        };
        self.register_future_node(ptr)?;
        let layout = self.async_layout;
        let vtable_ptr = self
            .load_i32(ptr, layout.future_header_vtable_offset)
            .unwrap_or(0);
        if self.future_ready(ptr)? || vtable_ptr == 0 {
            self.mark_future_completed(ptr)?;
        } else {
            self.enqueue_future(ptr);
        }
        Ok(ptr)
    }

    fn register_future_node(&mut self, base: u32) -> Result<(), WasmExecutionError> {
        if self.async_nodes.contains_key(&base) {
            return Ok(());
        }
        let flags = self.future_flags(base)?;
        let completed = flags & (FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED) != 0;
        let faulted = flags & FUTURE_FLAG_FAULTED != 0;
        let cancelled = flags & FUTURE_FLAG_CANCELLED != 0;
        self.async_nodes.insert(
            base,
            AsyncNode {
                waiters: Vec::new(),
                completed,
                faulted,
                cancelled,
                queued: false,
                result_offset: None,
                borrows: HashMap::new(),
            },
        );
        if completed {
            self.wake_waiters(base);
        }
        Ok(())
    }

    fn future_flags(&self, base: u32) -> Result<u32, WasmExecutionError> {
        let layout = self.async_layout;
        self.read_u32(base + layout.future_header_flags_offset)
    }

    fn enqueue_future(&mut self, base: u32) {
        let entry = self.async_nodes.entry(base).or_insert_with(|| AsyncNode {
            waiters: Vec::new(),
            completed: false,
            faulted: false,
            cancelled: false,
            queued: false,
            result_offset: None,
            borrows: HashMap::new(),
        });
        if entry.completed || entry.queued {
            return;
        }
        entry.queued = true;
        self.ready_queue.push_back(base);
    }

    fn poll_future(&mut self, base: u32) -> Result<AwaitStatus, WasmExecutionError> {
        self.register_future_node(base)?;
        if self.future_ready(base)? {
            self.mark_future_completed(base)?;
            return Ok(AwaitStatus::Ready);
        }
        let layout = self.async_layout;
        let vtable_ptr = self.load_i32(base, layout.future_header_vtable_offset)? as u32;
        if vtable_ptr == 0 {
            self.mark_future_completed(base)?;
            return Ok(AwaitStatus::Ready);
        }
        let poll_idx = self.load_i32(vtable_ptr, 0)? as u32;
        let state_ptr = self.load_i32(base, layout.future_header_state_offset)? as u32;
        let ctx_ptr = self
            .load_i32(base, layout.future_header_executor_context_offset)
            .unwrap_or(0) as u32;
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            eprintln!(
                "[wasm-async] poll_future base={:#x} vtable={:#x} poll_idx={:#x} state_ptr={:#x} ctx_ptr={:#x}",
                base, vtable_ptr, poll_idx, state_ptr, ctx_ptr
            );
        }
        let prev = self.switch_borrow_context(Some(base));
        let status_raw = self
            .invoke(
                poll_idx,
                &[Value::I32(state_ptr as i32), Value::I32(ctx_ptr as i32)],
            )?
            .first()
            .and_then(|value| value.as_i32().ok())
            .unwrap_or(0);
        self.switch_borrow_context(prev);
        let status = if status_raw == AwaitStatus::Ready as i32 {
            AwaitStatus::Ready
        } else {
            AwaitStatus::Pending
        };
        if matches!(status, AwaitStatus::Ready) {
            self.mark_future_completed(base)?;
        }
        Ok(status)
    }

    fn mark_future_completed(&mut self, base: u32) -> Result<(), WasmExecutionError> {
        self.register_future_node(base)?;
        let mut flags = self.future_flags(base)?;
        let ready_mask = FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED;
        if flags & ready_mask != ready_mask {
            flags |= ready_mask;
            let layout = self.async_layout;
            self.write_u32(base + layout.future_header_flags_offset, flags)?;
        }
        // If this is a Task<T>, mirror completion into the inner future.
        let layout = self.async_layout;
        let inner_base = self.resolve_inner_base(base);
        if inner_base != base {
            let inner_flags = self
                .read_u32(inner_base + layout.future_header_flags_offset)
                .unwrap_or_default();
            let cancel_fault = flags & (FUTURE_FLAG_CANCELLED | FUTURE_FLAG_FAULTED);
            let propagated_flags = inner_flags | cancel_fault | ready_mask;
            let _ = self.write_u32(
                inner_base + layout.future_header_flags_offset,
                propagated_flags,
            );
            self.store_completion_flag(inner_base);
        } else {
            self.store_completion_flag(base);
        }
        if let Some(node) = self.async_nodes.get_mut(&base) {
            node.completed = true;
            node.faulted = flags & FUTURE_FLAG_FAULTED != 0;
            node.cancelled = flags & FUTURE_FLAG_CANCELLED != 0;
            node.queued = false;
        }
        self.wake_waiters(base);
        Ok(())
    }

    fn store_completion_flag(&mut self, base: u32) {
        let offset = self.async_layout.future_completed_offset;
        if self.async_layout.bool_size <= 1 {
            let _ = self.store_i8(base, offset, 1);
        } else {
            let _ = self.store_i32(base, offset, 1);
        }
    }

    fn wake_waiters(&mut self, base: u32) {
        if let Some(node) = self.async_nodes.get_mut(&base) {
            let waiters = std::mem::take(&mut node.waiters);
            for waiter in waiters {
                self.enqueue_future(waiter);
            }
        }
    }

    pub(crate) fn await_future_once(
        &mut self,
        base: u32,
    ) -> Result<AwaitStatus, WasmExecutionError> {
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            eprintln!("[wasm-async] await_future_once base={:#x}", base);
        }
        self.register_future_node(base)?;
        if self.future_ready(base)? {
            self.mark_future_completed(base)?;
            return Ok(AwaitStatus::Ready);
        }
        let status = self.poll_future(base)?;
        if matches!(status, AwaitStatus::Pending) {
            if let Some(current) = self.current_future {
                self.register_future_node(current)?;
                if let Some(node) = self.async_nodes.get_mut(&base) {
                    if !node.waiters.contains(&current) {
                        node.waiters.push(current);
                    }
                }
            }
            self.enqueue_future(base);
        }
        Ok(status)
    }

    pub(crate) fn await_future_blocking(
        &mut self,
        base: u32,
        result_len: Option<u32>,
    ) -> Result<i32, WasmExecutionError> {
        let in_task = self.current_future.is_some();
        let debug_async = std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok();
        if debug_async {
            eprintln!("[wasm-async] await_future_blocking base={:#x}", base);
            // Snapshot a wider window so async state locals (including cancellation flags)
            // are visible during debugging.
            let remaining = self.memory_len().saturating_sub(base as usize);
            let window_len: u32 = remaining.min(320).try_into().unwrap_or(0);
            if window_len > 0 {
                if let Ok(window) = self.read_bytes(base, window_len) {
                    eprintln!(
                        "[wasm-async] await_future_blocking mem[{:#x}..{:#x}]={:?}",
                        base,
                        base + window_len,
                        window
                    );
                }
            }
        }
        self.register_future_node(base)?;
        self.enqueue_future(base);
        let mut polls = 0usize;
        loop {
            if self.future_ready(base)? {
                let flags = self.future_flags(base)?;
                if flags & FUTURE_FLAG_FAULTED != 0 {
                    return Err(WasmExecutionError {
                        message: format!("async future at 0x{base:08X} faulted"),
                    });
                }
                self.mark_future_completed(base)?;
                if flags & FUTURE_FLAG_CANCELLED != 0 {
                    return Ok(0);
                }
                if debug_async {
                    eprintln!("[wasm-async] future {:#x} completed, loading result", base);
                }
                let len = result_len
                    .or_else(|| {
                        if in_task {
                            None
                        } else {
                            self.options.async_result_len
                        }
                    })
                    .unwrap_or(self.async_layout.uint_size.max(4));
                let align = if in_task {
                    None
                } else {
                    self.options.async_result_align
                };
                return self.load_future_result(base, len, align);
            }
            let next = match self.ready_queue.pop_front() {
                Some(value) => value,
                None => {
                    return Err(WasmExecutionError {
                        message: format!(
                            "async scheduler stalled while waiting for future at 0x{base:08X}"
                        ),
                    });
                }
            };
            polls += 1;
            if polls > 4096 {
                return Err(WasmExecutionError {
                    message: format!(
                        "async scheduler exceeded poll budget while awaiting 0x{base:08X}"
                    ),
                });
            }
            if let Some(node) = self.async_nodes.get_mut(&next) {
                node.queued = false;
            }
            let status = self.poll_future(next)?;
            if debug_async {
                eprintln!(
                    "[wasm-async] poll #{polls} future={:#x} status={:?}",
                    next, status
                );
            }
            if matches!(status, AwaitStatus::Pending) && !self.future_ready(next)? {
                self.enqueue_future(next);
            }
        }
    }

    fn yield_current(&mut self) -> AwaitStatus {
        if let Some(current) = self.current_future {
            self.enqueue_future(current);
            AwaitStatus::Pending
        } else {
            AwaitStatus::Ready
        }
    }

    pub(crate) fn cancel_future(&mut self, base: u32) -> Result<(), WasmExecutionError> {
        let debug_async = std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok();
        if debug_async {
            eprintln!("[wasm-async] cancel_future base={:#x}", base);
        }
        self.register_future_node(base)?;
        let mut flags = self.future_flags(base)?;
        flags |= FUTURE_FLAG_CANCELLED | FUTURE_FLAG_COMPLETED | FUTURE_FLAG_READY;
        let layout = self.async_layout;
        self.write_u32(base + layout.future_header_flags_offset, flags)?;
        let _ = self.write_u32(base + layout.task_flags_offset, flags);
        let inner_base = self.resolve_inner_base(base);
        if self
            .read_u32(inner_base + layout.future_header_flags_offset)
            .is_ok()
        {
            let result_offset =
                layout.result_offset(layout.uint_size.max(4), Some(layout.uint_align));
            let _ = self.write_u32(inner_base + layout.future_header_flags_offset, flags);
            self.store_completion_flag(inner_base);
            let _ = self.store_i32(inner_base, result_offset, 0);
        }
        if let Some(node) = self.async_nodes.get_mut(&base) {
            node.completed = true;
            node.cancelled = true;
            node.queued = false;
        }
        self.wake_waiters(base);
        Ok(())
    }

    fn future_ready(&self, base: u32) -> Result<bool, WasmExecutionError> {
        let layout = self.async_layout;
        let flags = self.read_u32(base + layout.future_header_flags_offset)?;
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            eprintln!(
                "[wasm-async] future_ready base={:#x} flags=0x{:x}",
                base, flags
            );
        }
        Ok(flags & FUTURE_FLAG_COMPLETED != 0
            || flags & FUTURE_FLAG_READY != 0
            || flags & FUTURE_FLAG_CANCELLED != 0
            || flags & FUTURE_FLAG_FAULTED != 0)
    }

    fn resolve_inner_base(&self, base: u32) -> u32 {
        let layout = self.async_layout;
        if layout.task_inner_future_offset == 0 {
            return base;
        }
        let candidate = base.saturating_add(layout.task_inner_future_offset);
        let base_flags = self
            .read_u32(base + layout.future_header_flags_offset)
            .unwrap_or_default();
        let candidate_flags = self
            .read_u32(candidate + layout.future_header_flags_offset)
            .unwrap_or_default();
        if base_flags != 0 && candidate_flags == 0 {
            base
        } else {
            candidate
        }
    }

    fn load_future_result(
        &mut self,
        base: u32,
        result_len: u32,
        result_align: Option<u32>,
    ) -> Result<i32, WasmExecutionError> {
        let layout = self.async_layout;
        let task_flags = self.read_u32(base + layout.task_flags_offset)?;
        let debug_async = std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok();
        let node_result_offset = self
            .async_nodes
            .get(&base)
            .and_then(|node| node.result_offset);
        let state_ptr = self
            .load_i32(base, layout.future_header_state_offset)
            .unwrap_or_default() as u32;
        if debug_async {
            eprintln!(
                "[wasm-async] load_future_result base={:#x} state_ptr={:#x} task_flags=0x{:x} len={} align={:?}",
                base, state_ptr, task_flags, result_len, result_align
            );
        }
        let header_flags = self.read_u32(base + layout.future_header_flags_offset)?;
        if header_flags & FUTURE_FLAG_CANCELLED != 0 || task_flags & FUTURE_FLAG_CANCELLED != 0 {
            return Ok(0);
        }
        let inner_base = self.resolve_inner_base(base);
        let inner_flags = self
            .read_u32(inner_base + layout.future_header_flags_offset)
            .unwrap_or_default();
        let inner_completed = self
            .load_i32(inner_base, layout.future_completed_offset)
            .unwrap_or_default();
        let result_offset =
            node_result_offset.unwrap_or_else(|| layout.result_offset(result_len, result_align));
        if header_flags & FUTURE_FLAG_CANCELLED != 0
            || task_flags & FUTURE_FLAG_CANCELLED != 0
            || inner_flags & FUTURE_FLAG_CANCELLED != 0
        {
            return Ok(0);
        }

        if result_len == self.async_layout.bool_size {
            let value = *self
                .load_bytes(inner_base, result_offset, 1)?
                .first()
                .unwrap_or(&0);
            let value = i32::from(value != 0);
            if debug_async {
                eprintln!(
                    "[wasm-async] load_future_result bool value={} addr={:#x}",
                    value,
                    inner_base + result_offset
                );
            }
            return Ok(value);
        }
        if debug_async {
            let probe_addr = inner_base.saturating_add(result_offset.saturating_sub(4));
            match self.read_bytes(probe_addr, result_len.max(16)) {
                Ok(window) => {
                    eprintln!(
                        "[wasm-async] load_future_result probe addr={:#x} len={} bytes={:?}",
                        probe_addr,
                        result_len.max(16),
                        window
                    );
                }
                Err(err) => {
                    eprintln!(
                        "[wasm-async] load_future_result probe failed addr={:#x} len={} err={}",
                        probe_addr,
                        result_len.max(16),
                        err.message
                    );
                }
            }
            eprintln!(
                "[wasm-async] load_future_result inner_base={:#x} inner_flags=0x{:x} inner_completed={} result_len={} result_offset={} header_flags=0x{:x}",
                inner_base, inner_flags, inner_completed, result_len, result_offset, header_flags
            );
        }
        let mut result_value = self.load_i32(inner_base, result_offset).ok();
        let mut exported: Option<i32> = None;
        let mut observed_offset = None;
        if result_len <= 4 {
            exported = self
                .module
                .exports
                .get("chic_rt_async_task_bool_result")
                .and_then(|idx| self.invoke(*idx, &[Value::I32(base as i32)]).ok())
                .and_then(|values| values.first().copied())
                .and_then(|value| value.as_i32().ok())
                .map(|v| if result_len == 1 { (v != 0) as i32 } else { v })
                .or_else(|| {
                    self.module
                        .exports
                        .get("chic_rt_async_task_int_result")
                        .and_then(|idx| self.invoke(*idx, &[Value::I32(base as i32)]).ok())
                        .and_then(|values| values.first().copied())
                        .and_then(|value| value.as_i32().ok())
                });
        }
        if debug_async {
            let window_start =
                inner_base.saturating_add(layout.future_completed_offset.saturating_sub(4));
            if let Ok(window) = self.read_bytes(window_start, 40) {
                eprintln!(
                    "[wasm-async] load_future_result window[{:#x}..{:#x}]={:?}",
                    window_start,
                    window_start + 40,
                    window
                );
            }
        }
        if node_result_offset.is_none() && result_len <= 4 && result_value.is_none() {
            for step in [0, 4, 8, 12, 16] {
                let candidate = result_offset.saturating_add(step);
                if let Ok(value) = self.load_i32(inner_base, candidate) {
                    if value != 0 {
                        observed_offset = Some(candidate);
                        result_value = Some(value);
                        if let Some(node) = self.async_nodes.get_mut(&base) {
                            node.result_offset = Some(candidate);
                        }
                        break;
                    }
                }
            }
        }
        if (result_value.is_none() || result_value == Some(0))
            && result_len == self.async_layout.bool_size
        {
            let alt_offset = self.async_layout.result_offset(
                self.async_layout.uint_size.max(4),
                Some(self.async_layout.uint_align),
            );
            if alt_offset != result_offset {
                if let Ok(value) = self.load_i32(inner_base, alt_offset) {
                    if value != 0 {
                        observed_offset = Some(alt_offset);
                        result_value = Some(value);
                        if let Some(node) = self.async_nodes.get_mut(&base) {
                            node.result_offset = Some(alt_offset);
                        }
                    }
                }
            }
            if result_value.is_none() || result_value == Some(0) {
                for delta in 1..=8 {
                    let candidate = result_offset.saturating_add(delta);
                    if let Ok(value) = self.load_i32(inner_base, candidate) {
                        if value != 0 {
                            observed_offset = Some(candidate);
                            result_value = Some(value);
                            if let Some(node) = self.async_nodes.get_mut(&base) {
                                node.result_offset = Some(candidate);
                            }
                            break;
                        }
                    }
                }
            }
        }
        let final_offset = observed_offset.unwrap_or(result_offset);
        let mut value = result_value
            .or_else(|| self.load_i32(inner_base, final_offset).ok())
            .or(exported)
            .unwrap_or(0);
        if result_len == self.async_layout.bool_size {
            value = (value != 0) as i32;
        }
        if debug_async {
            eprintln!(
                "[wasm-async] load_future_result value={} addr={:#x}",
                value,
                inner_base + final_offset
            );
        }
        Ok(value)
    }

    fn read_mmio(
        &self,
        address: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<u64, WasmExecutionError> {
        let width = Self::width_from_bits(width_bits)?;
        let (endianness, space) = decode_flags(flags);
        let key = (space, address);
        let stored = *self.mmio.get(&key).unwrap_or(&0);
        decode_value(stored, width, endianness).map_err(Self::mmio_width_error)
    }

    fn write_mmio(
        &mut self,
        address: u64,
        value: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<(), WasmExecutionError> {
        let width = Self::width_from_bits(width_bits)?;
        let (endianness, space) = decode_flags(flags);
        let key = (space, address);
        let stored = encode_value(value, width, endianness).map_err(Self::mmio_width_error)?;
        self.mmio.insert(key, stored);
        Ok(())
    }

    fn width_from_bits(width_bits: u32) -> Result<u16, WasmExecutionError> {
        if width_bits == 0 || width_bits > 64 || width_bits % 8 != 0 {
            return Err(Self::invalid_width(width_bits));
        }
        u16::try_from(width_bits).map_err(|_| Self::invalid_width(width_bits))
    }

    fn invalid_width(width_bits: u32) -> WasmExecutionError {
        WasmExecutionError {
            message: format!("invalid MMIO width {width_bits}; expected 8, 16, 32, or 64 bits"),
        }
    }

    fn mmio_width_error(err: InvalidWidthError) -> WasmExecutionError {
        WasmExecutionError {
            message: err.to_string(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn set_mmio_value(&mut self, address: u64, value: u64) {
        self.mmio.insert((AddressSpaceId::DEFAULT, address), value);
    }

    #[cfg(test)]
    pub fn test_mmio_write(
        &mut self,
        address: u64,
        value: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<(), WasmExecutionError> {
        self.write_mmio(address, value, width_bits, flags)
    }

    #[cfg(test)]
    pub fn test_mmio_read(
        &self,
        address: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<u64, WasmExecutionError> {
        self.read_mmio(address, width_bits, flags)
    }

    fn allocate_object_instance(&mut self, type_id: u64) -> Result<u32, WasmExecutionError> {
        let swapped = type_id.swap_bytes();
        let (resolved_type_id, metadata) = if let Some(meta) = self.type_metadata.get(&type_id) {
            (type_id, meta.clone())
        } else if let Some(meta) = self.type_metadata.get(&swapped) {
            (swapped, meta.clone())
        } else {
            if std::env::var("CHIC_DEBUG_WASM_OBJECT_NEW").is_ok() {
                eprintln!(
                    "[wasm-object-new] missing type=0x{type_id:016x} (meta_entries={})",
                    self.type_metadata.len()
                );
            }
            return Err(WasmExecutionError {
                message: format!(
                    "chic_rt.object_new missing type metadata for type_id=0x{type_id:016X}"
                ),
            });
        };
        if std::env::var("CHIC_DEBUG_WASM_OBJECT_NEW").is_ok() {
            eprintln!(
                "[wasm-object-new] type=0x{type_id:016x} resolved=0x{resolved_type_id:016x} size={} align={}",
                metadata.size, metadata.align
            );
        }
        if metadata.size == 0 || metadata.align == 0 {
            return Err(WasmExecutionError {
                message: format!(
                    "chic_rt.object_new invalid metadata for type_id=0x{type_id:016X} (size={}, align={})",
                    metadata.size, metadata.align
                ),
            });
        }
        let address = self.allocate_heap_block(metadata.size, metadata.align)?;
        if std::env::var("CHIC_DEBUG_WASM_OBJECT_NEW").is_ok() {
            eprintln!(
                "[wasm-object-new] type=0x{type_id:016x} allocated=0x{address:08x}",
                type_id = type_id,
                address = address
            );
        }
        Ok(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::wasm_executor::executor::options::WasmExecutionOptions;
    use crate::runtime::wasm_executor::instructions::Instruction;
    use crate::runtime::wasm_executor::module::{Function, Module};
    use crate::runtime::wasm_executor::types::{FuncType, ValueType};
    use std::collections::HashMap;

    fn empty_module_with_memory() -> Module {
        Module {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            function_names: Vec::new(),
            tables: Vec::new(),
            exports: HashMap::new(),
            memory_min_pages: Some(1),
            globals: Vec::new(),
            data_segments: Vec::new(),
            interface_defaults: Vec::new(),
            type_metadata: Vec::new(),
            hash_glue: Vec::new(),
            eq_glue: Vec::new(),
        }
    }

    #[test]
    fn await_future_reads_ready_result_from_future_layout() {
        let module = empty_module_with_memory();
        let mut exec =
            Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
        let layout = exec.async_layout;
        exec.write_u32(
            layout.future_header_flags_offset,
            FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED,
        )
        .unwrap();
        let result_offset = layout.result_offset(layout.uint_size.max(4), Some(layout.uint_align));
        exec.store_i32(0, result_offset, 42).unwrap();
        let value = exec.await_future_blocking(0, None).expect("await result");
        assert_eq!(value, 42);
    }

    #[test]
    fn await_future_reports_pending_then_completes() {
        let module = Module {
            types: vec![FuncType {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            }],
            imports: Vec::new(),
            functions: vec![Function {
                type_index: 0,
                locals: Vec::new(),
                code: vec![
                    Instruction::I32Const(0),
                    Instruction::Return,
                    Instruction::End,
                ],
            }],
            function_names: Vec::new(),
            tables: Vec::new(),
            exports: HashMap::new(),
            memory_min_pages: Some(1),
            globals: Vec::new(),
            data_segments: Vec::new(),
            interface_defaults: Vec::new(),
            type_metadata: Vec::new(),
            hash_glue: Vec::new(),
            eq_glue: Vec::new(),
        };
        let mut exec =
            Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
        let base = 0x100;
        let layout = exec.async_layout;
        exec.write_u32(base + layout.future_header_vtable_offset, 0x80)
            .unwrap();
        exec.write_u32(0x80, 0).unwrap();

        let status = exec.await_future_once(base).expect("await once");
        assert_eq!(status, AwaitStatus::Pending);
        assert!(exec.ready_queue.contains(&base));

        exec.write_u32(
            base + layout.future_header_flags_offset,
            FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED,
        )
        .unwrap();
        let result_offset = layout.result_offset(layout.uint_size.max(4), Some(layout.uint_align));
        exec.store_i32(base, result_offset, 7).unwrap();
        let value = exec
            .await_future_blocking(base, None)
            .expect("await completion");
        assert_eq!(value, 7);
    }

    #[test]
    fn await_future_blocking_reports_faulted_state() {
        let module = empty_module_with_memory();
        let mut exec =
            Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
        let layout = exec.async_layout;
        exec.write_u32(
            layout.future_header_flags_offset,
            FUTURE_FLAG_FAULTED | FUTURE_FLAG_COMPLETED,
        )
        .unwrap();
        let err = exec
            .await_future_blocking(0, None)
            .expect_err("faulted future");
        assert!(
            err.message.contains("faulted"),
            "unexpected error message: {}",
            err.message
        );
    }

    #[test]
    fn cancel_future_marks_flags_and_wakes_waiters() {
        let module = empty_module_with_memory();
        let mut exec =
            Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
        let layout = exec.async_layout;
        exec.write_u32(layout.future_header_flags_offset, FUTURE_FLAG_READY)
            .unwrap();
        exec.register_future_node(0).expect("register");
        exec.cancel_future(0).expect("cancel");
        let flags = exec.future_flags(0).expect("flags");
        assert!(flags & FUTURE_FLAG_CANCELLED != 0);
        assert!(flags & FUTURE_FLAG_COMPLETED != 0);
        assert!(flags & FUTURE_FLAG_READY != 0);
    }
}
