mod helpers;
mod invocation;
mod memory;
mod tracing;

use crate::mmio::AddressSpaceId;
use crate::runtime::error::RuntimeThrownException;
use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::module::{Module, TypeMetadataRecord};
#[cfg(test)]
use crate::runtime::wasm_executor::types::WasmValue;
use crate::runtime::wasm_executor::types::{Value, ValueType};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use super::options::{AsyncLayoutOverrides, WasmExecutionOptions, WasmExecutionTrace};
use super::runtime::{BorrowRuntimeKey, BorrowRuntimeRecord};
#[cfg(test)]
use super::runtime::{STRING_EMPTY_PTR, STRING_INLINE_CAPACITY, STRING_INLINE_TAG, WasmStringRepr};
use crate::runtime::float_env::{clear_flags, read_flags, rounding_mode, set_rounding_mode};
pub(crate) use tracing::SchedulerTracer;

#[derive(Clone, Copy, Debug)]
pub(super) struct AsyncLayout {
    pub ptr_size: u32,
    pub ptr_align: u32,
    pub bool_size: u32,
    pub bool_align: u32,
    pub uint_size: u32,
    pub uint_align: u32,
    pub future_header_state_offset: u32,
    pub future_header_vtable_offset: u32,
    pub future_header_executor_context_offset: u32,
    pub future_header_flags_offset: u32,
    pub future_completed_offset: u32,
    pub future_result_offset: u32,
    pub task_flags_offset: u32,
    pub task_inner_future_offset: u32,
    pub result_override: bool,
}

impl AsyncLayout {
    fn align_to(value: u32, align: u32) -> u32 {
        let a = align.max(1);
        ((value + a - 1) / a) * a
    }

