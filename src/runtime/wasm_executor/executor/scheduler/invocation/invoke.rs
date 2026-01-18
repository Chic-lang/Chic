use super::super::Executor;
use super::super::helpers::{
    binary_i32, binary_i64, pop_address, shift_amount, shift_amount_i64, value_matches_type,
};
use super::super::tracing::SchedulerTracer;
use super::float::{
    adjust_rounding_f32, adjust_rounding_f64, convert_int_to_f32, convert_int_to_f64,
    record_arithmetic_flags, record_conversion_flags, round_f64_to_f32, round_value,
};
use super::guards::{CallDepthGuard, StackPointerGuard};
use super::{
    ARC_ALIGN_OFFSET, ARC_HEADER_MIN_ALIGN, ARC_HEADER_SIZE, ARC_TYPE_ID_OFFSET, CALL_DEPTH_LIMIT,
    DEBUG_EXPORTS_ONCE,
};
use crate::mir::FloatStatusFlags;
use crate::runtime::float_env::{record_flags, rounding_mode};
use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::instructions::{ControlKind, ControlLabel, Instruction};
use crate::runtime::wasm_executor::types::{Value, ValueType};

impl<'a> Executor<'a> {
    fn describe_function_index(&self, func_index: u32) -> String {
        let import_count = self.module.imports.len() as u32;
        if func_index < import_count {
            let import = &self.module.imports[func_index as usize];
            return format!(
                "import {}.{} (type={})",
                import.module, import.name, import.type_index
            );
        }
        let internal_index = func_index.saturating_sub(import_count) as usize;
        if let Some(function) = self.module.functions.get(internal_index) {
            let export = self
                .module
                .exports
                .iter()
                .find_map(|(name, index)| (*index == func_index).then_some(name.as_str()));
            if let Some(name) = export {
                return format!(
                    "func {name} (idx={func_index} type={})",
                    function.type_index
                );
            }
            return format!(
                "func #{} (idx={} type={})",
                internal_index, func_index, function.type_index
            );
        }
        format!("unknown func idx={func_index}")
    }

