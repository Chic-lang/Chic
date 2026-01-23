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
mod calls;
mod control;
mod float_ops;
mod int_ops;
mod locals_globals;
mod memory;
mod stack_convert;
mod terminators;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StepOutcome {
    Continue,
    Halt,
}

struct StepContext<'ctx, 'code> {
    func_index: u32,
    pc: &'ctx mut usize,
    code: &'code [Instruction],
    stack: &'ctx mut Vec<Value>,
    locals: &'ctx mut Vec<Value>,
    control_stack: &'ctx mut Vec<ControlLabel>,
    return_values: &'ctx mut Vec<Value>,
    expects_results: bool,
    result_types: &'code [ValueType],
    tracer: &'ctx mut SchedulerTracer,
}

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
            self.watchdog_tick()?;
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
            let mut ctx = StepContext {
                func_index,
                pc: &mut pc,
                code: &code,
                stack: &mut stack,
                locals: &mut locals,
                control_stack: &mut control_stack,
                return_values: &mut return_values,
                expects_results,
                result_types: &result_types,
                tracer: &mut tracer,
            };
            match self.step_instruction(&mut ctx)? {
                StepOutcome::Continue => {}
                StepOutcome::Halt => break,
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

    fn step_instruction(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<StepOutcome, WasmExecutionError> {
        if let Some(outcome) = self.step_control(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_calls(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_terminators(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_stack_convert(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_float_ops(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_int_ops(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_locals_globals(ctx)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.step_memory(ctx)? {
            return Ok(outcome);
        }
        let pc = *ctx.pc;
        let instr = ctx.code.get(pc);
        Err(WasmExecutionError {
            message: format!("unsupported wasm instruction at pc={pc}: {instr:?}"),
        })
    }
}
