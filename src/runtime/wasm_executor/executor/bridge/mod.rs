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

mod abi;

mod collections;

mod hashmap_imports;
mod hashset_imports;

mod async_rt;

mod mmio;

mod strings;

mod vec;

mod int128;

mod dispatch;

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
mod tests;