    fn type_size_align(
        metadata: &HashMap<u64, TypeMetadataRecord>,
        name: &str,
    ) -> Option<(u32, u32)> {
        let digest = blake3::hash(name.as_bytes());
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest.as_bytes()[..8]);
        let type_id = u64::from_le_bytes(bytes);
        metadata
            .get(&type_id)
            .map(|record| (record.size, record.align))
    }

    pub(super) fn from_metadata(
        metadata: &HashMap<u64, TypeMetadataRecord>,
        overrides: Option<&AsyncLayoutOverrides>,
    ) -> Self {
        let layout_override = overrides;
        let (mut ptr_size, mut ptr_align) = Self::type_size_align(metadata, "isize")
            .or_else(|| Self::type_size_align(metadata, "usize"))
            .or_else(|| Self::type_size_align(metadata, "nint"))
            .or_else(|| Self::type_size_align(metadata, "nuint"))
            .unwrap_or((4, 4));
        ptr_size = ptr_size.max(1).min(4);
        ptr_align = ptr_align.max(1).min(ptr_size);
        let (uint_size, uint_align) = Self::type_size_align(metadata, "uint")
            .or_else(|| Self::type_size_align(metadata, "int"))
            .unwrap_or((4, 4));
        let (bool_size, bool_align) = Self::type_size_align(metadata, "bool").unwrap_or((1, 1));
        let (_int_size, int_align) = Self::type_size_align(metadata, "int").unwrap_or((4, 4));
        let header_size = ptr_size * 4;
        let header_align = ptr_align.max(uint_align);

        let future_completed_offset = Self::align_to(header_size, bool_align);
        let default_result_align = int_align.max(1);
        let future_result_offset =
            Self::align_to(future_completed_offset + bool_size, default_result_align);
        let future_align = header_align.max(bool_align).max(default_result_align);

        let pointer_align = ptr_align.max(future_align).max(uint_align);
        let base_size = Self::align_to(header_size + uint_size, pointer_align);
        let task_inner_future_offset = Self::align_to(base_size, future_align);

        let mut layout = Self {
            ptr_size,
            ptr_align,
            bool_size,
            bool_align,
            uint_size,
            uint_align,
            future_header_state_offset: 0,
            future_header_vtable_offset: ptr_size,
            future_header_executor_context_offset: ptr_size * 2,
            future_header_flags_offset: ptr_size * 3,
            future_completed_offset,
            future_result_offset,
            task_flags_offset: header_size,
            task_inner_future_offset,
            result_override: false,
        };

        if let Some(override_layout) = layout_override {
            layout.result_override = override_layout.future_result_offset.is_some();
            if let Some(value) = override_layout.future_header_state_offset {
                layout.future_header_state_offset = value;
            }
            if let Some(value) = override_layout.future_header_vtable_offset {
                layout.future_header_vtable_offset = value;
            }
            if let Some(value) = override_layout.future_header_executor_context_offset {
                layout.future_header_executor_context_offset = value;
            }
            if let Some(value) = override_layout.future_header_flags_offset {
                layout.future_header_flags_offset = value;
            }
            if let Some(value) = override_layout.future_completed_offset {
                layout.future_completed_offset = value;
            }
            if let Some(value) = override_layout.future_result_offset {
                layout.future_result_offset = value;
            }
            if let Some(value) = override_layout.task_flags_offset {
                layout.task_flags_offset = value;
            }
            if let Some(value) = override_layout.task_inner_future_offset {
                layout.task_inner_future_offset = value;
            }
        }

        layout
    }

    #[inline]
    pub(super) fn result_offset(&self, result_len: u32, result_align: Option<u32>) -> u32 {
        if self.result_override && result_len <= self.bool_size {
            return self.future_result_offset;
        }
        if result_len <= self.bool_size {
            return self.future_result_offset;
        }
        let align = result_align.unwrap_or_else(|| {
            if result_len <= self.bool_size {
                self.bool_align.max(1)
            } else if result_len <= 2 {
                2
            } else if result_len <= self.uint_size {
                self.uint_align.max(4)
            } else {
                self.ptr_align
                    .max(self.uint_align)
                    .max(self.ptr_size.max(4))
            }
        });
        Self::align_to(self.future_completed_offset + self.bool_size, align)
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct AsyncNode {
    pub waiters: Vec<u32>,
    pub completed: bool,
    pub faulted: bool,
    pub cancelled: bool,
    pub queued: bool,
    pub result_offset: Option<u32>,
    pub borrows: HashMap<BorrowRuntimeKey, BorrowRuntimeRecord>,
}

struct HostFile {
    file: File,
    readable: bool,
    writable: bool,
}

enum HostSocket {
    Pending {
        _domain: i32,
        _typ: i32,
        _proto: i32,
    },
    Tcp(TcpStream),
}

pub struct Executor<'a> {
    pub(super) module: &'a Module,
    memory: Vec<u8>,
    globals: Vec<RuntimeGlobal>,
    pub(super) options: WasmExecutionOptions,
    pub(super) borrow_records: HashMap<BorrowRuntimeKey, BorrowRuntimeRecord>,
    pub(super) root_borrows: HashMap<BorrowRuntimeKey, BorrowRuntimeRecord>,
    pub(super) mmio: HashMap<(AddressSpaceId, u64), u64>,
    pub(super) type_metadata: HashMap<u64, TypeMetadataRecord>,
    pub(super) hash_glue: HashMap<u64, u32>,
    pub(super) eq_glue: HashMap<u64, u32>,
    pub(super) heap_cursor: u32,
    pub(super) heap_allocations: HashMap<u32, usize>,
    pub(super) host_stdout: Vec<u8>,
    pub(super) host_stderr: Vec<u8>,
    pub(super) host_stdin: Vec<u8>,
    pub(super) host_stdin_cursor: usize,
    pub(super) host_terminals: (bool, bool, bool),
    host_files: HashMap<i32, HostFile>,
    host_sockets: HashMap<i32, HostSocket>,
    next_file_handle: i32,
    next_socket_handle: i32,
    next_thread_id: u32,
    pub(super) async_nodes: HashMap<u32, AsyncNode>,
    pub(super) ready_queue: VecDeque<u32>,
    pub(super) current_future: Option<u32>,
    pub(super) async_layout: AsyncLayout,
    pub(super) start_time: Instant,
    pub(super) call_depth: usize,
    pub(super) current_function: Option<u32>,
    pub(super) call_stack: Vec<u32>,
    pub(super) current_import: Option<(String, String)>,
    pub(super) pending_exception: Option<RuntimeThrownException>,
    pub(super) last_arc_header: Option<u32>,
    pub(super) last_arc_payload: Option<u32>,
    pub(super) last_object_new: Option<u32>,
    pub(super) tracked_fn_range: Option<(u32, u32)>,
    pub(super) last_fn_struct: Option<Vec<u8>>,
}

impl<'a> Executor<'a> {
    #[cfg(test)]
    pub fn new(module: &'a Module) -> Self {
        Self::with_options(module, &WasmExecutionOptions::default())
            .expect("default execution options must be valid")
    }

    pub(super) fn memory_len(&self) -> usize {
        self.memory.len()
    }