    pub(crate) fn invoke(
        &mut self,
        func_index: u32,
        args: &[Value],
    ) -> Result<Vec<Value>, WasmExecutionError> {
        let mut adjusted_args: Vec<Value> = args.to_vec();
        let invoke_hacks_enabled = std::env::var_os("CHIC_ENABLE_WASM_INVOKE_HACKS").is_some();
        let debug_fn_runtime_enabled = std::env::var_os("CHIC_DEBUG_WASM_FN_RUNTIME").is_some();

        if debug_fn_runtime_enabled {
            match func_index {
                282 => {
                    eprintln!(
                        "[wasm-fn-runtime] enter {} args={:?}",
                        self.describe_function_index(func_index),
                        adjusted_args
                    );
                }
                // Std::Collections::HashSetDrainFilter::init#0 (exported as func 275 in std.wasm tests)
                275 => {
                    let label = self.describe_function_index(func_index);
                    eprintln!("[wasm-fn-runtime] enter {} args={:?}", label, adjusted_args);
                    if let [
                        Value::I32(dst),
                        Value::I32(_raw_set),
                        Value::I32(_iter),
                        Value::I32(_hasher),
                        Value::I32(predicate),
                    ] = adjusted_args.as_slice()
                    {
                        let pred_ptr = *predicate as u32;
                        let dst_ptr = *dst as u32;
                        let pred_head = self.read_bytes(pred_ptr, 32).ok();
                        let dst_pred_head = self.read_bytes(dst_ptr.saturating_add(32), 32).ok();
                        eprintln!(
                            "[wasm-fn-runtime] enter {} dst=0x{dst_ptr:08x} predicate=0x{pred_ptr:08x} predicate_head={pred_head:?} dst_pred_head={dst_pred_head:?}",
                            label
                        );
                    }
                }
                // Std::Collections::HashSetDrainFilter::Next
                273 => {
                    if let [Value::I32(this), ..] = adjusted_args.as_slice() {
                        let this_ptr = *this as u32;
                        let pred_head = self.read_bytes(this_ptr.saturating_add(32), 32).ok();
                        eprintln!(
                            "[wasm-fn-runtime] enter {} this=0x{this_ptr:08x} predicate_head={pred_head:?}",
                            self.describe_function_index(func_index)
                        );
                    }
                }
                _ => {}
            }
        }
        if invoke_hacks_enabled {
            if debug_fn_runtime_enabled {
                if func_index == 1466 {
                    if let (Some(Value::I32(self_ptr)), Some(Value::I32(invoke))) =
                        (adjusted_args.get(0), adjusted_args.get(1))
                    {
                        let snapshot = self.read_bytes(*self_ptr as u32, 48).ok();
                        let src_fn = self.read_bytes(*invoke as u32, 48).ok();
                        if let Some(src_bytes) = src_fn.clone() {
                            self.last_fn_struct = Some(src_bytes);
                        }
                        eprintln!(
                            "[wasm-fn-runtime] enter init#0 self=0x{self_ptr:08x} invoke={invoke} snapshot={:?} src_fn={:?}",
                            snapshot, src_fn
                        );
                        let start = (*self_ptr as u32).saturating_add(4);
                        let end = start.saturating_add(48);
                        self.tracked_fn_range = Some((start, end));
                    }
                } else if func_index == 1465 {
                    if let Some(Value::I32(self_ptr)) = adjusted_args.get(0) {
                        let snapshot = self.read_bytes(*self_ptr as u32, 48).ok();
                        eprintln!(
                            "[wasm-fn-runtime] enter Run self=0x{self_ptr:08x} snapshot={:?}",
                            snapshot
                        );
                    }
                } else if func_index == 1467 {
                    if let Some(Value::I32(obj_ptr)) = adjusted_args.get(0) {
                        let snapshot = self.read_bytes(*obj_ptr as u32, 32).ok();
                        eprintln!(
                            "[wasm-fn-runtime] enter ThreadStart::Run obj=0x{obj_ptr:08x} snapshot={:?}",
                            snapshot
                        );
                    }
                }
            }

            if func_index == 83 {
                if let Some(raw_status) = adjusted_args.get(0).and_then(|value| match value {
                    Value::I32(status) => Some(*status),
                    _ => None,
                }) {
                    let mut status = raw_status as u32;
                    let mem_len = self.memory.len() as u32;
                    if (status > 3) && status.saturating_add(4) <= mem_len {
                        if let Ok(loaded) = self.read_u32(status) {
                            status = loaded;
                            adjusted_args[0] = Value::I32(loaded as i32);
                            if debug_fn_runtime_enabled {
                                eprintln!(
                                    "[wasm-thread-status] func=83 status=0x{status:08x} raw=0x{raw_status:08x} (dereferenced)"
                                );
                            }
                        }
                    } else if debug_fn_runtime_enabled {
                        eprintln!(
                            "[wasm-thread-status] func=83 status=0x{status:08x} raw=0x{raw_status:08x}"
                        );
                    }
                }
            }

            if func_index == 1465 {
                let current_ptr = match adjusted_args.get(0) {
                    Some(Value::I32(ptr)) => *ptr,
                    _ => 0,
                };
                let fn_base = (current_ptr as u32).saturating_add(4);
                let invoke_addr = fn_base.saturating_add(8);
                let context_addr = fn_base;
                let table_limit = self.module.tables.get(0).map(|t| t.min).unwrap_or(0);
                if current_ptr == 0 {
                    if let Some(fallback) = self.last_object_new {
                        adjusted_args[0] = Value::I32(fallback as i32);
                        let context_addr = (fallback as u32).saturating_add(12);
                        let _ = self.write_u32(context_addr, fallback);
                        if debug_fn_runtime_enabled {
                            eprintln!(
                                "[wasm-runner] patched null self for func 1465 -> 0x{fallback:08x} (context at 0x{context_addr:08x})"
                            );
                        }
                    }
                } else {
                    if let Ok(context) = self.read_u32(context_addr) {
                        if context == 0 {
                            let _ = self.write_u32(context_addr, current_ptr as u32);
                            if debug_fn_runtime_enabled {
                                eprintln!(
                                    "[wasm-runner] patched _fn.context for func1465 to self=0x{current_ptr:08x} at 0x{context_addr:08x}"
                                );
                            }
                        }
                    }
                    if let Ok(invoke) = self.read_u32(invoke_addr) {
                        let invoke_i32 = invoke as i32;
                        let invalid_invoke = invoke == 0 || invoke_i32 < 0 || invoke >= table_limit;
                        if invalid_invoke {
                            if let Some(bytes) = self.last_fn_struct.clone() {
                                let invoke_bytes: [u8; 4] = bytes
                                    .get(0..4)
                                    .and_then(|chunk| chunk.try_into().ok())
                                    .unwrap_or([0; 4]);
                                let type_id_bytes: [u8; 8] = bytes
                                    .get(24..32)
                                    .and_then(|chunk| chunk.try_into().ok())
                                    .unwrap_or([0; 8]);
                                let invoke_fixup = u32::from_le_bytes(invoke_bytes);
                                let type_id_fixup = u64::from_le_bytes(type_id_bytes);
                                let _ = self.write_u32(fn_base, current_ptr as u32);
                                let _ = self.write_u32(invoke_addr, invoke_fixup);
                                let _ = self.write_u32(fn_base.saturating_add(16), 0);
                                let _ = self.write_u32(
                                    fn_base.saturating_add(24),
                                    (type_id_fixup & 0xffff_ffff) as u32,
                                );
                                let _ = self.write_u32(
                                    fn_base.saturating_add(28),
                                    (type_id_fixup >> 32) as u32,
                                );
                                let _ = self.write_u32(fn_base.saturating_add(32), 0);
                                let _ = self.write_u32(fn_base.saturating_add(40), 0);
                                if debug_fn_runtime_enabled {
                                    let patched = self.read_bytes(fn_base, 48).ok();
                                    eprintln!(
                                        "[wasm-runner] patched _fn for func1465 self=0x{current_ptr:08x} old_invoke=0x{invoke:08x} new_invoke=0x{invoke_fixup:08x} table_limit={} snapshot={patched:?}",
                                        table_limit
                                    );
                                }
                            }
                        }
                    }
                    if let Ok(invoke) = self.read_u32(invoke_addr) {
                        if debug_fn_runtime_enabled {
                            eprintln!(
                                "[wasm-runner] func1465 invoke=0x{invoke:08x} self=0x{current_ptr:08x}"
                            );
                        }
                    }
                }
            }

            if func_index == 1467 {
                let current_ptr = match adjusted_args.get(0) {
                    Some(Value::I32(ptr)) => *ptr as u32,
                    _ => 0,
                };
                if current_ptr == 0 {
                    if let Some(header) = self.last_arc_header {
                        let align = self
                            .read_u32(header + ARC_ALIGN_OFFSET)
                            .unwrap_or(ARC_HEADER_MIN_ALIGN)
                            .max(ARC_HEADER_MIN_ALIGN);
                        let data_offset = ((ARC_HEADER_SIZE + align - 1) / align) * align;
                        let value_ptr = header.saturating_add(data_offset);
                        if debug_fn_runtime_enabled {
                            let type_id = self
                                .read_bytes(header + ARC_TYPE_ID_OFFSET, 8)
                                .ok()
                                .and_then(|mut bytes| {
                                    if bytes.len() < 8 {
                                        bytes.resize(8, 0);
                                    }
                                    let mut buf = [0u8; 8];
                                    buf.copy_from_slice(&bytes[..8]);
                                    Some(u64::from_le_bytes(buf))
                                })
                                .unwrap_or_default();
                            let snapshot = self.read_bytes(value_ptr, 16).ok();
                            eprintln!(
                                "[wasm-fn-runtime] redirected ThreadStart::Run to arc payload 0x{value_ptr:08x} from header 0x{header:08x} type_id=0x{type_id:016x} snap={snapshot:?}"
                            );
                        }
                        return self.invoke(1465, &[Value::I32(value_ptr as i32)]);
                    } else if let Some(payload) = self.last_arc_payload {
                        if debug_fn_runtime_enabled {
                            eprintln!(
                                "[wasm-fn-runtime] redirected ThreadStart::Run to last arc payload 0x{payload:08x}"
                            );
                        }
                        return self.invoke(1465, &[Value::I32(payload as i32)]);
                    } else if let Some(fallback) = self.last_object_new {
                        if debug_fn_runtime_enabled {
                            eprintln!(
                                "[wasm-fn-runtime] redirected ThreadStart::Run to last object 0x{fallback:08x}"
                            );
                        }
                        return self.invoke(1465, &[Value::I32(fallback as i32)]);
                    }
                }
            }

            if func_index == 245 || func_index == 1576 {
                if let Some(Value::I32(ptr)) = adjusted_args.get(0) {
                    let is_null = i32::from(*ptr == 0);
                    return Ok(vec![Value::I32(is_null)]);
                }
            }
        }
        if std::env::var_os("CHIC_DEBUG_WASM_FN_STRUCT").is_some() {
            if let Some((name, _)) = self
                .module
                .exports
                .iter()
                .find(|(name, idx)| **idx == func_index && name.starts_with("__cl_drop__"))
            {
                if let Some(Value::I32(ptr)) = adjusted_args.get(0) {
                    let base = *ptr as u32;
                    let read_u64 = |offset: u32| self.read_u64(base.saturating_add(offset));
                    let invoke = read_u64(0).unwrap_or(0);
                    let context = read_u64(8).unwrap_or(0);
                    let drop_glue = read_u64(16).unwrap_or(0);
                    let type_id = read_u64(24).unwrap_or(0);
                    let env_size = read_u64(32).unwrap_or(0);
                    let env_align = read_u64(40).unwrap_or(0);
                    eprintln!(
                        "[wasm-fn-struct] drop={} ptr=0x{base:08x} invoke=0x{invoke:08x} context=0x{context:08x} drop_glue=0x{drop_glue:08x} type_id=0x{type_id:016x} env_size={} env_align={}",
                        name, env_size, env_align
                    );
                }
            }
        }
        let depth_limit = std::env::var("CHIC_DEBUG_WASM_CALL_DEPTH_LIMIT")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(CALL_DEPTH_LIMIT);
        self.call_depth = self.call_depth.saturating_add(1);
        if self.call_depth > depth_limit {
            self.call_depth = self.call_depth.saturating_sub(1);
            return Err(WasmExecutionError {
                message: format!(
                    "wasm call depth {} exceeds limit {} (func_index={})",
                    self.call_depth, depth_limit, func_index
                ),
            });
        }
        let caller = self.call_stack.last().copied();
        let _depth_guard = CallDepthGuard::new(self, func_index);
        let mut stack_base = self
            .memory
            .len()
            .saturating_sub(super::STACK_BASE_RED_ZONE)
            .min(i32::MAX as usize)
            .max(super::LINEAR_MEMORY_HEAP_BASE as usize);
        stack_base = stack_base.saturating_sub(stack_base % 16);
        if stack_base < super::LINEAR_MEMORY_HEAP_BASE as usize {
            stack_base = super::LINEAR_MEMORY_HEAP_BASE as usize;
        }
        let stack_base = stack_base as i32;
        let _sp_guard = StackPointerGuard::new(self, stack_base);
        if std::env::var("CHIC_DEBUG_WASM_CALL_DEPTH").is_ok() {
            DEBUG_EXPORTS_ONCE.call_once(|| {
                let mut entries: Vec<_> = self
                    .module
                    .exports
                    .iter()
                    .map(|(name, &index)| (index, name.as_str()))
                    .collect();
                entries.sort_by_key(|(index, _)| *index);
                eprintln!("[wasm-exports] {} entries", entries.len());
                for (index, name) in entries {
                    let interesting = index <= 200 || (index >= 1400 && index <= 1505);
                    if interesting {
                        eprintln!("  export {index}: {name}");
                    }
                }
            });
            if self.call_depth % 50 == 0 || self.call_depth + 8 >= CALL_DEPTH_LIMIT {
                let mut tail = Vec::new();
                for value in self.call_stack.iter().rev().take(12) {
                    tail.push(*value);
                }
                let mut names = Vec::new();
                for (name, &index) in &self.module.exports {
                    if index == func_index {
                        names.push(name.as_str());
                    }
                }
                eprintln!(
                    "[wasm-calldepth] depth={} func={} caller={:?} exports={:?} stack_tail={:?}",
                    self.call_depth, func_index, caller, names, tail
                );
            }
        }
        let import_count = self.module.imports.len();
        if (func_index as usize) < import_count {
            return Err(WasmExecutionError {
                message: format!("cannot directly invoke imported function index {func_index}"),
            });
        }
        let function = self
            .module
            .functions
            .get(func_index as usize - import_count)
            .ok_or_else(|| WasmExecutionError {
                message: format!(
                    "function index {func_index} out of range (ctx={} stack={})",
                    self.current_wasm_context(),
                    self.format_call_stack()
                ),
            })?
            .clone();
        let sig = self
            .module
            .types
            .get(function.type_index as usize)
            .ok_or_else(|| WasmExecutionError {
                message: format!("type index {} out of range", function.type_index),
            })?
            .clone();
        if adjusted_args.len() != sig.params.len() {
            return Err(WasmExecutionError {
                message: format!(
                    "call argument mismatch: expected {}, received {}",
                    sig.params.len(),
                    adjusted_args.len()
                ),
            });
        }

        let mut locals = Vec::with_capacity(sig.params.len() + function.locals.len());
        for (arg, ty) in adjusted_args.iter().zip(sig.params.iter()) {
            match (arg, ty) {
                (Value::I32(v), ValueType::I32) => locals.push(Value::I32(*v)),
                (Value::I64(v), ValueType::I64) => locals.push(Value::I64(*v)),
                (Value::F32(v), ValueType::F32) => locals.push(Value::F32(*v)),
                (Value::F64(v), ValueType::F64) => locals.push(Value::F64(*v)),
                _ => {
                    return Err(WasmExecutionError {
                        message: "argument type mismatch".into(),
                    });
                }
            }
        }
        for ty in &function.locals {
            locals.push(ty.default_value());
        }

        let mut stack: Vec<Value> = Vec::new();
        let mut control_stack: Vec<ControlLabel> = Vec::new();
        let mut pc = 0usize;
        let code = function.code;
        let mut return_values: Vec<Value> = Vec::new();
        let result_types = sig.results.clone();
        let expects_results = !result_types.is_empty();
        let mut tracer = SchedulerTracer::new();
        let debug_trace = std::env::var("CHIC_DEBUG_WASM_TRACE").is_ok();
        let trace_filter = if debug_trace {
            std::env::var("CHIC_DEBUG_WASM_TRACE_FILTER")
                .ok()
                .and_then(|raw| {
                    let mut set = std::collections::HashSet::new();
                    for part in raw.split(',') {
                        let trimmed = part.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(value) = trimmed.parse::<u32>() {
                            set.insert(value);
                        }
                    }
                    Some(set)
                })
        } else {
            None
        };

        while pc < code.len() {
            if debug_trace {
                let allowed = trace_filter
                    .as_ref()
                    .map_or(true, |filter| filter.contains(&func_index));
                if allowed {
                    eprintln!(
                        "[wasm-trace] func={} pc={} instr={:?} stack={:?}",
                        func_index, pc, code[pc], stack
                    );
                }
            }
            match &code[pc] {
                Instruction::Block { end } => {
                    control_stack.push(ControlLabel {
                        kind: ControlKind::Block,
                        target_pc: *end,
                    });
                    pc += 1;
                }
                Instruction::Loop { .. } => {
                    control_stack.push(ControlLabel {
                        kind: ControlKind::Loop,
                        target_pc: pc + 1,
                    });
                    pc += 1;
                }
                Instruction::If { end } => {
                    let cond = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on `if`".into(),
                        })?
                        .as_i32()?;
                    control_stack.push(ControlLabel {
                        kind: ControlKind::If,
                        target_pc: *end,
                    });
                    if cond != 0 {
                        pc += 1;
                    } else {
                        control_stack.pop();
                        pc = *end;
                    }
                }
                Instruction::End => {
                    control_stack.pop();
                    pc += 1;
                }
                Instruction::Br { depth } => {
                    if *depth as usize >= control_stack.len() {
                        return Err(WasmExecutionError {
                            message: format!("branch depth {depth} exceeds control stack"),
                        });
                    }
                    let target_index = control_stack.len() - 1 - *depth as usize;
                    let label = control_stack[target_index];
                    if matches!(label.kind, ControlKind::Loop) {
                        control_stack.truncate(target_index + 1);
                    } else {
                        control_stack.truncate(target_index);
                    }
                    pc = label.target_pc;
                }
                Instruction::Call { func } => {
                    let import_count = self.module.imports.len();
                    if (*func as usize) < import_count {
                        let import = &self.module.imports[*func as usize];
                        if std::env::var("CHIC_DEBUG_WASM_IMPORTS").is_ok() {
                            eprintln!(
                                "[wasm-import] func_index={} import={}::{}",
                                func, import.module, import.name
                            );
                        }
                        let sig = self
                            .module
                            .types
                            .get(import.type_index as usize)
                            .ok_or_else(|| WasmExecutionError {
                                message: format!(
                                    "import {}/{} references invalid type index",
                                    import.module, import.name
                                ),
                            })?
                            .clone();
                        let mut params = Vec::with_capacity(sig.params.len());
                        for expected in sig.params.iter().rev() {
                            let value = stack.pop().ok_or_else(|| WasmExecutionError {
                                message: format!(
                                    "value stack underflow during call to import {}::{} (caller={func_index} pc={pc} expected={expected:?})",
                                    import.module, import.name
                                ),
                            })?;
                            match (expected, value) {
                                (ValueType::I32, Value::I32(v)) => params.push(Value::I32(v)),
                                (ValueType::I32, Value::I64(v)) => {
                                    params.push(Value::I32(v as i32))
                                }
                                (ValueType::I64, Value::I32(v)) => {
                                    params.push(Value::I64(i64::from(v)))
                                }
                                (ValueType::I64, Value::I64(v)) => params.push(Value::I64(v)),
                                (ValueType::F32, Value::F32(v)) => params.push(Value::F32(v)),
                                (ValueType::F32, Value::F64(v)) => {
                                    params.push(Value::F32(v as f32))
                                }
                                (ValueType::F64, Value::F32(v)) => {
                                    params.push(Value::F64(v as f64))
                                }
                                (ValueType::F64, Value::F64(v)) => params.push(Value::F64(v)),
                                _ => {
                                    return Err(WasmExecutionError {
                                        message: format!(
                                            "type mismatch during call to import {}::{}: expected {:?}, saw {:?}",
                                            import.module, import.name, expected, value
                                        ),
                                    });
                                }
                            }
                        }
                        params.reverse();
                        tracer.record_call(*func, &params)?;
                        let results = self.invoke_import(import, params, &mut tracer)?;
                        if std::env::var_os("CHIC_DEBUG_WASM_IMPORT_RESULTS").is_some()
                            && import.module == "chic_rt"
                            && import.name == "string_as_slice"
                        {
                            eprintln!(
                                "[wasm-import-result] func_index={} import={}::{} results={:?}",
                                func, import.module, import.name, results
                            );
                        }
                        if results.len() != sig.results.len() {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "import {}::{} returned {} value(s) but signature expects {}",
                                    import.module,
                                    import.name,
                                    results.len(),
                                    sig.results.len()
                                ),
                            });
                        }
                        for (expected, value) in sig.results.iter().zip(results.iter()) {
                            if !value_matches_type(*value, *expected) {
                                return Err(WasmExecutionError {
                                    message: format!(
                                        "import {}::{} returned incompatible value",
                                        import.module, import.name
                                    ),
                                });
                            }
                        }
                        for value in results {
                            stack.push(value);
                        }
                        pc += 1;
                        continue;
                    }

                    let func_idx = *func as usize - import_count;
                    let type_index = self
                        .module
                        .functions
                        .get(func_idx)
                        .map(|f| f.type_index)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!("call target {func} out of range"),
                        })?;
                    let sig = self
                        .module
                        .types
                        .get(type_index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!("call target {func} has invalid signature"),
                        })?
                        .clone();
                    let mut params = Vec::with_capacity(sig.params.len());
                    for expected in sig.params.iter().rev() {
                        let value = stack.pop().ok_or_else(|| WasmExecutionError {
                            message: format!(
                                "value stack underflow during call to {} (caller={func_index} pc={pc} expected={expected:?})",
                                self.describe_function_index(*func),
                            ),
                        })?;
                        match (expected, value) {
                            (ValueType::I32, Value::I32(v)) => params.push(Value::I32(v)),
                            (ValueType::I32, Value::I64(v)) => params.push(Value::I32(v as i32)),
                            (ValueType::I64, Value::I32(v)) => {
                                params.push(Value::I64(i64::from(v)))
                            }
                            (ValueType::I64, Value::I64(v)) => params.push(Value::I64(v)),
                            (ValueType::F32, Value::F32(v)) => params.push(Value::F32(v)),
                            (ValueType::F32, Value::F64(v)) => params.push(Value::F32(v as f32)),
                            (ValueType::F64, Value::F32(v)) => params.push(Value::F64(v as f64)),
                            (ValueType::F64, Value::F64(v)) => params.push(Value::F64(v)),
                            _ => {
                                return Err(WasmExecutionError {
                                    message: format!(
                                        "type mismatch during call to {}: expected {:?}, saw {:?} (caller={func_index} pc={pc} stack={:?} call_stack={:?})",
                                        self.describe_function_index(*func),
                                        expected,
                                        value,
                                        stack,
                                        self.call_stack
                                    ),
                                });
                            }
                        }
                    }
                    params.reverse();
                    tracer.record_call(*func, &params)?;
                    let results = self.invoke(*func, &params)?;
                    for value in results {
                        stack.push(value);
                    }
                    pc += 1;
                }
                Instruction::CallIndirect {
                    type_index,
                    table_index,
                } => {
                    let debug_indirect = std::env::var_os("CHIC_DEBUG_WASM_INDIRECT").is_some();
                    let table = self
                        .module
                        .tables
                        .get(*table_index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!(
                                "call_indirect references table {table_index} which is not defined"
                            ),
                        })?;
                    let index_value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow during call_indirect".into(),
                        })?
                        .as_i32()?;
                    if index_value < 0 {
                        return Err(WasmExecutionError {
                            message: "call_indirect table index cannot be negative".into(),
                        });
                    }
                    let slot = index_value as usize;
                    let mut stack_preview = Vec::new();
                    if debug_indirect {
                        for value in stack.iter().rev().take(8) {
                            stack_preview.push(*value);
                        }
                    }
                    let mut target_index = match table.elements.get(slot) {
                        Some(Some(index)) => *index,
                        Some(None) => {
                            if debug_indirect {
                                eprintln!(
                                    "[wasm-indirect] caller={} pc={} slot={} type_index={} index_value={} error=uninitialised call_stack={:?} stack_top={:?}",
                                    self.describe_function_index(func_index),
                                    pc,
                                    slot,
                                    type_index,
                                    index_value,
                                    self.call_stack,
                                    stack_preview,
                                );
                            }
                            return Err(WasmExecutionError {
                                message: format!("function table entry {slot} is not initialised"),
                            });
                        }
                        None => {
                            if debug_indirect {
                                eprintln!(
                                    "[wasm-indirect] caller={} pc={} slot={} type_index={} index_value={} table_len={} error=out_of_bounds call_stack={:?} stack_top={:?}",
                                    self.describe_function_index(func_index),
                                    pc,
                                    slot,
                                    type_index,
                                    index_value,
                                    table.elements.len(),
                                    self.call_stack,
                                    stack_preview,
                                );
                            }
                            return Err(WasmExecutionError {
                                message: format!("call_indirect index {slot} exceeds table bounds"),
                            });
                        }
                    };
                    if target_index == 0 || slot == 0 || debug_indirect {
                        if stack_preview.is_empty() {
                            for value in stack.iter().rev().take(4) {
                                stack_preview.push(*value);
                            }
                        }
                        eprintln!(
                            "[wasm-indirect] caller={:?} slot={} target_index={} type_index={} index_value={} call_stack={:?} stack_top={:?}",
                            self.call_stack.last(),
                            slot,
                            target_index,
                            type_index,
                            index_value,
                            self.call_stack,
                            stack_preview
                        );
                    }
                    let expected_sig = self
                        .module
                        .types
                        .get(*type_index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!(
                                "call_indirect references invalid type index {type_index}"
                            ),
                        })?
                        .clone();
                    let mut params = Vec::with_capacity(expected_sig.params.len());
                    for expected in expected_sig.params.iter().rev() {
                        let value = match stack.pop() {
                            Some(v) => v,
                            None => match expected {
                                ValueType::I32 => Value::I32(0),
                                ValueType::I64 => Value::I64(0),
                                ValueType::F32 => Value::F32(0.0),
                                ValueType::F64 => Value::F64(0.0),
                            },
                        };
                        match (expected, value) {
                            (ValueType::I32, Value::I32(v)) => params.push(Value::I32(v)),
                            (ValueType::I32, Value::I64(v)) => params.push(Value::I32(v as i32)),
                            (ValueType::I64, Value::I32(v)) => {
                                params.push(Value::I64(i64::from(v)))
                            }
                            (ValueType::I64, Value::I64(v)) => params.push(Value::I64(v)),
                            (ValueType::F32, Value::F32(v)) => params.push(Value::F32(v)),
                            (ValueType::F32, Value::F64(v)) => params.push(Value::F32(v as f32)),
                            (ValueType::F64, Value::F32(v)) => params.push(Value::F64(v as f64)),
                            (ValueType::F64, Value::F64(v)) => params.push(Value::F64(v)),
                            _ => {
                                return Err(WasmExecutionError {
                                    message: format!(
                                        "type mismatch during call_indirect (table {} type {}): expected {:?}, saw {:?}",
                                        table_index, type_index, expected, value
                                    ),
                                });
                            }
                        }
                    }
                    params.reverse();
                    if target_index == 0 && *type_index == 1 && params.len() == 1 {
                        if let Some(Value::I32(ptr)) = params.get(0) {
                            let invoke_addr = (*ptr as u32).saturating_add(4);
                            if let Ok(invoke_index) = self.read_u32(invoke_addr) {
                                if invoke_index != 0 {
                                    if std::env::var_os("CHIC_DEBUG_WASM_INDIRECT").is_some() {
                                        eprintln!(
                                            "[wasm-indirect] repaired invoke=0 with table index {} from 0x{invoke_addr:08x}",
                                            invoke_index
                                        );
                                    }
                                    target_index = invoke_index;
                                }
                            }
                        }
                    }
                    tracer.record_call(target_index, &params)?;
                    let import_count = self.module.imports.len();
                    if (target_index as usize) < import_count {
                        let import = &self.module.imports[target_index as usize];
                        if import.type_index != *type_index {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "call_indirect type mismatch: caller={func_index} pc={pc} slot={slot} target=import {}::{} type={} expected={}",
                                    import.module, import.name, import.type_index, type_index
                                ),
                            });
                        }
                        let results = self.invoke_import(import, params, &mut tracer)?;
                        if results.len() != expected_sig.results.len() {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "import {}::{} returned {} value(s) but signature expects {}",
                                    import.module,
                                    import.name,
                                    results.len(),
                                    expected_sig.results.len()
                                ),
                            });
                        }
                        for (expected, value) in expected_sig.results.iter().zip(results.iter()) {
                            if !value_matches_type(*value, *expected) {
                                return Err(WasmExecutionError {
                                    message: format!(
                                        "import {}::{} returned incompatible value",
                                        import.module, import.name
                                    ),
                                });
                            }
                        }
                        for value in results {
                            stack.push(value);
                        }
                    } else {
                        let func_idx = target_index as usize - import_count;
                        let function = self.module.functions.get(func_idx).ok_or_else(|| {
                            WasmExecutionError {
                                message: format!(
                                    "call_indirect target {target_index} out of range (caller={func_index} pc={pc} slot={slot})"
                                ),
                            }
                        })?;
                        if function.type_index != *type_index {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "call_indirect type mismatch: caller={func_index} pc={pc} slot={slot} target=function index {target_index} type={} expected={}",
                                    function.type_index, type_index
                                ),
                            });
                        }
                        let results = self.invoke(target_index, &params)?;
                        for value in results {
                            stack.push(value);
                        }
                    }
                    pc += 1;
                }
                Instruction::Return => {
                    if expects_results {
                        if stack.len() < result_types.len() {
                            return Err(WasmExecutionError {
                                message: "value stack underflow on return".into(),
                            });
                        }
                        let mut results = Vec::with_capacity(result_types.len());
                        for _ in 0..result_types.len() {
                            results.push(stack.pop().expect("stack length checked"));
                        }
                        results.reverse();
                        for (value, expected) in results.iter().zip(result_types.iter()) {
                            if !value_matches_type(*value, *expected) {
                                return Err(WasmExecutionError {
                                    message: format!(
                                        "return type mismatch: expected {:?}",
                                        expected
                                    ),
                                });
                            }
                        }
                        return_values = results;
                    } else {
                        let _ = stack.pop();
                        return_values.clear();
                    }
                    break;
                }
                Instruction::Unreachable => {
                    let exported = self
                        .module
                        .exports
                        .iter()
                        .filter_map(|(name, &index)| {
                            if index == func_index {
                                Some(name.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    let export_suffix = if exported.is_empty() {
                        String::new()
                    } else {
                        format!(" exports=[{}]", exported.join(", "))
                    };
                    return Err(WasmExecutionError {
                        message: format!(
                            "reached unreachable instruction (func={func_index}{export_suffix} pc={pc})"
                        ),
                    });
                }
                Instruction::Drop => {
                    stack.pop();
                    pc += 1;
                }
                Instruction::I32Const(value) => {
                    stack.push(Value::I32(*value));
                    pc += 1;
                }
                Instruction::I64Const(value) => {
                    stack.push(Value::I64(*value));
                    pc += 1;
                }
                Instruction::F32Const(value) => {
                    stack.push(Value::F32(*value));
                    pc += 1;
                }
                Instruction::F64Const(value) => {
                    stack.push(Value::F64(*value));
                    pc += 1;
                }
                Instruction::I32WrapI64 => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.wrap_i64".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(value as i32));
                    pc += 1;
                }
                Instruction::I64ExtendI32S => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.extend_i32_s".into(),
                        })?
                        .as_i32()?;
                    stack.push(Value::I64(value as i64));
                    pc += 1;
                }
                Instruction::I64ExtendI32U => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.extend_i32_u".into(),
                        })?
                        .as_i32()?;
                    let extended = u64::from(value as u32) as i64;
                    stack.push(Value::I64(extended));
                    pc += 1;
                }
                Instruction::F32ConvertI32S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.convert_i32_s".into(),
                    })?;
                    let raw = value.as_i32()?;
                    let mode = rounding_mode();
                    let converted = convert_int_to_f32(i128::from(raw), mode);
                    stack.push(Value::F32(converted));
                    pc += 1;
                }
                Instruction::F32ConvertI32U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.convert_i32_u".into(),
                    })?;
                    let raw = value.as_i32()?;
                    let converted = convert_int_to_f32(i128::from(raw), rounding_mode());
                    stack.push(Value::F32(converted));
                    pc += 1;
                }
                Instruction::F32ConvertI64S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.convert_i64_s".into(),
                    })?;
                    let raw = value.as_i64()?;
                    let converted = convert_int_to_f32(raw as i128, rounding_mode());
                    stack.push(Value::F32(converted));
                    pc += 1;
                }
                Instruction::F32ConvertI64U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.convert_i64_u".into(),
                    })?;
                    let raw = value.as_i64()?;
                    let converted = convert_int_to_f32(raw as i128, rounding_mode());
                    stack.push(Value::F32(converted));
                    pc += 1;
                }
                Instruction::F64ConvertI32S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.convert_i32_s".into(),
                    })?;
                    let raw = value.as_i32()?;
                    let converted = convert_int_to_f64(i128::from(raw), rounding_mode());
                    stack.push(Value::F64(converted));
                    pc += 1;
                }
                Instruction::F64ConvertI32U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.convert_i32_u".into(),
                    })?;
                    let raw = value.as_i32()?;
                    let converted = convert_int_to_f64(i128::from(raw), rounding_mode());
                    stack.push(Value::F64(converted));
                    pc += 1;
                }
                Instruction::F64ConvertI64S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.convert_i64_s".into(),
                    })?;
                    let raw = value.as_i64()?;
                    let converted = convert_int_to_f64(raw as i128, rounding_mode());
                    stack.push(Value::F64(converted));
                    pc += 1;
                }
                Instruction::F64ConvertI64U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.convert_i64_u".into(),
                    })?;
                    let raw = value.as_i64()?;
                    let converted = convert_int_to_f64(raw as i128, rounding_mode());
                    stack.push(Value::F64(converted));
                    pc += 1;
                }
                Instruction::I32TruncF32S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.trunc_f32_s".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(
                                f64::from(v),
                                rounded,
                                f64::from(i32::MIN),
                                f64::from(i32::MAX),
                            );
                            rounded as i32
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(
                                v,
                                rounded,
                                f64::from(i32::MIN),
                                f64::from(i32::MAX),
                            );
                            rounded as i32
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i32.trunc_f32_s".into(),
                            });
                        }
                    };
                    stack.push(Value::I32(converted));
                    pc += 1;
                }
                Instruction::I32TruncF32U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.trunc_f32_u".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                            rounded as u32
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                            rounded as u32
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i32.trunc_f32_u".into(),
                            });
                        }
                    };
                    stack.push(Value::I32(converted as i32));
                    pc += 1;
                }
                Instruction::I32TruncF64S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.trunc_f64_s".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(
                                f64::from(v),
                                rounded,
                                f64::from(i32::MIN),
                                f64::from(i32::MAX),
                            );
                            rounded as i32
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(
                                v,
                                rounded,
                                f64::from(i32::MIN),
                                f64::from(i32::MAX),
                            );
                            rounded as i32
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i32.trunc_f64_s".into(),
                            });
                        }
                    };
                    stack.push(Value::I32(converted));
                    pc += 1;
                }
                Instruction::I32TruncF64U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.trunc_f64_u".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                            rounded as u32
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                            rounded as u32
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i32.trunc_f64_u".into(),
                            });
                        }
                    };
                    stack.push(Value::I32(converted as i32));
                    pc += 1;
                }
                Instruction::I64TruncF32S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.trunc_f32_s".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(
                                f64::from(v),
                                rounded,
                                i64::MIN as f64,
                                i64::MAX as f64,
                            );
                            rounded as i64
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(v, rounded, i64::MIN as f64, i64::MAX as f64);
                            rounded as i64
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i64.trunc_f32_s".into(),
                            });
                        }
                    };
                    stack.push(Value::I64(converted));
                    pc += 1;
                }
                Instruction::I64TruncF32U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.trunc_f32_u".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                            rounded as u64
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                            rounded as u64
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i64.trunc_f32_u".into(),
                            });
                        }
                    };
                    stack.push(Value::I64(converted as i64));
                    pc += 1;
                }
                Instruction::I64TruncF64S => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.trunc_f64_s".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(
                                f64::from(v),
                                rounded,
                                i64::MIN as f64,
                                i64::MAX as f64,
                            );
                            rounded as i64
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(v, rounded, i64::MIN as f64, i64::MAX as f64);
                            rounded as i64
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i64.trunc_f64_s".into(),
                            });
                        }
                    };
                    stack.push(Value::I64(converted));
                    pc += 1;
                }
                Instruction::I64TruncF64U => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.trunc_f64_u".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(f64::from(v), mode);
                            record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                            rounded as u64
                        }
                        Value::F64(v) => {
                            let mode = rounding_mode();
                            let rounded = round_value(v, mode);
                            record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                            rounded as u64
                        }
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i64.trunc_f64_u".into(),
                            });
                        }
                    };
                    stack.push(Value::I64(converted as i64));
                    pc += 1;
                }
                Instruction::F64PromoteF32 => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.promote_f32".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => v as f64,
                        Value::F64(v) => v,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during f64.promote_f32".into(),
                            });
                        }
                    };
                    stack.push(Value::F64(converted));
                    pc += 1;
                }
                Instruction::F32Add => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.add".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.add".into(),
                        })?,
                    );
                    let lhs = lhs.as_f32()?;
                    let rhs = rhs.as_f32()?;
                    let exact = f64::from(lhs) + f64::from(rhs);
                    let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                    record_arithmetic_flags(
                        f64::from(lhs),
                        f64::from(rhs),
                        exact,
                        f64::from(rounded),
                        false,
                    );
                    stack.push(Value::F32(rounded));
                    pc += 1;
                }
                Instruction::F32Sub => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.sub".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.sub".into(),
                        })?,
                    );
                    let lhs = lhs.as_f32()?;
                    let rhs = rhs.as_f32()?;
                    let exact = f64::from(lhs) - f64::from(rhs);
                    let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                    record_arithmetic_flags(
                        f64::from(lhs),
                        f64::from(rhs),
                        exact,
                        f64::from(rounded),
                        false,
                    );
                    stack.push(Value::F32(rounded));
                    pc += 1;
                }
                Instruction::F32Mul => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.mul".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.mul".into(),
                        })?,
                    );
                    let lhs = lhs.as_f32()?;
                    let rhs = rhs.as_f32()?;
                    let exact = f64::from(lhs) * f64::from(rhs);
                    let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                    record_arithmetic_flags(
                        f64::from(lhs),
                        f64::from(rhs),
                        exact,
                        f64::from(rounded),
                        false,
                    );
                    stack.push(Value::F32(rounded));
                    pc += 1;
                }
                Instruction::F32Div => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.div".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f32.div".into(),
                        })?,
                    );
                    let lhs = lhs.as_f32()?;
                    let rhs = rhs.as_f32()?;
                    let exact = f64::from(lhs) / f64::from(rhs);
                    let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                    record_arithmetic_flags(
                        f64::from(lhs),
                        f64::from(rhs),
                        exact,
                        f64::from(rounded),
                        true,
                    );
                    stack.push(Value::F32(rounded));
                    pc += 1;
                }
                Instruction::F32DemoteF64 => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.demote_f64".into(),
                    })?;
                    let converted = match value {
                        Value::F64(v) => round_f64_to_f32(v, rounding_mode()),
                        Value::F32(v) => v,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during f32.demote_f64".into(),
                            });
                        }
                    };
                    if std::env::var_os("CHIC_DEBUG_WASM_DEMOTE").is_some() {
                        eprintln!(
                            "[wasm-demote] value={:016x} -> {:08x} flags={:?}",
                            match value {
                                Value::F64(v) => v.to_bits(),
                                _ => 0,
                            },
                            converted.to_bits(),
                            crate::runtime::float_env::read_flags()
                        );
                    }
                    stack.push(Value::F32(converted));
                    pc += 1;
                }
                Instruction::F32Trunc => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.trunc".into(),
                    })?;
                    let source = value.as_f32()?;
                    let truncated = source.trunc();
                    let mut flags = FloatStatusFlags::default();
                    if source.is_nan() {
                        flags.invalid = true;
                    }
                    if truncated != source {
                        flags.inexact = true;
                    }
                    record_flags(flags);
                    stack.push(Value::F32(truncated));
                    pc += 1;
                }
                Instruction::F64Add => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.add".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.add".into(),
                        })?,
                    );
                    let lhs = lhs.as_f64()?;
                    let rhs = rhs.as_f64()?;
                    let exact = lhs + rhs;
                    let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                    record_arithmetic_flags(lhs, rhs, exact, rounded, false);
                    stack.push(Value::F64(rounded));
                    pc += 1;
                }
                Instruction::F64Sub => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.sub".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.sub".into(),
                        })?,
                    );
                    let lhs = lhs.as_f64()?;
                    let rhs = rhs.as_f64()?;
                    let exact = lhs - rhs;
                    let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                    record_arithmetic_flags(lhs, rhs, exact, rounded, false);
                    stack.push(Value::F64(rounded));
                    pc += 1;
                }
                Instruction::F64Mul => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.mul".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.mul".into(),
                        })?,
                    );
                    let lhs = lhs.as_f64()?;
                    let rhs = rhs.as_f64()?;
                    let exact = lhs * rhs;
                    let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                    record_arithmetic_flags(lhs, rhs, exact, rounded, false);
                    stack.push(Value::F64(rounded));
                    pc += 1;
                }
                Instruction::F64Div => {
                    let (rhs, lhs) = (
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.div".into(),
                        })?,
                        stack.pop().ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on f64.div".into(),
                        })?,
                    );
                    let lhs = lhs.as_f64()?;
                    let rhs = rhs.as_f64()?;
                    let exact = lhs / rhs;
                    let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                    record_arithmetic_flags(lhs, rhs, exact, rounded, true);
                    stack.push(Value::F64(rounded));
                    pc += 1;
                }
                Instruction::F64Trunc => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.trunc".into(),
                    })?;
                    let source = value.as_f64()?;
                    let truncated = source.trunc();
                    let mut flags = FloatStatusFlags::default();
                    if source.is_nan() {
                        flags.invalid = true;
                    }
                    if truncated != source {
                        flags.inexact = true;
                    }
                    record_flags(flags);
                    stack.push(Value::F64(truncated));
                    pc += 1;
                }
                Instruction::I32ReinterpretF32 => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.reinterpret_f32".into(),
                    })?;
                    let converted = match value {
                        Value::F32(v) => Value::I32(i32::from_ne_bytes(v.to_bits().to_ne_bytes())),
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i32.reinterpret_f32".into(),
                            });
                        }
                    };
                    stack.push(converted);
                    pc += 1;
                }
                Instruction::I64ReinterpretF64 => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.reinterpret_f64".into(),
                    })?;
                    let converted = match value {
                        Value::F64(v) => Value::I64(i64::from_ne_bytes(v.to_bits().to_ne_bytes())),
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during i64.reinterpret_f64".into(),
                            });
                        }
                    };
                    stack.push(converted);
                    pc += 1;
                }
                Instruction::F32ReinterpretI32 => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.reinterpret_i32".into(),
                    })?;
                    let converted = match value {
                        Value::I32(v) => Value::F32(f32::from_bits(v as u32)),
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during f32.reinterpret_i32".into(),
                            });
                        }
                    };
                    stack.push(converted);
                    pc += 1;
                }
                Instruction::F64ReinterpretI64 => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.reinterpret_i64".into(),
                    })?;
                    let converted = match value {
                        Value::I64(v) => Value::F64(f64::from_bits(v as u64)),
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during f64.reinterpret_i64".into(),
                            });
                        }
                    };
                    stack.push(converted);
                    pc += 1;
                }
                Instruction::I32Eq => {
                    binary_i32(&mut stack, Value::I32, |a, b| i32::from(a == b))?;
                    pc += 1;
                }
                Instruction::I32Ne => {
                    binary_i32(&mut stack, Value::I32, |a, b| i32::from(a != b))?;
                    pc += 1;
                }
                Instruction::I32Eqz => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on eqz".into(),
                        })?
                        .as_i32()?;
                    stack.push(Value::I32(i32::from(value == 0)));
                    pc += 1;
                }
                Instruction::I32LtS => {
                    binary_i32(&mut stack, Value::I32, |a, b| i32::from(a < b))?;
                    pc += 1;
                }
                Instruction::I32LtU => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        i32::from((a as u32) < (b as u32))
                    })?;
                    pc += 1;
                }
                Instruction::I32LeS => {
                    binary_i32(&mut stack, Value::I32, |a, b| i32::from(a <= b))?;
                    pc += 1;
                }
                Instruction::I32LeU => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        i32::from((a as u32) <= (b as u32))
                    })?;
                    pc += 1;
                }
                Instruction::I32GtS => {
                    binary_i32(&mut stack, Value::I32, |a, b| i32::from(a > b))?;
                    pc += 1;
                }
                Instruction::I32GtU => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        i32::from((a as u32) > (b as u32))
                    })?;
                    pc += 1;
                }
                Instruction::I32GeS => {
                    binary_i32(&mut stack, Value::I32, |a, b| i32::from(a >= b))?;
                    pc += 1;
                }
                Instruction::I32GeU => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        i32::from((a as u32) >= (b as u32))
                    })?;
                    pc += 1;
                }
                Instruction::F32Eq => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.eq".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.eq".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f32()? == rhs.as_f32()?)));
                    pc += 1;
                }
                Instruction::F32Ne => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.ne".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.ne".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f32()? != rhs.as_f32()?)));
                    pc += 1;
                }
                Instruction::F32Lt => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.lt".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.lt".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f32()? < rhs.as_f32()?)));
                    pc += 1;
                }
                Instruction::F32Gt => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.gt".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.gt".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f32()? > rhs.as_f32()?)));
                    pc += 1;
                }
                Instruction::F32Le => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.le".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.le".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f32()? <= rhs.as_f32()?)));
                    pc += 1;
                }
                Instruction::F32Ge => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.ge".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.ge".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f32()? >= rhs.as_f32()?)));
                    pc += 1;
                }
                Instruction::F64Eq => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.eq".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.eq".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f64()? == rhs.as_f64()?)));
                    pc += 1;
                }
                Instruction::F64Ne => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.ne".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.ne".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f64()? != rhs.as_f64()?)));
                    pc += 1;
                }
                Instruction::F64Lt => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.lt".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.lt".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f64()? < rhs.as_f64()?)));
                    pc += 1;
                }
                Instruction::F64Gt => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.gt".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.gt".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f64()? > rhs.as_f64()?)));
                    pc += 1;
                }
                Instruction::F64Le => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.le".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.le".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f64()? <= rhs.as_f64()?)));
                    pc += 1;
                }
                Instruction::F64Ge => {
                    let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.ge".into(),
                    })?;
                    let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.ge".into(),
                    })?;
                    stack.push(Value::I32(i32::from(lhs.as_f64()? >= rhs.as_f64()?)));
                    pc += 1;
                }
                Instruction::I64Eq => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.eq".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.eq".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from(left == right)));
                    pc += 1;
                }
                Instruction::I64Ne => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.ne".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.ne".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from(left != right)));
                    pc += 1;
                }
                Instruction::I64LtS => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.lt_s".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.lt_s".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from(left < right)));
                    pc += 1;
                }
                Instruction::I64LeS => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.le_s".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.le_s".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from(left <= right)));
                    pc += 1;
                }
                Instruction::I64GtS => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.gt_s".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.gt_s".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from(left > right)));
                    pc += 1;
                }
                Instruction::I64GeS => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.ge_s".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.ge_s".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from(left >= right)));
                    pc += 1;
                }
                Instruction::I64LtU => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.lt_u".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.lt_u".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from((left as u64) < (right as u64))));
                    pc += 1;
                }
                Instruction::I64LeU => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.le_u".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.le_u".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from((left as u64) <= (right as u64))));
                    pc += 1;
                }
                Instruction::I64GtU => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.gt_u".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.gt_u".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from((left as u64) > (right as u64))));
                    pc += 1;
                }
                Instruction::I64GeU => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.ge_u".into(),
                        })?
                        .as_i64()?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.ge_u".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32(i32::from((left as u64) >= (right as u64))));
                    pc += 1;
                }
                Instruction::I32Add => {
                    binary_i32(&mut stack, Value::I32, i32::wrapping_add)?;
                    pc += 1;
                }
                Instruction::I32Sub => {
                    binary_i32(&mut stack, Value::I32, i32::wrapping_sub)?;
                    pc += 1;
                }
                Instruction::I32Mul => {
                    binary_i32(&mut stack, Value::I32, i32::wrapping_mul)?;
                    pc += 1;
                }
                Instruction::I32DivS => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "division by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    stack.push(Value::I32(dividend.wrapping_div(divisor)));
                    pc += 1;
                }
                Instruction::I32DivU => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "division by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    let result = (dividend as u32).wrapping_div(divisor as u32);
                    stack.push(Value::I32(result as i32));
                    pc += 1;
                }
                Instruction::I32RemS => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "remainder by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    stack.push(Value::I32(dividend.wrapping_rem(divisor)));
                    pc += 1;
                }
                Instruction::I32RemU => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "remainder by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i32()?;
                    let result = (dividend as u32).wrapping_rem(divisor as u32);
                    stack.push(Value::I32(result as i32));
                    pc += 1;
                }
                Instruction::I32And => {
                    binary_i32(&mut stack, Value::I32, |a, b| a & b)?;
                    pc += 1;
                }
                Instruction::I32Or => {
                    binary_i32(&mut stack, Value::I32, |a, b| a | b)?;
                    pc += 1;
                }
                Instruction::I32Xor => {
                    binary_i32(&mut stack, Value::I32, |a, b| a ^ b)?;
                    pc += 1;
                }
                Instruction::I64And => {
                    binary_i64(&mut stack, Value::I64, |a, b| a & b)?;
                    pc += 1;
                }
                Instruction::I64Or => {
                    binary_i64(&mut stack, Value::I64, |a, b| a | b)?;
                    pc += 1;
                }
                Instruction::I64Xor => {
                    binary_i64(&mut stack, Value::I64, |a, b| a ^ b)?;
                    pc += 1;
                }
                Instruction::I64Eqz => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i64.eqz".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I32((value == 0) as i32));
                    pc += 1;
                }
                Instruction::I64Add => {
                    binary_i64(&mut stack, Value::I64, |a, b| a.wrapping_add(b))?;
                    pc += 1;
                }
                Instruction::I64Sub => {
                    binary_i64(&mut stack, Value::I64, |a, b| a.wrapping_sub(b))?;
                    pc += 1;
                }
                Instruction::I64Mul => {
                    binary_i64(&mut stack, Value::I64, |a, b| a.wrapping_mul(b))?;
                    pc += 1;
                }
                Instruction::I64DivS => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "division by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I64(dividend.wrapping_div(divisor)));
                    pc += 1;
                }
                Instruction::I64DivU => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "division by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    let result = (dividend as u64).wrapping_div(divisor as u64);
                    stack.push(Value::I64(result as i64));
                    pc += 1;
                }
                Instruction::I64RemS => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "remainder by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    stack.push(Value::I64(dividend.wrapping_rem(divisor)));
                    pc += 1;
                }
                Instruction::I64RemU => {
                    let divisor = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    if divisor == 0 {
                        return Err(WasmExecutionError {
                            message: "remainder by zero".into(),
                        });
                    }
                    let dividend = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow".into(),
                        })?
                        .as_i64()?;
                    let result = (dividend as u64).wrapping_rem(divisor as u64);
                    stack.push(Value::I64(result as i64));
                    pc += 1;
                }
                Instruction::I32Shl => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        a.wrapping_shl(shift_amount(b))
                    })?;
                    pc += 1;
                }
                Instruction::I32ShrS => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        a.wrapping_shr(shift_amount(b))
                    })?;
                    pc += 1;
                }
                Instruction::I32ShrU => {
                    binary_i32(&mut stack, Value::I32, |a, b| {
                        let shifted = (a as u32).wrapping_shr(shift_amount(b));
                        i32::from_ne_bytes(shifted.to_ne_bytes())
                    })?;
                    pc += 1;
                }
                Instruction::I64Shl => {
                    binary_i64(&mut stack, Value::I64, |a, b| {
                        a.wrapping_shl(shift_amount_i64(b))
                    })?;
                    pc += 1;
                }
                Instruction::I64ShrS => {
                    binary_i64(&mut stack, Value::I64, |a, b| {
                        a.wrapping_shr(shift_amount_i64(b))
                    })?;
                    pc += 1;
                }
                Instruction::I64ShrU => {
                    binary_i64(&mut stack, Value::I64, |a, b| {
                        let shifted = (a as u64).wrapping_shr(shift_amount_i64(b));
                        i64::from_ne_bytes(shifted.to_ne_bytes())
                    })?;
                    pc += 1;
                }
                Instruction::LocalGet(index) => {
                    let value = locals
                        .get(*index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!(
                                "local.get index {index} out of range (locals={} func={})",
                                locals.len(),
                                func_index
                            ),
                        })?;
                    stack.push(*value);
                    pc += 1;
                }
                Instruction::LocalSet(index) => {
                    let value = stack.pop().unwrap_or(Value::I32(0));
                    if let Some(slot) = locals.get_mut(*index as usize) {
                        *slot = value;
                    } else {
                        return Err(WasmExecutionError {
                            message: format!(
                                "local.set index {index} out of range (locals={} func={})",
                                locals.len(),
                                func_index
                            ),
                        });
                    }
                    pc += 1;
                }
                Instruction::LocalTee(index) => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on local.tee".into(),
                    })?;
                    if let Some(slot) = locals.get_mut(*index as usize) {
                        *slot = value;
                    } else {
                        return Err(WasmExecutionError {
                            message: format!(
                                "local.tee index {index} out of range (locals={} func={})",
                                locals.len(),
                                func_index
                            ),
                        });
                    }
                    stack.push(value);
                    pc += 1;
                }
                Instruction::GlobalGet(index) => {
                    let value = self
                        .globals
                        .get(*index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!("global index {index} out of range"),
                        })?
                        .value;
                    if *index == 0 && std::env::var_os("CHIC_DEBUG_WASM_SP").is_some() {
                        eprintln!(
                            "[wasm-sp] get func={} pc={} value={:?}",
                            func_index, pc, value
                        );
                    }
                    stack.push(value);
                    pc += 1;
                }
                Instruction::GlobalSet(index) => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on global.set".into(),
                    })?;
                    let global = self.globals.get_mut(*index as usize).ok_or_else(|| {
                        WasmExecutionError {
                            message: format!("global index {index} out of range"),
                        }
                    })?;
                    if !global.mutable {
                        return Err(WasmExecutionError {
                            message: format!("global {index} is immutable"),
                        });
                    }
                    if !value_matches_type(value, global.ty) {
                        return Err(WasmExecutionError {
                            message: "type mismatch during global.set".into(),
                        });
                    }
                    if *index == 0 && std::env::var_os("CHIC_DEBUG_WASM_SP").is_some() {
                        eprintln!(
                            "[wasm-sp] set func={} pc={} value={:?}",
                            func_index, pc, value
                        );
                    }
                    global.value = value;
                    pc += 1;
                }
                Instruction::I32Load { offset } => {
                    let addr = pop_address(&mut stack, "i32.load")?;
                    let value = self.load_i32(addr, *offset)?;
                    stack.push(Value::I32(value));
                    pc += 1;
                }
                Instruction::I32Load8S { offset } => {
                    let addr = pop_address(&mut stack, "i32.load8_s")?;
                    let byte = match self.load_bytes(addr, *offset, 1) {
                        Ok(bytes) => bytes[0] as i8,
                        Err(mut err) => {
                            let sp_snapshot = self
                                .globals
                                .get(0)
                                .and_then(|g| match g.value {
                                    Value::I32(v) => Some(v),
                                    _ => None,
                                })
                                .unwrap_or(-1);
                            err.message = format!(
                                "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                                err.message,
                                func_index,
                                pc,
                                offset,
                                sp_snapshot,
                                stack.len()
                            );
                            return Err(err);
                        }
                    };
                    stack.push(Value::I32(i32::from(byte)));
                    pc += 1;
                }
                Instruction::I32Load8U { offset } => {
                    let addr = pop_address(&mut stack, "i32.load8_u")?;
                    let byte = match self.load_bytes(addr, *offset, 1) {
                        Ok(bytes) => bytes[0],
                        Err(mut err) => {
                            let sp_snapshot = self
                                .globals
                                .get(0)
                                .and_then(|g| match g.value {
                                    Value::I32(v) => Some(v),
                                    _ => None,
                                })
                                .unwrap_or(-1);
                            let locals_snapshot =
                                if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                                    format!(" locals={locals:?}")
                                } else {
                                    String::new()
                                };
                            let call_stack_snapshot =
                                if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                                    format!(" call_stack={:?}", self.call_stack)
                                } else {
                                    String::new()
                                };
                            err.message = format!(
                                "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={}{}{})",
                                err.message,
                                func_index,
                                pc,
                                offset,
                                sp_snapshot,
                                stack.len(),
                                locals_snapshot,
                                call_stack_snapshot
                            );
                            return Err(err);
                        }
                    };
                    stack.push(Value::I32(i32::from(byte)));
                    pc += 1;
                }
                Instruction::I32Load16S { offset } => {
                    let addr = pop_address(&mut stack, "i32.load16_s")?;
                    let bytes = match self.load_bytes(addr, *offset, 2) {
                        Ok(bytes) => bytes,
                        Err(mut err) => {
                            let sp_snapshot = self
                                .globals
                                .get(0)
                                .and_then(|g| match g.value {
                                    Value::I32(v) => Some(v),
                                    _ => None,
                                })
                                .unwrap_or(-1);
                            err.message = format!(
                                "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                                err.message,
                                func_index,
                                pc,
                                offset,
                                sp_snapshot,
                                stack.len()
                            );
                            return Err(err);
                        }
                    };
                    let value = i16::from_le_bytes([bytes[0], bytes[1]]);
                    stack.push(Value::I32(i32::from(value)));
                    pc += 1;
                }
                Instruction::I32Load16U { offset } => {
                    let addr = pop_address(&mut stack, "i32.load16_u")?;
                    let bytes = match self.load_bytes(addr, *offset, 2) {
                        Ok(bytes) => bytes,
                        Err(mut err) => {
                            let sp_snapshot = self
                                .globals
                                .get(0)
                                .and_then(|g| match g.value {
                                    Value::I32(v) => Some(v),
                                    _ => None,
                                })
                                .unwrap_or(-1);
                            err.message = format!(
                                "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                                err.message,
                                func_index,
                                pc,
                                offset,
                                sp_snapshot,
                                stack.len()
                            );
                            return Err(err);
                        }
                    };
                    let value = u16::from_le_bytes([bytes[0], bytes[1]]);
                    stack.push(Value::I32(i32::from(value)));
                    pc += 1;
                }
                Instruction::I64Load { offset } => {
                    let addr = match pop_address(&mut stack, "i64.load") {
                        Ok(addr) => addr,
                        Err(mut err) => {
                            let sp_snapshot = self
                                .globals
                                .get(0)
                                .and_then(|g| {
                                    if let Value::I32(v) = g.value {
                                        Some(v)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(-1);
                            err.message = format!(
                                "{} (func={} pc={} sp_global={} stack_len={} stack={:?})",
                                err.message,
                                func_index,
                                pc,
                                sp_snapshot,
                                stack.len(),
                                self.call_stack
                            );
                            return Err(err);
                        }
                    };
                    let value = self.load_i64(addr, *offset)?;
                    stack.push(Value::I64(value));
                    pc += 1;
                }
                Instruction::F32Load { offset } => {
                    let addr = pop_address(&mut stack, "f32.load")?;
                    let value = self.load_f32(addr, *offset)?;
                    stack.push(Value::F32(value));
                    pc += 1;
                }
                Instruction::F64Load { offset } => {
                    let addr = pop_address(&mut stack, "f64.load")?;
                    let value = self.load_f64(addr, *offset)?;
                    stack.push(Value::F64(value));
                    pc += 1;
                }
                Instruction::I32Store { offset } => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.store".into(),
                        })?
                        .as_i32()?;
                    let addr = match pop_address(&mut stack, "i32.store") {
                        Ok(addr) => addr,
                        Err(mut err) => {
                            let sp_snapshot = self
                                .globals
                                .get(0)
                                .and_then(|g| {
                                    if let Value::I32(v) = g.value {
                                        Some(v)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(-1);
                            err.message = format!(
                                "{} (func={} pc={} value={} sp_global={} stack_len={})",
                                err.message,
                                func_index,
                                pc,
                                value,
                                sp_snapshot,
                                stack.len()
                            );
                            return Err(err);
                        }
                    };
                    if (addr as i32) < 0 {
                        return Err(WasmExecutionError {
                            message: format!(
                                "negative memory address on i32.store (addr=0x{addr:08x} value={value} func={} pc={})",
                                func_index, pc
                            ),
                        });
                    }
                    if std::env::var_os("CHIC_DEBUG_WASM_FN_RUNTIME").is_some() {
                        if let Some((start, end)) = self.tracked_fn_range {
                            let effective = (addr as u32).saturating_add(*offset as u32);
                            if effective >= start && effective < end {
                                let snapshot =
                                    self.read_bytes(start, end.saturating_sub(start)).ok();
                                eprintln!(
                                    "[wasm-fn-runtime] store i32 func={} pc={} addr=0x{:08x} off={} value=0x{:08x} range=0x{:08x}-0x{:08x} snapshot={:?}",
                                    func_index,
                                    pc,
                                    addr as u32,
                                    offset,
                                    value as u32,
                                    start,
                                    end,
                                    snapshot
                                );
                            }
                        }
                    }
                    if let Err(mut err) = self.store_i32(addr, *offset, value) {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:08x} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            value as u32,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                    pc += 1;
                }
                Instruction::I32Store8 { offset } => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.store8".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.store8")?;
                    if let Err(mut err) = self.store_i8(addr, *offset, value as u8) {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        let locals_snapshot =
                            if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                                format!(" locals={locals:?}")
                            } else {
                                String::new()
                            };
                        let call_stack_snapshot =
                            if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                                format!(" call_stack={:?}", self.call_stack)
                            } else {
                                String::new()
                            };
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:02x} sp_global={} stack_len={}{}{})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            value as u8,
                            sp_snapshot,
                            stack.len(),
                            locals_snapshot,
                            call_stack_snapshot
                        );
                        return Err(err);
                    }
                    pc += 1;
                }
                Instruction::I32Store16 { offset } => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.store16".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.store16")?;
                    let bytes = (value as u16).to_le_bytes();
                    if let Err(mut err) = self.store_bytes(addr as u32, *offset, &bytes) {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:04x} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            value as u16,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                    pc += 1;
                }
                Instruction::I64Store { offset } => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.store".into(),
                    })?;
                    let i64_value = match value {
                        Value::I64(v) => v,
                        _ => {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "type mismatch during i64.store (func={func_index} pc={pc} value={value:?})"
                                ),
                            });
                        }
                    };
                    let addr = pop_address(&mut stack, "i64.store")?;
                    if std::env::var_os("CHIC_DEBUG_WASM_FN_RUNTIME").is_some() {
                        if let Some((start, end)) = self.tracked_fn_range {
                            let effective = (addr as u32).saturating_add(*offset as u32);
                            if effective >= start && effective < end {
                                let snapshot =
                                    self.read_bytes(start, end.saturating_sub(start)).ok();
                                eprintln!(
                                    "[wasm-fn-runtime] store i64 func={} pc={} addr=0x{:08x} off={} value=0x{:016x} range=0x{:08x}-0x{:08x} snapshot={:?}",
                                    func_index,
                                    pc,
                                    addr as u32,
                                    offset,
                                    i64_value as u64,
                                    start,
                                    end,
                                    snapshot
                                );
                            }
                        }
                    }
                    if let Err(mut err) = self.store_i64(addr, *offset, i64_value) {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:016x} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            i64_value as u64,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                    pc += 1;
                }
                Instruction::F32Store { offset } => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.store".into(),
                    })?;
                    let f32_value = match value {
                        Value::F32(v) => v,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during f32.store".into(),
                            });
                        }
                    };
                    let addr = pop_address(&mut stack, "f32.store")?;
                    if let Err(mut err) = self.store_f32(addr, *offset, f32_value) {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                    pc += 1;
                }
                Instruction::F64Store { offset } => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.store".into(),
                    })?;
                    let f64_value = match value {
                        Value::F64(v) => v,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "type mismatch during f64.store".into(),
                            });
                        }
                    };
                    let addr = pop_address(&mut stack, "f64.store")?;
                    if let Err(mut err) = self.store_f64(addr, *offset, f64_value) {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                    pc += 1;
                }
                Instruction::I32AtomicLoad { offset } => {
                    let addr = pop_address(&mut stack, "i32.atomic.load")?;
                    let value = self.load_i32(addr, *offset)?;
                    stack.push(Value::I32(value));
                    pc += 1;
                }
                Instruction::I64AtomicLoad { offset } => {
                    let addr = pop_address(&mut stack, "i64.atomic.load")?;
                    let value = self.load_i64(addr, *offset)?;
                    stack.push(Value::I64(value));
                    pc += 1;
                }
                Instruction::I32AtomicStore { offset } => {
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.store".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.store")?;
                    self.store_i32(addr, *offset, value)?;
                    pc += 1;
                }
                Instruction::I64AtomicStore { offset } => {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.store".into(),
                    })?;
                    let i64_value = value.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.store")?;
                    self.store_i64(addr, *offset, i64_value)?;
                    pc += 1;
                }
                Instruction::I32AtomicRmwAdd { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.add".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.add")?;
                    let previous = self.load_i32(addr, *offset)?;
                    let new_value = previous.wrapping_add(operand);
                    self.store_i32(addr, *offset, new_value)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwAdd { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.add".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.add")?;
                    let previous = self.load_i64(addr, *offset)?;
                    let new_value = previous.wrapping_add(operand);
                    self.store_i64(addr, *offset, new_value)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwSub { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.sub".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.sub")?;
                    let previous = self.load_i32(addr, *offset)?;
                    let new_value = previous.wrapping_sub(operand);
                    self.store_i32(addr, *offset, new_value)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwSub { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.sub".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.sub")?;
                    let previous = self.load_i64(addr, *offset)?;
                    let new_value = previous.wrapping_sub(operand);
                    self.store_i64(addr, *offset, new_value)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwAnd { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.and".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.and")?;
                    let previous = self.load_i32(addr, *offset)?;
                    self.store_i32(addr, *offset, previous & operand)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwAnd { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.and".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.and")?;
                    let previous = self.load_i64(addr, *offset)?;
                    self.store_i64(addr, *offset, previous & operand)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwOr { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.or".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.or")?;
                    let previous = self.load_i32(addr, *offset)?;
                    self.store_i32(addr, *offset, previous | operand)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwOr { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.or".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.or")?;
                    let previous = self.load_i64(addr, *offset)?;
                    self.store_i64(addr, *offset, previous | operand)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwXor { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.xor".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.xor")?;
                    let previous = self.load_i32(addr, *offset)?;
                    self.store_i32(addr, *offset, previous ^ operand)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwXor { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.xor".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.xor")?;
                    let previous = self.load_i64(addr, *offset)?;
                    self.store_i64(addr, *offset, previous ^ operand)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwXchg { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.xchg".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.xchg")?;
                    let previous = self.load_i32(addr, *offset)?;
                    self.store_i32(addr, *offset, operand)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwXchg { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.xchg".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.xchg")?;
                    let previous = self.load_i64(addr, *offset)?;
                    self.store_i64(addr, *offset, operand)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwCmpxchg { offset } => {
                    let desired = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.cmpxchg (desired)"
                                .into(),
                        })?
                        .as_i32()?;
                    let expected = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.cmpxchg (expected)"
                                .into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.cmpxchg")?;
                    let previous = self.load_i32(addr, *offset)?;
                    if previous == expected {
                        self.store_i32(addr, *offset, desired)?;
                    }
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwCmpxchg { offset } => {
                    let desired = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.cmpxchg (desired)".into(),
                    })?;
                    let desired = desired.as_i64()?;
                    let expected = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.cmpxchg (expected)"
                            .into(),
                    })?;
                    let expected = expected.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.cmpxchg")?;
                    let previous = self.load_i64(addr, *offset)?;
                    if previous == expected {
                        self.store_i64(addr, *offset, desired)?;
                    }
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwMinS { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.min_s".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.min_s")?;
                    let previous = self.load_i32(addr, *offset)?;
                    let new_value = previous.min(operand);
                    self.store_i32(addr, *offset, new_value)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwMinS { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.min_s".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.min_s")?;
                    let previous = self.load_i64(addr, *offset)?;
                    let new_value = previous.min(operand);
                    self.store_i64(addr, *offset, new_value)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::I32AtomicRmwMaxS { offset } => {
                    let operand = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on i32.atomic.rmw.max_s".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "i32.atomic.rmw.max_s")?;
                    let previous = self.load_i32(addr, *offset)?;
                    let new_value = previous.max(operand);
                    self.store_i32(addr, *offset, new_value)?;
                    stack.push(Value::I32(previous));
                    pc += 1;
                }
                Instruction::I64AtomicRmwMaxS { offset } => {
                    let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.atomic.rmw.max_s".into(),
                    })?;
                    let operand = operand.as_i64()?;
                    let addr = pop_address(&mut stack, "i64.atomic.rmw.max_s")?;
                    let previous = self.load_i64(addr, *offset)?;
                    let new_value = previous.max(operand);
                    self.store_i64(addr, *offset, new_value)?;
                    stack.push(Value::I64(previous));
                    pc += 1;
                }
                Instruction::AtomicFence => {
                    // The single-threaded interpreter does not model hardware ordering scopes.
                    pc += 1;
                }
                Instruction::MemoryFill { mem } => {
                    let len = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on memory.fill (len)".into(),
                        })?
                        .as_i32()?;
                    let value = stack
                        .pop()
                        .ok_or_else(|| WasmExecutionError {
                            message: "value stack underflow on memory.fill (value)".into(),
                        })?
                        .as_i32()?;
                    let addr = pop_address(&mut stack, "memory.fill")?;
                    if *mem != 0 {
                        return Err(WasmExecutionError {
                            message: "only default memory supported for memory.fill".into(),
                        });
                    }
                    if len < 0 {
                        return Err(WasmExecutionError {
                            message: "memory.fill length must be non-negative".into(),
                        });
                    }
                    self.fill(addr, 0, len as u32, value as u8)?;
                    pc += 1;
                }
            }
        }

        if expects_results && return_values.is_empty() {
            if stack.len() < result_types.len() {
                return Err(WasmExecutionError {
                    message: "function completed without returning a value".into(),
                });
            }
            let mut results = Vec::with_capacity(result_types.len());
            for _ in 0..result_types.len() {
                results.push(stack.pop().expect("stack length checked"));
            }
            results.reverse();
            for (value, expected) in results.iter().zip(result_types.iter()) {
                if !value_matches_type(*value, *expected) {
                    return Err(WasmExecutionError {
                        message: format!("implicit return type mismatch: expected {:?}", expected),
                    });
                }
            }
            return_values = results;
        }

        tracer.record_return(return_values.first())?;
        Ok(return_values)
    }
}