    pub fn with_options(
        module: &'a Module,
        options: &WasmExecutionOptions,
    ) -> Result<Self, WasmExecutionError> {
        if let (Some(limit), Some(required)) = (options.memory_limit_pages, module.memory_min_pages)
        {
            if required > limit {
                return Err(WasmExecutionError {
                    message: format!(
                        "module requires at least {required} memory page(s) but execution is limited to {limit} page(s)"
                    ),
                });
            }
        }

        let mut memory = Vec::new();
        if let Some(pages) = module.memory_min_pages {
            let total = pages as usize * WASM_PAGE_SIZE;
            memory.resize(total, 0);
        }
        let mut globals: Vec<RuntimeGlobal> = module
            .globals
            .iter()
            .map(|global| RuntimeGlobal {
                ty: global.ty,
                mutable: global.mutable,
                value: global.initial,
            })
            .collect();
        if let Some(global) = globals.get_mut(0) {
            if matches!(global.ty, ValueType::I32) && global.mutable {
                let mut stack_base = memory
                    .len()
                    .saturating_sub(STACK_BASE_RED_ZONE)
                    .min(i32::MAX as usize)
                    .max(LINEAR_MEMORY_HEAP_BASE as usize);
                stack_base = stack_base.saturating_sub(stack_base % 16);
                if stack_base < LINEAR_MEMORY_HEAP_BASE as usize {
                    stack_base = LINEAR_MEMORY_HEAP_BASE as usize;
                }
                let stack_base = stack_base as i32;
                let current = match global.value {
                    Value::I32(v) => v,
                    _ => 0,
                };
                if current != stack_base && std::env::var_os("CHIC_DEBUG_WASM_SP").is_some() {
                    eprintln!("[wasm-sp] init global0 from {} to {}", current, stack_base);
                }
                global.value = Value::I32(stack_base);
            }
        }

        let mut data_end = 0usize;
        for segment in &module.data_segments {
            let start = segment.offset as usize;
            let end = start
                .checked_add(segment.bytes.len())
                .ok_or_else(|| WasmExecutionError {
                    message: "data segment exceeds linear memory address space".into(),
                })?;
            if end > memory.len() {
                return Err(WasmExecutionError {
                    message: format!(
                        "data segment requires {end} byte(s) of memory but module declares only {}",
                        memory.len()
                    ),
                });
            }
            memory[start..end].copy_from_slice(&segment.bytes);
            data_end = data_end.max(end);
        }

        let mut heap_cursor = (data_end as u32).max(LINEAR_MEMORY_HEAP_BASE);
        heap_cursor = heap_cursor.saturating_add((16 - (heap_cursor % 16)) % 16);
        if std::env::var_os("CHIC_DEBUG_WASM_DATA").is_some() {
            eprintln!(
                "[wasm-data] segments={} data_end=0x{data_end:08X} heap_base=0x{heap_cursor:08X} mem_len=0x{:08X}",
                module.data_segments.len(),
                memory.len()
            );
        }
        if let Some(global) = globals.get(0) {
            if matches!(global.ty, ValueType::I32) {
                let stack_base = match global.value {
                    Value::I32(v) => v.max(0) as u32,
                    _ => 0,
                };
                if heap_cursor >= stack_base.saturating_sub(256) && stack_base != 0 {
                    return Err(WasmExecutionError {
                        message: format!(
                            "linear memory exhausted: heap base 0x{heap_cursor:08X} overlaps stack base 0x{stack_base:08X}"
                        ),
                    });
                }
            }
        }
        let mut type_metadata = HashMap::new();
        for entry in &module.type_metadata {
            type_metadata.insert(entry.type_id, entry.clone());
        }
        if std::env::var_os("CHIC_DEBUG_WASM_TYPEMETA").is_some() {
            fn type_id_first(name: &str) -> u64 {
                let digest = blake3::hash(name.as_bytes());
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&digest.as_bytes()[..8]);
                u64::from_le_bytes(bytes)
            }
            fn type_id_last(name: &str) -> u64 {
                let digest = blake3::hash(name.as_bytes());
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&digest.as_bytes()[24..32]);
                u64::from_le_bytes(bytes)
            }

            let mut keys: Vec<_> = type_metadata.keys().copied().collect();
            keys.sort_unstable();
            eprintln!(
                "[wasm-typemeta] entries={} first_keys={:?}",
                keys.len(),
                keys.iter()
                    .take(12)
                    .map(|k| format!("0x{k:016X}"))
                    .collect::<Vec<_>>()
            );
            for name in [
                "bool",
                "int",
                "uint",
                "usize",
                "isize",
                "K",
                "V",
                "T",
                "nuint",
                "nint",
                "Std::Int32",
                "Std::UInt32",
                "Std::IntPtr",
                "Std::UIntPtr",
                "string",
                "str",
            ] {
                let id_first = type_id_first(name);
                let id_last = type_id_last(name);
                let meta = type_metadata.get(&id_first);
                eprintln!(
                    "[wasm-typemeta] lookup name={name} type_id_first=0x{id_first:016X} type_id_last=0x{id_last:016X} -> {}",
                    meta.as_ref()
                        .map(|m| format!("size={} align={}", m.size, m.align))
                        .unwrap_or_else(|| "missing".into())
                );
            }
        }
        let mut hash_glue = HashMap::new();
        for entry in &module.hash_glue {
            hash_glue.insert(entry.type_id, entry.function_index);
        }
        let mut eq_glue = HashMap::new();
        for entry in &module.eq_glue {
            eq_glue.insert(entry.type_id, entry.function_index);
        }
        if std::env::var_os("CHIC_DEBUG_WASM_TYPEMETA").is_some() {
            let mut keys: Vec<_> = eq_glue.keys().copied().collect();
            keys.sort_unstable();
            eprintln!(
                "[wasm-eqglue] entries={} first_keys={:?}",
                keys.len(),
                keys.iter()
                    .take(12)
                    .map(|k| format!("0x{k:016X}"))
                    .collect::<Vec<_>>()
            );
        }
        let async_layout =
            AsyncLayout::from_metadata(&type_metadata, options.async_layout.as_ref());
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() && !type_metadata.is_empty() {
            eprintln!("[wasm-async] type metadata entries:");
            for (type_id, meta) in &type_metadata {
                eprintln!(
                    "  type_id=0x{type_id:016x} size={} align={}",
                    meta.size, meta.align
                );
            }
            eprintln!(
                "[wasm-async] async layout: header_state={} vtable={} exec_ctx={} flags={} completed={} result={} task_flags={} task_inner={} override_result={}",
                async_layout.future_header_state_offset,
                async_layout.future_header_vtable_offset,
                async_layout.future_header_executor_context_offset,
                async_layout.future_header_flags_offset,
                async_layout.future_completed_offset,
                async_layout.future_result_offset,
                async_layout.task_flags_offset,
                async_layout.task_inner_future_offset,
                async_layout.result_override,
            );
        }
        Ok(Self {
            module,
            memory,
            globals,
            options: options.clone(),
            borrow_records: HashMap::new(),
            root_borrows: HashMap::new(),
            mmio: HashMap::new(),
            type_metadata,
            hash_glue,
            eq_glue,
            heap_cursor,
            heap_allocations: HashMap::new(),
            host_stdout: Vec::new(),
            host_stderr: Vec::new(),
            host_stdin: options.stdin.clone(),
            host_stdin_cursor: 0,
            host_terminals: (
                options.stdin_is_terminal,
                options.stdout_is_terminal,
                options.stderr_is_terminal,
            ),
            host_files: HashMap::new(),
            host_sockets: HashMap::new(),
            next_file_handle: HOST_FILE_BASE,
            next_socket_handle: HOST_SOCKET_BASE,
            next_thread_id: 1,
            async_nodes: HashMap::new(),
            ready_queue: VecDeque::new(),
            current_future: None,
            async_layout,
            start_time: Instant::now(),
            call_depth: 0,
            current_function: None,
            call_stack: Vec::new(),
            current_import: None,
            pending_exception: None,
            last_arc_header: None,
            last_arc_payload: None,
            last_object_new: None,
            tracked_fn_range: None,
            last_fn_struct: None,
        })
    }

    #[cfg(test)]
    pub fn has_type_metadata(&self, type_id: u64) -> bool {
        self.type_metadata.contains_key(&type_id)
    }

    #[cfg(test)]
    pub fn type_metadata_len(&self) -> usize {
        self.type_metadata.len()
    }

    #[cfg(test)]
    pub fn heap_cursor(&self) -> u32 {
        self.heap_cursor
    }

    pub fn call_with_trace(
        &mut self,
        func_index: u32,
        args: &[Value],
    ) -> Result<(Option<Value>, WasmExecutionTrace), WasmExecutionError> {
        clear_flags();
        let prev_rounding = rounding_mode();
        if let Some(mode) = self.options.rounding_mode {
            set_rounding_mode(mode);
        }
        let mut trace = WasmExecutionTrace::from_options(&self.options);
        let mut values = self.invoke(func_index, args)?;
        if values.len() > 1 {
            return Err(WasmExecutionError {
                message: format!(
                    "function returned {} value(s); expected at most one",
                    values.len()
                ),
            });
        }
        let mut value = values.pop();
        if self.options.await_entry_task {
            if let Some(Value::I32(ptr)) = value {
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] chic_main returned pointer {ptr}");
                }
                // Heuristic: async tasks are allocated from the heap and should sit well past the
                // initial stack/zero-page region. If the entry returns a tiny integer (e.g., an
                // exit code), treat it as a synchronous result instead of dereferencing it.
                if ptr != 0 && ptr >= 0x100 {
                    let mem_len = self.memory.len() as u32;
                    if ptr as u32 >= mem_len {
                        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                            eprintln!(
                                "[wasm-async] treating out-of-range task pointer {:#x} as synchronous exit (mem_len={:#x})",
                                ptr, mem_len
                            );
                        }
                        value = Some(Value::I32(0));
                    } else {
                        let awaited =
                            self.await_future_blocking(ptr as u32, self.options.async_result_len)?;
                        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                            eprintln!("[wasm-async] awaited task {ptr:#x} -> {awaited}");
                        }
                        value = Some(Value::I32(awaited));
                    }
                }
            }
        }
        if self.options.capture_stdout {
            trace.stdout = self.host_stdout.clone();
        }
        if self.options.capture_stderr {
            trace.stderr = self.host_stderr.clone();
        }
        let flags = read_flags();
        if std::env::var_os("CHIC_DEBUG_WASM_DEMOTE").is_some() {
            eprintln!("[wasm-demote] final flags {:?}", flags);
        }
        trace.float_flags = flags;
        if let Some(mode) = self.options.rounding_mode {
            set_rounding_mode(prev_rounding);
            trace.rounding_mode = mode;
        } else {
            trace.rounding_mode = prev_rounding;
        }
        Ok((value, trace))
    }

    #[cfg(test)]
    pub fn call(
        &mut self,
        func_index: u32,
        args: &[Value],
    ) -> Result<Option<WasmValue>, WasmExecutionError> {
        let (value, _) = self.call_with_trace(func_index, args)?;
        Ok(value.map(Into::into))
    }

    fn read_c_string(&self, ptr: u32, label: &str) -> Result<String, WasmExecutionError> {
        let start = ptr as usize;
        if start >= self.memory.len() {
            return Err(WasmExecutionError {
                message: format!("{label} points outside linear memory"),
            });
        }
        let mut bytes = Vec::new();
        let mut idx = start;
        let mut terminated = false;
        while idx < self.memory.len() && bytes.len() < 8192 {
            let byte = self.memory[idx];
            if byte == 0 {
                terminated = true;
                break;
            }
            bytes.push(byte);
            idx += 1;
        }
        if !terminated {
            return Err(WasmExecutionError {
                message: format!("{label} missing null terminator"),
            });
        }
        String::from_utf8(bytes).map_err(|_| WasmExecutionError {
            message: format!("{label} contains invalid UTF-8"),
        })
    }

    fn allocate_file_handle(&mut self) -> i32 {
        let handle = self.next_file_handle;
        self.next_file_handle = self.next_file_handle.wrapping_add(1);
        handle
    }

    fn allocate_socket_handle(&mut self) -> i32 {
        let handle = self.next_socket_handle;
        self.next_socket_handle = self.next_socket_handle.wrapping_add(1);
        handle
    }

    pub(super) fn allocate_thread_id(&mut self) -> u32 {
        let handle = self.next_thread_id;
        self.next_thread_id = self.next_thread_id.wrapping_add(1);
        handle
    }

    pub(super) fn host_fopen(
        &mut self,
        path_ptr: u32,
        mode_ptr: u32,
    ) -> Result<i32, WasmExecutionError> {
        let path = self.read_c_string(path_ptr, "env.fopen path")?;
        let mode = self.read_c_string(mode_ptr, "env.fopen mode")?;
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(open) = &hooks.fopen {
                return Ok(open(&path, &mode));
            }
        }

        let mut options = OpenOptions::new();
        let mut readable = false;
        let mut writable = false;
        let mut append = false;
        if mode.starts_with('r') {
            options.read(true);
            readable = true;
        } else if mode.starts_with('w') {
            options.write(true).create(true).truncate(true);
            writable = true;
        } else if mode.starts_with('a') {
            options.write(true).create(true).append(true);
            writable = true;
            append = true;
        } else {
            return Ok(0);
        }
        if mode.contains('+') {
            options.read(true).write(true);
            readable = true;
            writable = true;
            if !append && mode.starts_with('a') {
                options.append(true);
            }
        }

        match options.open(&path) {
            Ok(file) => {
                let handle = self.allocate_file_handle();
                self.host_files.insert(
                    handle,
                    HostFile {
                        file,
                        readable,
                        writable,
                    },
                );
                Ok(handle)
            }
            Err(_) => Ok(0),
        }
    }

    pub(super) fn host_fread(
        &mut self,
        stream: i32,
        ptr: u32,
        size: u32,
        count: u32,
    ) -> Result<i32, WasmExecutionError> {
        if size == 0 || count == 0 {
            return Ok(0);
        }
        let total = size.checked_mul(count).ok_or_else(|| WasmExecutionError {
            message: "env.fread length overflow".into(),
        })?;
        let mut buffer = vec![0u8; total as usize];
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(read) = &hooks.fread {
                let result = match read(stream, &mut buffer) {
                    Ok(bytes) => bytes,
                    Err(code) => return Ok(code),
                };
                let items = (result / size as usize) as i32;
                let to_store = result.min(buffer.len());
                if to_store > 0 {
                    self.store_bytes(ptr, 0, &buffer[..to_store])?;
                }
                return Ok(items);
            }
        }
        let Some(file) = self.host_files.get_mut(&stream) else {
            return Err(WasmExecutionError {
                message: format!("env.fread received unknown stream {stream}"),
            });
        };
        if !file.readable {
            return Ok(0);
        }
        let read = file.file.read(&mut buffer).unwrap_or(0);
        if read > 0 {
            self.store_bytes(ptr, 0, &buffer[..read])?;
        }
        let items = read / size as usize;
        Ok(i32::try_from(items).unwrap_or(i32::MAX))
    }

    pub(super) fn host_fwrite(
        &mut self,
        stream: i32,
        ptr: u32,
        size: u32,
        count: u32,
    ) -> Result<i32, WasmExecutionError> {
        if size == 0 || count == 0 {
            return Ok(0);
        }
        let total = size.checked_mul(count).ok_or_else(|| WasmExecutionError {
            message: "env.fwrite length overflow".into(),
        })?;
        let data = self.read_bytes(ptr, total)?;
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(write) = &hooks.fwrite {
                let written = match write(stream, &data) {
                    Ok(bytes) => bytes,
                    Err(code) => return Ok(code),
                };
                let items = written / size as usize;
                return Ok(i32::try_from(items).unwrap_or(i32::MAX));
            }
        }
        let Some(file) = self.host_files.get_mut(&stream) else {
            return Err(WasmExecutionError {
                message: format!("env.fwrite received unknown stream {stream}"),
            });
        };
        if !file.writable {
            return Ok(0);
        }
        let written = file.file.write(&data).unwrap_or(0);
        let items = written / size as usize;
        Ok(i32::try_from(items).unwrap_or(i32::MAX))
    }

    pub(super) fn host_fflush(&mut self, stream: i32) -> Result<i32, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(flush) = &hooks.fflush {
                return Ok(flush(stream));
            }
        }
        let Some(file) = self.host_files.get_mut(&stream) else {
            return Err(WasmExecutionError {
                message: format!("env.fflush received unknown stream {stream}"),
            });
        };
        match file.file.flush() {
            Ok(_) => Ok(0),
            Err(_) => Ok(-1),
        }
    }

    pub(super) fn host_fclose(&mut self, stream: i32) -> Result<i32, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(close) = &hooks.fclose {
                return Ok(close(stream));
            }
        }
        if self.host_files.remove(&stream).is_some() {
            return Ok(0);
        }
        Err(WasmExecutionError {
            message: format!("env.fclose received unknown stream {stream}"),
        })
    }

    pub(super) fn host_fileno(&mut self, stream: i32) -> Result<i32, WasmExecutionError> {
        if self.host_files.contains_key(&stream) {
            return Ok(stream);
        }
        Err(WasmExecutionError {
            message: format!("env.fileno received unknown stream {stream}"),
        })
    }

    pub(super) fn host_ftell(&mut self, stream: i32) -> Result<i64, WasmExecutionError> {
        let Some(file) = self.host_files.get_mut(&stream) else {
            return Err(WasmExecutionError {
                message: format!("env.ftell received unknown stream {stream}"),
            });
        };
        file.file
            .seek(SeekFrom::Current(0))
            .map(|pos| pos as i64)
            .map_err(|err| WasmExecutionError {
                message: format!("env.ftell failed: {err}"),
            })
    }

    pub(super) fn host_ftruncate(
        &mut self,
        stream: i32,
        length: i64,
    ) -> Result<i32, WasmExecutionError> {
        let Some(file) = self.host_files.get_mut(&stream) else {
            return Err(WasmExecutionError {
                message: format!("env.ftruncate received unknown stream {stream}"),
            });
        };
        let len = length.max(0) as u64;
        file.file
            .set_len(len)
            .map(|_| 0)
            .map_err(|err| WasmExecutionError {
                message: format!("env.ftruncate failed: {err}"),
            })
    }

    pub(super) fn host_clock_gettime(
        &mut self,
        clock_id: i32,
        ts_ptr: u32,
    ) -> Result<i32, WasmExecutionError> {
        if clock_id != 1 {
            return Ok(-1);
        }
        let nanos = self.host_monotonic_nanos()?;
        let secs = nanos / 1_000_000_000;
        let remainder = nanos % 1_000_000_000;
        self.store_i64(ts_ptr, 0, secs)?;
        self.store_i64(ts_ptr, 8, remainder)?;
        Ok(0)
    }

    pub(super) fn host_nanosleep(
        &mut self,
        req_ptr: u32,
        rem_ptr: Option<u32>,
    ) -> Result<i32, WasmExecutionError> {
        let secs = self.load_i64(req_ptr, 0)?;
        let nanos = self.load_i64(req_ptr, 8)?;
        let secs = secs.max(0) as u64;
        let nanos = nanos.max(0) as u64;
        let duration =
            Duration::from_secs(secs).saturating_add(Duration::from_nanos(nanos.min(999_999_999)));
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(sleep) = &hooks.sleep_millis {
                let millis = duration.as_millis().min(u64::from(u32::MAX) as u128) as u64;
                let code = sleep(millis);
                if let Some(rem) = rem_ptr {
                    let _ = self.store_i64(rem, 0, 0);
                    let _ = self.store_i64(rem, 8, 0);
                }
                return Ok(code);
            }
        }
        std::thread::sleep(duration);
        if let Some(rem) = rem_ptr {
            self.store_i64(rem, 0, 0)?;
            self.store_i64(rem, 8, 0)?;
        }
        Ok(0)
    }

    pub(super) fn host_socket(
        &mut self,
        domain: i32,
        typ: i32,
        proto: i32,
    ) -> Result<i32, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(socket) = &hooks.socket {
                let fd = match socket(domain, typ, proto) {
                    Ok(fd) => fd,
                    Err(code) => return Ok(code),
                };
                return Ok(fd);
            }
        }
        let handle = self.allocate_socket_handle();
        self.host_sockets.insert(
            handle,
            HostSocket::Pending {
                _domain: domain,
                _typ: typ,
                _proto: proto,
            },
        );
        Ok(handle)
    }

    fn decode_sockaddr_in(&self, ptr: u32, len: u32) -> Result<SocketAddr, WasmExecutionError> {
        if len < 8 {
            return Err(WasmExecutionError {
                message: "env.connect received too-short sockaddr".into(),
            });
        }
        let data = self.read_bytes(ptr, len)?;
        let family = u16::from_le_bytes([data[0], data[1]]);
        if family != 2 {
            return Err(WasmExecutionError {
                message: format!("env.connect received unsupported family {family}"),
            });
        }
        let port_net = u16::from_le_bytes([data[2], data[3]]);
        let addr_net = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let port = u16::from_be(port_net);
        let ip = Ipv4Addr::from(u32::from_be(addr_net));
        Ok(SocketAddr::from((ip, port)))
    }

    pub(super) fn host_connect(
        &mut self,
        fd: i32,
        addr_ptr: u32,
        addr_len: u32,
    ) -> Result<i32, WasmExecutionError> {
        let sockaddr = self.decode_sockaddr_in(addr_ptr, addr_len)?;
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(connect) = &hooks.connect {
                return Ok(connect(
                    fd,
                    match sockaddr {
                        SocketAddr::V4(v4) => u32::from(*v4.ip()),
                        _ => 0,
                    },
                    sockaddr.port(),
                ));
            }
        }
        let Some(socket) = self.host_sockets.get_mut(&fd) else {
            return Err(WasmExecutionError {
                message: format!("env.connect received unknown fd {fd}"),
            });
        };
        match socket {
            HostSocket::Pending { .. } => match TcpStream::connect(sockaddr) {
                Ok(stream) => {
                    *socket = HostSocket::Tcp(stream);
                    Ok(0)
                }
                Err(_) => Ok(-1),
            },
            HostSocket::Tcp(_) => Ok(0),
        }
    }

    pub(super) fn host_send(
        &mut self,
        fd: i32,
        ptr: u32,
        len: u32,
    ) -> Result<i32, WasmExecutionError> {
        let data = self.read_bytes(ptr, len)?;
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(send) = &hooks.send {
                let written = match send(fd, &data) {
                    Ok(bytes) => bytes,
                    Err(code) => return Ok(code),
                };
                return Ok(i32::try_from(written).unwrap_or(i32::MAX));
            }
        }
        let Some(HostSocket::Tcp(stream)) = self.host_sockets.get_mut(&fd) else {
            return Err(WasmExecutionError {
                message: format!("env.send received unknown fd {fd}"),
            });
        };
        match stream.write(&data) {
            Ok(written) => Ok(i32::try_from(written).unwrap_or(i32::MAX)),
            Err(_) => Ok(-1),
        }
    }

    pub(super) fn host_recv(
        &mut self,
        fd: i32,
        ptr: u32,
        len: u32,
    ) -> Result<i32, WasmExecutionError> {
        if len == 0 {
            return Ok(0);
        }
        let mut buffer = vec![0u8; len as usize];
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(recv) = &hooks.recv {
                let read = match recv(fd, &mut buffer) {
                    Ok(bytes) => bytes,
                    Err(code) => return Ok(code),
                };
                if read > 0 {
                    self.store_bytes(ptr, 0, &buffer[..read.min(buffer.len())])?;
                }
                return Ok(i32::try_from(read).unwrap_or(i32::MAX));
            }
        }
        let Some(HostSocket::Tcp(stream)) = self.host_sockets.get_mut(&fd) else {
            return Err(WasmExecutionError {
                message: format!("env.recv received unknown fd {fd}"),
            });
        };
        match stream.read(&mut buffer) {
            Ok(read) => {
                if read > 0 {
                    self.store_bytes(ptr, 0, &buffer[..read])?;
                }
                Ok(i32::try_from(read).unwrap_or(i32::MAX))
            }
            Err(_) => Ok(-1),
        }
    }

    pub(super) fn host_shutdown_socket(
        &mut self,
        fd: i32,
        how: i32,
    ) -> Result<i32, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(shutdown) = &hooks.shutdown {
                return Ok(shutdown(fd, how));
            }
        }
        let Some(HostSocket::Tcp(stream)) = self.host_sockets.get_mut(&fd) else {
            return Err(WasmExecutionError {
                message: format!("env.shutdown received unknown fd {fd}"),
            });
        };
        let how = match how {
            1 => Shutdown::Write,
            0 => Shutdown::Read,
            _ => Shutdown::Both,
        };
        match stream.shutdown(how) {
            Ok(_) => Ok(0),
            Err(_) => Ok(-1),
        }
    }

    pub(super) fn host_close_socket(&mut self, fd: i32) -> Result<i32, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(close) = &hooks.close_socket {
                return Ok(close(fd));
            }
        }
        if self.host_sockets.remove(&fd).is_some() {
            return Ok(0);
        }
        Err(WasmExecutionError {
            message: format!("env.close received unknown fd {fd}"),
        })
    }

    pub(super) fn host_htons(&self, value: u16) -> i32 {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(htons) = &hooks.htons {
                return i32::from(htons(value));
            }
        }
        i32::from(value.to_be())
    }

    pub(super) fn host_inet_pton(
        &mut self,
        af: i32,
        src_ptr: u32,
        dst_ptr: u32,
    ) -> Result<i32, WasmExecutionError> {
        if af != 2 {
            return Ok(-1);
        }
        let text = self.read_c_string(src_ptr, "env.inet_pton src")?;
        let octets = if let Some(hooks) = &self.options.io_hooks {
            if let Some(parse) = &hooks.inet_pton {
                match parse(af, &text) {
                    Ok(bytes) => bytes,
                    Err(code) => return Ok(code),
                }
            } else {
                match text.parse::<Ipv4Addr>() {
                    Ok(addr) => addr.octets(),
                    Err(_) => return Ok(0),
                }
            }
        } else {
            match text.parse::<Ipv4Addr>() {
                Ok(addr) => addr.octets(),
                Err(_) => return Ok(0),
            }
        };
        self.store_bytes(dst_ptr, 0, &octets)?;
        Ok(1)
    }

    pub(super) fn host_write(
        &mut self,
        fd: i32,
        ptr: u32,
        len: u32,
    ) -> Result<i32, WasmExecutionError> {
        if len == 0 {
            return Ok(0);
        }
        let data = self.read_bytes(ptr, len)?;
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(write) = &hooks.write {
                let mut written: usize = 0;
                let result = write(fd, data.as_ptr(), data.len(), &mut written);
                return Ok(result);
            }
        }
        match fd {
            1 => {
                if self.options.capture_stdout {
                    self.host_stdout.extend_from_slice(&data);
                }
                Ok(i32::try_from(len).unwrap_or(i32::MAX))
            }
            2 => {
                if self.options.capture_stderr {
                    self.host_stderr.extend_from_slice(&data);
                }
                Ok(i32::try_from(len).unwrap_or(i32::MAX))
            }
            _ => Err(WasmExecutionError {
                message: format!("env.write received unsupported fd {fd}"),
            }),
        }
    }

    pub(super) fn host_read(
        &mut self,
        fd: i32,
        ptr: u32,
        len: u32,
    ) -> Result<i32, WasmExecutionError> {
        if len == 0 {
            return Ok(0);
        }
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(read) = &hooks.read {
                let mut buf = vec![0u8; len as usize];
                let mut read_bytes: usize = 0;
                let result = read(fd, buf.as_mut_ptr(), buf.len(), &mut read_bytes);
                let slice = &buf[..read_bytes.min(buf.len())];
                self.store_bytes(ptr, 0, slice)?;
                return Ok(result);
            }
        }

        if fd != 0 {
            return Err(WasmExecutionError {
                message: format!("env.read received unsupported fd {fd}"),
            });
        }
        if self.host_stdin_cursor >= self.host_stdin.len() {
            return Ok(0);
        }
        let available = (self.host_stdin.len() - self.host_stdin_cursor) as u32;
        let to_copy = available.min(len) as usize;
        let slice =
            self.host_stdin[self.host_stdin_cursor..self.host_stdin_cursor + to_copy].to_vec();
        self.store_bytes(ptr, 0, &slice)?;
        self.host_stdin_cursor += to_copy;
        Ok(i32::try_from(to_copy).unwrap_or(i32::MAX))
    }

    pub(super) fn host_isatty(&self, fd: i32) -> i32 {
        if let Some(hooks) = &self.options.io_hooks {
            if hooks.read.is_some() || hooks.write.is_some() || hooks.flush.is_some() {
                // Assume host provided IO implies terminal awareness; default to false when unknown.
                return 0;
            }
        }
        match fd {
            0 => self.host_terminals.0 as i32,
            1 => self.host_terminals.1 as i32,
            2 => self.host_terminals.2 as i32,
            _ => 0,
        }
    }

    pub(super) fn host_monotonic_nanos(&self) -> Result<i64, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(clock) = &hooks.monotonic_nanos {
                return Ok(clock());
            }
        }
        let nanos = self.start_time.elapsed().as_nanos();
        Ok(nanos.min(i64::MAX as u128) as i64)
    }

    pub(super) fn host_sleep_millis(&self, millis: u32) -> Result<i32, WasmExecutionError> {
        if let Some(hooks) = &self.options.io_hooks {
            if let Some(sleep) = &hooks.sleep_millis {
                return Ok(sleep(millis as u64));
            }
        }
        std::thread::sleep(Duration::from_millis(millis as u64));
        Ok(0)
    }

    #[cfg(test)]
    pub(crate) fn expose_string_repr_for_tests(&self, ptr: u32) -> WasmStringRepr {
        self.read_string_repr(ptr).expect("string repr")
    }

    #[cfg(test)]
    pub(crate) fn expose_string_data_ptr_for_tests(&self, base: u32) -> u32 {
        let repr = self.read_string_repr(base).expect("string repr");
        if repr.len == 0 {
            return STRING_EMPTY_PTR;
        }
        let inline = repr.len <= STRING_INLINE_CAPACITY
            && ((repr.cap & STRING_INLINE_TAG) != 0 || repr.ptr == 0);
        if inline {
            base + self.async_layout.ptr_size * 3
        } else {
            repr.ptr
        }
    }

    #[cfg(test)]
    pub(crate) fn test_memory_mut(&mut self) -> &mut Vec<u8> {
        &mut self.memory
    }
}

pub(super) const WASM_PAGE_SIZE: usize = 65536;
pub(super) const LINEAR_MEMORY_HEAP_BASE: u32 = 0x1000;
pub(super) const STACK_BASE_RED_ZONE: usize = 4096;
const HOST_FILE_BASE: i32 = 0x1000_0000;
const HOST_SOCKET_BASE: i32 = 0x2000_0000;

#[derive(Clone, Copy)]
pub(super) struct RuntimeGlobal {
    pub(super) ty: ValueType,
    pub(super) mutable: bool,
    pub(super) value: Value,
}
